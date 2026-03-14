//! COSMIC mutation-frequency provider.
//!
//! Supports two real data paths:
//! - Local COSMIC export file (`FERRUMYX_COSMIC_MUTATION_DATA_PATH`) for fast offline lookup.
//! - HTTP API endpoint (`FERRUMYX_COSMIC_BASE_URL` + `FERRUMYX_COSMIC_MUTATION_FREQUENCY_PATH`)
//!   with optional key (`FERRUMYX_COSMIC_API_KEY`).
//!
//! The remote API response schema is normalized leniently because deployments vary.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tracing::{debug, warn};

use super::LiteratureSource;
use crate::models::PaperMetadata;

const DEFAULT_BASE_URL: &str = "https://cancer.sanger.ac.uk/cosmic/api";
const DEFAULT_TIMEOUT_SECS: u64 = 10;
const DEFAULT_MUTATION_PATH: &str = "/mutations/frequency";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmicMutationFrequency {
    pub gene_symbol: String,
    pub cancer_code: String,
    pub mutated_sample_count: u32,
    pub profiled_sample_count: u32,
    pub mutation_frequency: f64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationRecord {
    pub gene_symbol: String,
    pub transcript_id: Option<String>,
    pub mutation: String,
    pub mutation_type: MutationType,
    pub cancer_type: String,
    pub tissue_type: Option<String>,
    pub sample_count: usize,
    pub frequency: Option<f64>,
    pub is_drug_resistance: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MutationType {
    Missense,
    Nonsense,
    Frameshift,
    InFrameDeletion,
    InFrameInsertion,
    SpliceSite,
    Synonymous,
    Unknown,
}

impl MutationType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "missense" | "substitution - missense" => MutationType::Missense,
            "nonsense" | "substitution - nonsense" => MutationType::Nonsense,
            "frameshift" | "deletion - frameshift" | "insertion - frameshift" => {
                MutationType::Frameshift
            }
            "inframe deletion" | "deletion - in frame" => MutationType::InFrameDeletion,
            "inframe insertion" | "insertion - in frame" => MutationType::InFrameInsertion,
            "splice site" | "complex" => MutationType::SpliceSite,
            "synonymous" | "substitution - coding silent" => MutationType::Synonymous,
            _ => MutationType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct CosmicDatasetIndex {
    by_gene_cancer: HashMap<String, CosmicMutationFrequency>,
    by_gene_any: HashMap<String, CosmicMutationFrequency>,
}

#[derive(Debug, Clone, Default)]
struct FrequencyAgg {
    mutated_sum: u64,
    profiled_sum: u64,
    frequency_sum: f64,
    frequency_n: u64,
}

impl FrequencyAgg {
    fn add(&mut self, mutated: Option<u32>, profiled: Option<u32>, frequency: Option<f64>) {
        if let Some(v) = mutated {
            self.mutated_sum += v as u64;
        }
        if let Some(v) = profiled {
            self.profiled_sum += v as u64;
        }
        if let Some(v) = frequency {
            self.frequency_sum += normalize_frequency(v);
            self.frequency_n += 1;
        }
    }

    fn to_frequency(
        &self,
        gene: &str,
        cancer: &str,
        source: &str,
    ) -> Option<CosmicMutationFrequency> {
        if self.profiled_sum == 0 && self.frequency_n == 0 {
            return None;
        }
        let frequency = if self.profiled_sum > 0 {
            (self.mutated_sum as f64 / self.profiled_sum as f64).clamp(0.0, 1.0)
        } else {
            (self.frequency_sum / self.frequency_n as f64).clamp(0.0, 1.0)
        };
        Some(CosmicMutationFrequency {
            gene_symbol: gene.to_string(),
            cancer_code: cancer.to_string(),
            mutated_sample_count: self.mutated_sum.min(u32::MAX as u64) as u32,
            profiled_sample_count: self.profiled_sum.min(u32::MAX as u64) as u32,
            mutation_frequency: frequency,
            source: source.to_string(),
        })
    }
}

#[derive(Debug, Clone, Default)]
struct ApiFrequencyCandidate {
    gene_symbol: Option<String>,
    cancer_code: Option<String>,
    mutated_sample_count: Option<u32>,
    profiled_sample_count: Option<u32>,
    mutation_frequency: Option<f64>,
    source: Option<String>,
}

pub struct CosmicClient {
    client: Client,
    base_url: String,
    mutation_frequency_path: String,
    api_key: Option<String>,
    dataset_path: Option<String>,
}

impl CosmicClient {
    pub fn new() -> Self {
        let base_url = std::env::var("FERRUMYX_COSMIC_BASE_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let mutation_frequency_path = std::env::var("FERRUMYX_COSMIC_MUTATION_FREQUENCY_PATH")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_MUTATION_PATH.to_string());
        let timeout_secs = std::env::var("FERRUMYX_COSMIC_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .clamp(3, 60);
        let api_key = std::env::var("FERRUMYX_COSMIC_API_KEY")
            .ok()
            .filter(|v| !v.trim().is_empty());
        let dataset_path = std::env::var("FERRUMYX_COSMIC_MUTATION_DATA_PATH")
            .ok()
            .filter(|v| !v.trim().is_empty());
        Self::with_config(
            base_url,
            mutation_frequency_path,
            api_key,
            dataset_path,
            timeout_secs,
        )
    }

    pub fn with_config(
        base_url: String,
        mutation_frequency_path: String,
        api_key: Option<String>,
        dataset_path: Option<String>,
        timeout_secs: u64,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs.clamp(3, 60)))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            base_url: base_url.trim().trim_end_matches('/').to_string(),
            mutation_frequency_path: normalize_path(&mutation_frequency_path),
            api_key,
            dataset_path,
        }
    }

    pub async fn get_mutation_frequency(
        &self,
        gene_symbol: &str,
        cancer_code: &str,
    ) -> anyhow::Result<Option<CosmicMutationFrequency>> {
        let gene = normalize_gene(gene_symbol);
        let cancer = normalize_cancer(cancer_code);
        if gene.is_empty() || cancer.is_empty() {
            return Ok(None);
        }

        if let Some(v) = self.lookup_dataset(&gene, Some(&cancer)).await {
            return Ok(Some(v));
        }

        if let Some(v) = self.fetch_frequency_from_api(&gene, Some(&cancer)).await {
            return Ok(Some(v));
        }

        Ok(None)
    }

    pub async fn get_mutation_frequency_any_cancer(
        &self,
        gene_symbol: &str,
    ) -> anyhow::Result<Option<CosmicMutationFrequency>> {
        let gene = normalize_gene(gene_symbol);
        if gene.is_empty() {
            return Ok(None);
        }

        if let Some(v) = self.lookup_dataset(&gene, None).await {
            return Ok(Some(v));
        }

        if let Some(v) = self.fetch_frequency_from_api(&gene, None).await {
            return Ok(Some(v));
        }

        Ok(None)
    }

    async fn lookup_dataset(
        &self,
        gene: &str,
        cancer: Option<&str>,
    ) -> Option<CosmicMutationFrequency> {
        let path = self.dataset_path.as_deref()?;
        let index = load_dataset_index(path).await?;

        if let Some(cancer_code) = cancer {
            for alias in cancer_aliases(cancer_code) {
                let key = format!("{}|{}", gene, alias);
                if let Some(v) = index.by_gene_cancer.get(&key) {
                    return Some(v.clone());
                }
            }
        }

        index.by_gene_any.get(gene).cloned()
    }

    async fn fetch_frequency_from_api(
        &self,
        gene: &str,
        cancer: Option<&str>,
    ) -> Option<CosmicMutationFrequency> {
        let mut req = self
            .client
            .get(format!("{}{}", self.base_url, self.mutation_frequency_path))
            .header("Accept", "application/json")
            .query(&[
                ("gene", gene.to_string()),
                ("gene_symbol", gene.to_string()),
                ("symbol", gene.to_string()),
            ]);
        if let Some(c) = cancer {
            req = req.query(&[
                ("cancer_code", c.to_string()),
                ("cancer", c.to_string()),
                ("cancer_type", c.to_string()),
            ]);
        }
        if let Some(key) = self.api_key.as_ref() {
            req = req
                .header("Authorization", format!("Bearer {key}"))
                .header("X-API-Key", key);
        }

        let resp = req.send().await.ok()?;
        if !resp.status().is_success() {
            debug!(
                status = %resp.status(),
                "COSMIC mutation frequency request returned non-success"
            );
            return None;
        }
        let text = resp.text().await.ok()?;
        if text.trim_start().starts_with('<') {
            // Most often login HTML page when endpoint/session is not authorized.
            return None;
        }
        let json = serde_json::from_str::<serde_json::Value>(&text).ok()?;
        let mut candidates = Vec::new();
        collect_api_candidates(&json, &mut candidates);
        if candidates.is_empty() {
            return None;
        }
        pick_best_candidate(gene, cancer, &candidates)
    }
}

impl Default for CosmicClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LiteratureSource for CosmicClient {
    async fn search(
        &self,
        _query: &str,
        _max_results: usize,
    ) -> anyhow::Result<Vec<PaperMetadata>> {
        Ok(Vec::new())
    }

    async fn fetch_full_text(&self, _paper_id: &str) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
}

fn normalize_path(path: &str) -> String {
    if path.trim().is_empty() {
        return DEFAULT_MUTATION_PATH.to_string();
    }
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn normalize_gene(gene_symbol: &str) -> String {
    gene_symbol.trim().to_uppercase()
}

fn normalize_cancer(cancer_code: &str) -> String {
    cancer_code.trim().to_uppercase()
}

fn normalize_frequency(v: f64) -> f64 {
    if (1.0..=100.0).contains(&v) {
        (v / 100.0).clamp(0.0, 1.0)
    } else {
        v.clamp(0.0, 1.0)
    }
}

fn parse_u32_like(s: &str) -> Option<u32> {
    let t = s.trim().replace(',', "");
    if t.is_empty() {
        return None;
    }
    if let Ok(v) = t.parse::<u32>() {
        return Some(v);
    }
    t.parse::<f64>().ok().and_then(|v| {
        if v.is_finite() && v >= 0.0 {
            Some(v.round().min(u32::MAX as f64) as u32)
        } else {
            None
        }
    })
}

fn parse_f64_like(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', "");
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok().map(normalize_frequency)
}

fn normalized_header_key(header: &str) -> String {
    header
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
}

fn find_header_index(headers: &[String], candidates: &[&str]) -> Option<usize> {
    for needle in candidates {
        let norm = normalized_header_key(needle);
        if let Some((idx, _)) = headers
            .iter()
            .enumerate()
            .find(|(_, h)| normalized_header_key(h) == norm)
        {
            return Some(idx);
        }
    }
    None
}

fn split_row(row: &str, delimiter: char) -> Vec<String> {
    row.split(delimiter)
        .map(|v| v.trim().trim_matches('"').to_string())
        .collect()
}

fn cancer_aliases(cancer: &str) -> Vec<String> {
    let code = normalize_cancer(cancer);
    let mut out = vec![code.clone()];
    match code.as_str() {
        "PAAD" => out.extend(
            ["PANCREAS", "PANCREATIC", "PANCREATICADENOCARCINOMA"]
                .iter()
                .map(|s| s.to_string()),
        ),
        "LUAD" => out.extend(
            ["LUNG", "LUNGADENOCARCINOMA", "NSCLC"]
                .iter()
                .map(|s| s.to_string()),
        ),
        "LUSC" => out.extend(["LUNGSQUAMOUS", "LUNG"].iter().map(|s| s.to_string())),
        "BRCA" => out.extend(["BREAST", "BREASTCANCER"].iter().map(|s| s.to_string())),
        "COAD" | "READ" => out.extend(["COLON", "COLORECTAL", "CRC"].iter().map(|s| s.to_string())),
        "SKCM" => out.extend(["MELANOMA", "SKIN"].iter().map(|s| s.to_string())),
        _ => {}
    }
    out
}

async fn load_dataset_index(path: &str) -> Option<CosmicDatasetIndex> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<CosmicDatasetIndex>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(found) = guard.get(path) {
            return found.clone();
        }
    }

    let text = tokio::fs::read_to_string(path).await.ok();
    let parsed = text.and_then(|content| parse_dataset_index(&content));
    if parsed.is_none() {
        warn!(path = %path, "COSMIC dataset path configured but failed to parse mutation file");
    }

    if let Ok(mut guard) = cache.lock() {
        guard.insert(path.to_string(), parsed.clone());
    }
    parsed
}

fn parse_dataset_index(content: &str) -> Option<CosmicDatasetIndex> {
    let mut lines = content.lines();
    let header_line = lines.next()?.trim();
    if header_line.is_empty() {
        return None;
    }
    let delimiter = if header_line.matches('\t').count() >= header_line.matches(',').count() {
        '\t'
    } else {
        ','
    };
    let headers = split_row(header_line, delimiter);

    let gene_idx = find_header_index(
        &headers,
        &["gene_symbol", "gene", "symbol", "hgnc_symbol", "gene_name"],
    )?;
    let cancer_idx = find_header_index(
        &headers,
        &[
            "cancer_code",
            "cancer_type",
            "primary_site",
            "tumour_site",
            "tumor_site",
            "disease",
        ],
    )?;
    let freq_idx = find_header_index(
        &headers,
        &[
            "mutation_frequency",
            "frequency",
            "mut_freq",
            "mutant_frequency",
        ],
    );
    let mutated_idx = find_header_index(
        &headers,
        &[
            "mutated_sample_count",
            "mutated_count",
            "mutation_count",
            "samples_mutated",
        ],
    );
    let profiled_idx = find_header_index(
        &headers,
        &[
            "profiled_sample_count",
            "sample_count",
            "samples_tested",
            "total_samples",
        ],
    );

    let mut per_cancer: HashMap<String, FrequencyAgg> = HashMap::new();
    let mut per_gene: HashMap<String, FrequencyAgg> = HashMap::new();

    for line in lines {
        let row = line.trim();
        if row.is_empty() {
            continue;
        }
        let cols = split_row(row, delimiter);
        if cols.len() <= gene_idx || cols.len() <= cancer_idx {
            continue;
        }
        let gene = normalize_gene(cols[gene_idx].as_str());
        let cancer = normalize_cancer(cols[cancer_idx].as_str());
        if gene.is_empty() || cancer.is_empty() {
            continue;
        }
        let mutated = mutated_idx
            .and_then(|i| cols.get(i))
            .and_then(|v| parse_u32_like(v));
        let profiled = profiled_idx
            .and_then(|i| cols.get(i))
            .and_then(|v| parse_u32_like(v));
        let frequency = freq_idx
            .and_then(|i| cols.get(i))
            .and_then(|v| parse_f64_like(v));

        per_cancer
            .entry(format!("{gene}|{cancer}"))
            .or_default()
            .add(mutated, profiled, frequency);
        per_gene
            .entry(gene.clone())
            .or_default()
            .add(mutated, profiled, frequency);
    }

    let mut index = CosmicDatasetIndex::default();
    for (k, agg) in per_cancer {
        let mut parts = k.split('|');
        let gene = parts.next().unwrap_or_default();
        let cancer = parts.next().unwrap_or_default();
        if let Some(row) = agg.to_frequency(gene, cancer, "cosmic_dataset") {
            index.by_gene_cancer.insert(k, row);
        }
    }
    for (gene, agg) in per_gene {
        if let Some(row) = agg.to_frequency(&gene, "PAN_CANCER", "cosmic_dataset") {
            index.by_gene_any.insert(gene, row);
        }
    }
    Some(index)
}

fn parse_u32_value(v: &serde_json::Value) -> Option<u32> {
    v.as_u64()
        .map(|x| x.min(u32::MAX as u64) as u32)
        .or_else(|| {
            v.as_i64()
                .and_then(|x| if x >= 0 { Some(x as u32) } else { None })
        })
        .or_else(|| {
            v.as_f64()
                .and_then(|x| if x >= 0.0 { Some(x as u32) } else { None })
        })
        .or_else(|| v.as_str().and_then(parse_u32_like))
}

fn parse_f64_value(v: &serde_json::Value) -> Option<f64> {
    v.as_f64()
        .map(normalize_frequency)
        .or_else(|| v.as_u64().map(|x| normalize_frequency(x as f64)))
        .or_else(|| v.as_str().and_then(parse_f64_like))
}

fn collect_api_candidates(value: &serde_json::Value, out: &mut Vec<ApiFrequencyCandidate>) {
    match value {
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_api_candidates(item, out);
            }
        }
        serde_json::Value::Object(obj) => {
            let pick_s = |keys: &[&str]| -> Option<String> {
                keys.iter()
                    .find_map(|k| obj.get(*k))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            };
            let pick_u = |keys: &[&str]| -> Option<u32> {
                keys.iter()
                    .find_map(|k| obj.get(*k))
                    .and_then(parse_u32_value)
            };
            let pick_f = |keys: &[&str]| -> Option<f64> {
                keys.iter()
                    .find_map(|k| obj.get(*k))
                    .and_then(parse_f64_value)
            };
            let candidate = ApiFrequencyCandidate {
                gene_symbol: pick_s(&["gene_symbol", "gene", "symbol", "hgnc_symbol"]),
                cancer_code: pick_s(&["cancer_code", "cancer", "cancer_type", "primary_site"]),
                mutated_sample_count: pick_u(&[
                    "mutated_sample_count",
                    "mutated_count",
                    "mutation_count",
                    "samples_mutated",
                ]),
                profiled_sample_count: pick_u(&[
                    "profiled_sample_count",
                    "sample_count",
                    "samples_tested",
                    "total_samples",
                ]),
                mutation_frequency: pick_f(&[
                    "mutation_frequency",
                    "frequency",
                    "mut_freq",
                    "mutant_frequency",
                ]),
                source: pick_s(&["source", "provider"]),
            };
            if candidate.gene_symbol.is_some()
                || candidate.mutation_frequency.is_some()
                || candidate.mutated_sample_count.is_some()
            {
                out.push(candidate);
            }

            for nested_key in ["data", "results", "items", "payload"] {
                if let Some(nested) = obj.get(nested_key) {
                    collect_api_candidates(nested, out);
                }
            }
        }
        _ => {}
    }
}

fn pick_best_candidate(
    gene: &str,
    cancer: Option<&str>,
    candidates: &[ApiFrequencyCandidate],
) -> Option<CosmicMutationFrequency> {
    let expected_cancers = cancer.map(cancer_aliases).unwrap_or_default();
    let mut best: Option<CosmicMutationFrequency> = None;

    for cand in candidates {
        let cand_gene = cand
            .gene_symbol
            .as_deref()
            .map(normalize_gene)
            .unwrap_or_else(|| gene.to_string());
        if !cand_gene.is_empty() && cand_gene != gene {
            continue;
        }

        let cand_cancer = cand
            .cancer_code
            .as_deref()
            .map(normalize_cancer)
            .unwrap_or_else(|| "PAN_CANCER".to_string());
        if !expected_cancers.is_empty() && !expected_cancers.iter().any(|v| v == &cand_cancer) {
            continue;
        }

        let profiled = cand.profiled_sample_count.unwrap_or(0);
        let mutated = cand.mutated_sample_count.unwrap_or(0);
        let freq = if let Some(f) = cand.mutation_frequency {
            normalize_frequency(f)
        } else if profiled > 0 {
            (mutated as f64 / profiled as f64).clamp(0.0, 1.0)
        } else {
            continue;
        };

        let row = CosmicMutationFrequency {
            gene_symbol: gene.to_string(),
            cancer_code: cand_cancer,
            mutated_sample_count: mutated,
            profiled_sample_count: profiled,
            mutation_frequency: freq,
            source: cand
                .source
                .clone()
                .unwrap_or_else(|| "cosmic_api".to_string()),
        };
        best = Some(match best.take() {
            Some(cur) => {
                if row.profiled_sample_count > cur.profiled_sample_count
                    || (row.profiled_sample_count == cur.profiled_sample_count
                        && row.mutation_frequency > cur.mutation_frequency)
                {
                    row
                } else {
                    cur
                }
            }
            None => row,
        });
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dataset_index_basic_tsv() {
        let data = "gene_symbol\tcancer_code\tmutated_sample_count\tprofiled_sample_count\nKRAS\tPAAD\t30\t100\nKRAS\tPAAD\t20\t50\nEGFR\tLUAD\t10\t200\n";
        let idx = parse_dataset_index(data).unwrap();
        let row = idx.by_gene_cancer.get("KRAS|PAAD").unwrap();
        assert_eq!(row.mutated_sample_count, 50);
        assert_eq!(row.profiled_sample_count, 150);
        assert!((row.mutation_frequency - (50.0 / 150.0)).abs() < 1e-6);
    }

    #[test]
    fn picks_best_api_candidate() {
        let payload = serde_json::json!({
            "data": [
                {"gene_symbol":"KRAS","cancer_code":"PAAD","mutation_frequency":0.2,"profiled_sample_count":100},
                {"gene_symbol":"KRAS","cancer_code":"PAAD","mutation_frequency":0.3,"profiled_sample_count":120}
            ]
        });
        let mut candidates = Vec::new();
        collect_api_candidates(&payload, &mut candidates);
        let best = pick_best_candidate("KRAS", Some("PAAD"), &candidates).unwrap();
        assert!((best.mutation_frequency - 0.3).abs() < 1e-6);
        assert_eq!(best.profiled_sample_count, 120);
    }
}
