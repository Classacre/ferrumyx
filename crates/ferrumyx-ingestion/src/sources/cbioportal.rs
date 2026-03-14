//! cBioPortal API client for source-backed mutation frequency signals.
//!
//! This client fetches per-gene mutation prevalence for a cancer cohort using:
//! - Study selection by OncoTree/cancer code
//! - Mutation molecular profile discovery
//! - Mutation sample list discovery
//! - Gene symbol -> Entrez ID resolution
//! - Mutation event fetch and unique mutated sample counting

use reqwest::{Client, Method};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "https://www.cbioportal.org/api";
const DEFAULT_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CbioMutationFrequency {
    pub gene_symbol: String,
    pub cancer_code: String,
    pub study_id: String,
    pub molecular_profile_id: String,
    pub sample_list_id: String,
    pub mutated_sample_count: u32,
    pub profiled_sample_count: u32,
    pub mutation_frequency: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CbioStudy {
    study_id: String,
    cancer_type_id: Option<String>,
    public_study: Option<bool>,
    sequenced_sample_count: Option<u32>,
    all_sample_count: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CbioMolecularProfile {
    molecular_profile_id: String,
    molecular_alteration_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CbioSampleList {
    sample_list_id: String,
    category: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CbioGene {
    entrez_gene_id: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CbioMutationEvent {
    sample_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MutationFetchBody {
    sample_list_id: String,
    entrez_gene_ids: Vec<i32>,
}

#[derive(Debug, Clone)]
struct StudySelection {
    study_id: String,
    mutation_profile_id: String,
    mutation_sample_list_id: String,
}

pub struct CbioPortalClient {
    client: Client,
    base_url: String,
    api_token: Option<String>,
}

impl CbioPortalClient {
    pub fn new() -> Self {
        let base = std::env::var("FERRUMYX_CBIOPORTAL_BASE_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let timeout_secs = std::env::var("FERRUMYX_CBIOPORTAL_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .clamp(3, 60);
        let api_token = std::env::var("FERRUMYX_CBIOPORTAL_API_TOKEN")
            .ok()
            .filter(|v| !v.trim().is_empty());
        Self::with_config(base, api_token, timeout_secs)
    }

    pub fn with_config(base_url: String, api_token: Option<String>, timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs.clamp(3, 60)))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            base_url: normalize_base_url(&base_url),
            api_token,
        }
    }

    pub async fn get_mutation_frequency(
        &self,
        gene_symbol: &str,
        cancer_code: &str,
    ) -> anyhow::Result<Option<CbioMutationFrequency>> {
        let gene = gene_symbol.trim().to_uppercase();
        let Some(cancer) = normalize_cancer_code(cancer_code) else {
            return Ok(None);
        };
        if gene.is_empty() {
            return Ok(None);
        }

        let Some(selection) = self.resolve_study_selection(&cancer).await? else {
            return Ok(None);
        };
        let Some(entrez_gene_id) = self.resolve_entrez_gene_id(&gene).await? else {
            return Ok(None);
        };
        let profiled_sample_count = self
            .resolve_profiled_sample_count(&selection.mutation_sample_list_id)
            .await?;
        if profiled_sample_count == 0 {
            return Ok(None);
        }

        let mutated_sample_count = self
            .fetch_mutated_sample_count(
                &selection.mutation_profile_id,
                &selection.mutation_sample_list_id,
                entrez_gene_id,
            )
            .await?;
        let mutation_frequency =
            (mutated_sample_count as f64 / profiled_sample_count as f64).clamp(0.0, 1.0);

        Ok(Some(CbioMutationFrequency {
            gene_symbol: gene,
            cancer_code: cancer,
            study_id: selection.study_id,
            molecular_profile_id: selection.mutation_profile_id,
            sample_list_id: selection.mutation_sample_list_id,
            mutated_sample_count,
            profiled_sample_count,
            mutation_frequency,
        }))
    }

    async fn resolve_study_selection(
        &self,
        cancer_code: &str,
    ) -> anyhow::Result<Option<StudySelection>> {
        static CACHE: OnceLock<Mutex<HashMap<String, Option<StudySelection>>>> = OnceLock::new();
        let key = cancer_code.trim().to_uppercase();
        if key.is_empty() {
            return Ok(None);
        }
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(guard) = cache.lock() {
            if let Some(found) = guard.get(&key) {
                return Ok(found.clone());
            }
        }

        let code_lc = key.to_lowercase();
        let mut candidates = vec![
            format!("{code_lc}_tcga_pan_can_atlas_2018"),
            format!("{code_lc}_tcga"),
            format!("{code_lc}_tcga_pub"),
            format!("{code_lc}_tcga_firehose_legacy"),
        ];

        for candidate in resolve_study_hints(cancer_code) {
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }

        for study_id in &candidates {
            if let Some(study) = self.fetch_study_by_id(study_id).await? {
                if !study.public_study.unwrap_or(true) {
                    continue;
                }
                if let Some(selection) = self.select_profile_and_sample_list(study_id).await? {
                    if let Ok(mut guard) = cache.lock() {
                        guard.insert(key.clone(), Some(selection.clone()));
                    }
                    return Ok(Some(selection));
                }
            }
        }

        let mut studies = self
            .get_json::<Vec<CbioStudy>>(
                Method::GET,
                "/studies",
                Some(&[
                    ("projection", "SUMMARY".to_string()),
                    ("pageSize", "5000".to_string()),
                    ("pageNumber", "0".to_string()),
                ]),
                None,
            )
            .await
            .unwrap_or_default();

        studies.retain(|s| {
            s.public_study.unwrap_or(true)
                && s.cancer_type_id
                    .as_ref()
                    .is_some_and(|v| v.eq_ignore_ascii_case(&code_lc))
        });
        studies.sort_by(|a, b| {
            let a_seq = a.sequenced_sample_count.unwrap_or(0);
            let b_seq = b.sequenced_sample_count.unwrap_or(0);
            b_seq.cmp(&a_seq).then_with(|| {
                b.all_sample_count
                    .unwrap_or(0)
                    .cmp(&a.all_sample_count.unwrap_or(0))
            })
        });

        for study in studies {
            if let Some(selection) = self.select_profile_and_sample_list(&study.study_id).await? {
                if let Ok(mut guard) = cache.lock() {
                    guard.insert(key.clone(), Some(selection.clone()));
                }
                return Ok(Some(selection));
            }
        }

        if let Ok(mut guard) = cache.lock() {
            guard.insert(key, None);
        }
        Ok(None)
    }

    async fn resolve_entrez_gene_id(&self, gene_symbol: &str) -> anyhow::Result<Option<i32>> {
        static CACHE: OnceLock<Mutex<HashMap<String, Option<i32>>>> = OnceLock::new();
        let key = gene_symbol.trim().to_uppercase();
        if key.is_empty() {
            return Ok(None);
        }

        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(guard) = cache.lock() {
            if let Some(v) = guard.get(&key) {
                return Ok(*v);
            }
        }

        let payload = serde_json::Value::Array(vec![serde_json::Value::String(key.clone())]);
        let genes = self
            .get_json::<Vec<CbioGene>>(
                Method::POST,
                "/genes/fetch",
                Some(&[("geneIdType", "HUGO_GENE_SYMBOL".to_string())]),
                Some(payload),
            )
            .await
            .unwrap_or_default();

        let resolved = genes.iter().find_map(|g| g.entrez_gene_id);
        if let Ok(mut guard) = cache.lock() {
            guard.insert(key, resolved);
        }
        Ok(resolved)
    }

    async fn resolve_profiled_sample_count(&self, sample_list_id: &str) -> anyhow::Result<u32> {
        static CACHE: OnceLock<Mutex<HashMap<String, Option<u32>>>> = OnceLock::new();
        let key = sample_list_id.trim().to_string();
        if key.is_empty() {
            return Ok(0);
        }

        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(guard) = cache.lock() {
            if let Some(v) = guard.get(&key) {
                return Ok(v.unwrap_or(0));
            }
        }

        let path = format!("/sample-lists/{}/sample-ids", key);
        let sample_ids = self
            .get_json::<Vec<String>>(Method::GET, &path, None, None)
            .await
            .unwrap_or_default();
        let count = sample_ids.len() as u32;
        if let Ok(mut guard) = cache.lock() {
            guard.insert(key, Some(count));
        }
        Ok(count)
    }

    async fn fetch_mutated_sample_count(
        &self,
        mutation_profile_id: &str,
        sample_list_id: &str,
        entrez_gene_id: i32,
    ) -> anyhow::Result<u32> {
        let path = format!(
            "/molecular-profiles/{}/mutations/fetch",
            mutation_profile_id
        );
        let body = MutationFetchBody {
            sample_list_id: sample_list_id.to_string(),
            entrez_gene_ids: vec![entrez_gene_id],
        };
        let events = self
            .get_json::<Vec<CbioMutationEvent>>(
                Method::POST,
                &path,
                Some(&[
                    ("projection", "SUMMARY".to_string()),
                    ("pageSize", "1000000".to_string()),
                ]),
                Some(serde_json::to_value(body)?),
            )
            .await
            .unwrap_or_default();

        let mut unique_samples = HashSet::new();
        for event in events {
            if !event.sample_id.trim().is_empty() {
                unique_samples.insert(event.sample_id);
            }
        }
        Ok(unique_samples.len() as u32)
    }

    async fn select_profile_and_sample_list(
        &self,
        study_id: &str,
    ) -> anyhow::Result<Option<StudySelection>> {
        let profiles_path = format!("/studies/{}/molecular-profiles", study_id);
        let profiles = self
            .get_json::<Vec<CbioMolecularProfile>>(Method::GET, &profiles_path, None, None)
            .await
            .unwrap_or_default();
        let Some(profile_id) = profiles
            .iter()
            .find(|p| {
                p.molecular_alteration_type
                    .as_deref()
                    .is_some_and(|t| t.eq_ignore_ascii_case("MUTATION_EXTENDED"))
            })
            .map(|p| p.molecular_profile_id.clone())
        else {
            return Ok(None);
        };

        let sample_lists_path = format!("/studies/{}/sample-lists", study_id);
        let sample_lists = self
            .get_json::<Vec<CbioSampleList>>(Method::GET, &sample_lists_path, None, None)
            .await
            .unwrap_or_default();
        let sample_list_id = choose_mutation_sample_list(&sample_lists)?;

        Ok(Some(StudySelection {
            study_id: study_id.to_string(),
            mutation_profile_id: profile_id,
            mutation_sample_list_id: sample_list_id,
        }))
    }

    async fn fetch_study_by_id(&self, study_id: &str) -> anyhow::Result<Option<CbioStudy>> {
        let path = format!("/studies/{}", study_id);
        let resp = self.request(Method::GET, &path).send().await.ok();
        let Some(resp) = resp else {
            return Ok(None);
        };
        if !resp.status().is_success() {
            return Ok(None);
        }
        let study = resp.json::<CbioStudy>().await.ok();
        Ok(study)
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: Option<&[(&str, String)]>,
        body: Option<serde_json::Value>,
    ) -> anyhow::Result<T> {
        let mut req = self.request(method, path);
        if let Some(params) = query {
            req = req.query(params);
        }
        if let Some(value) = body {
            req = req.json(&value);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("cbioportal request failed: {} {}", path, resp.status());
        }
        Ok(resp.json::<T>().await?)
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let mut req = self
            .client
            .request(method, url)
            .header("Accept", "application/json");
        if let Some(token) = self.api_token.as_ref() {
            req = req.bearer_auth(token);
        }
        req
    }
}

impl Default for CbioPortalClient {
    fn default() -> Self {
        Self::new()
    }
}

fn choose_mutation_sample_list(sample_lists: &[CbioSampleList]) -> anyhow::Result<String> {
    let pick = |pred: &dyn Fn(&CbioSampleList) -> bool| -> Option<String> {
        sample_lists
            .iter()
            .find(|s| pred(s))
            .map(|s| s.sample_list_id.clone())
    };
    let by_category = |needle: &str| -> Option<String> {
        pick(&|s| {
            s.category
                .as_deref()
                .is_some_and(|c| c.eq_ignore_ascii_case(needle))
        })
    };
    let mut out = by_category("all_cases_with_mutation_data");
    if out.is_none() {
        out = by_category("all_cases_with_mutation_and_cna_data");
    }
    if out.is_none() {
        out = pick(&|s| s.sample_list_id.to_lowercase().ends_with("_sequenced"));
    }
    if out.is_none() {
        out = pick(&|s| {
            let id = s.sample_list_id.to_lowercase();
            id.contains("mutation") || id.contains("seq")
        });
    }
    if out.is_none() {
        out = by_category("all_cases_in_study");
    }
    out.ok_or_else(|| anyhow::anyhow!("no compatible mutation sample list found"))
}

fn normalize_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return DEFAULT_BASE_URL.to_string();
    }
    trimmed.trim_end_matches('/').to_string()
}

fn normalize_cancer_code(cancer_code: &str) -> Option<String> {
    let mut code = cancer_code.trim().to_uppercase();
    if let Some(stripped) = code.strip_prefix("TCGA-") {
        code = stripped.to_string();
    }
    code = match code.as_str() {
        "NSCLC" => "LUAD".to_string(),
        other => other.to_string(),
    };
    if code.len() < 3 || code.len() > 12 {
        return None;
    }
    if !code
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return None;
    }
    Some(code)
}

fn resolve_study_hints(cancer_code: &str) -> Vec<String> {
    let key = cancer_code.trim().to_uppercase();
    let mapped = match key.as_str() {
        "NSCLC" => "LUAD",
        other => other,
    };
    let mut out = Vec::new();
    let var_name = format!("FERRUMYX_CBIOPORTAL_STUDY_HINT_{}", mapped);
    if let Ok(value) = std::env::var(var_name) {
        for part in value.split(',') {
            let s = part.trim().to_string();
            if !s.is_empty() && !out.contains(&s) {
                out.push(s);
            }
        }
    }
    out
}
