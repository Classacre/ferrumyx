//! ferrumyx-ranker — Target prioritization scoring engine.
//! Implements Phase 4 of ARCHITECTURE.md.

pub mod depmap_provider;
pub mod gtex_provider;
pub mod normalise;
pub mod providers;
pub mod scorer;
pub mod tcga_provider;
pub mod weights;

use ferrumyx_common::query::{QueryRequest, QueryResult, TargetMetrics};
use ferrumyx_db::entities::EntityRepository;
use ferrumyx_db::kg_conflicts::KgConflictRepository;
use ferrumyx_db::kg_facts::KgFactRepository;
use ferrumyx_db::papers::{PaperNoveltySignal, PaperRepository};
use ferrumyx_db::schema::{
    EntDruggability, EntStructure, Entity as DbEntity, EntityType as DbEntityType, KgFact,
};
use ferrumyx_db::Database;
use ferrumyx_db::{
    EntCbioMutationFrequency, EntChemblTarget, EntCosmicMutationFrequency, EntGtexExpression,
    EntProviderRefreshRun, EntReactomeGene, EntStageRepository, EntTcgaSurvival,
    Phase4SignalRepository,
};
use ferrumyx_ingestion::sources::CbioPortalClient;
use ferrumyx_ingestion::sources::ChemblClient;
use ferrumyx_ingestion::sources::CosmicClient;
use ferrumyx_ingestion::sources::DepMapCache;
use ferrumyx_ingestion::sources::GtexClient;
use ferrumyx_ingestion::sources::TcgaClient;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tracing::{info, warn};

const PROVIDER_SIGNAL_TTL_DAYS: i64 = 14;
const IN_PROCESS_PROVIDER_CACHE_TTL_SECS: u64 = 60 * 60;
const IN_PROCESS_PROVIDER_CACHE_MAX_ENTRIES: usize = 4096;

pub struct TargetQueryEngine {
    db: Arc<Database>,
}

#[derive(Debug, Clone)]
struct TimedCacheEntry<V> {
    value: V,
    inserted_at: Instant,
}

#[derive(Debug)]
struct InProcessTtlCache<V> {
    ttl: Duration,
    max_entries: usize,
    entries: Mutex<HashMap<String, TimedCacheEntry<V>>>,
}

impl<V> InProcessTtlCache<V> {
    fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            ttl,
            max_entries: max_entries.max(1),
            entries: Mutex::new(HashMap::new()),
        }
    }
}

impl<V: Clone> InProcessTtlCache<V> {
    fn get_cloned(&self, key: &str) -> Option<V> {
        let now = Instant::now();
        let Ok(mut guard) = self.entries.lock() else {
            return None;
        };
        cache_get_cloned(&mut guard, key, now, self.ttl)
    }

    fn insert(&self, key: String, value: V) {
        let Ok(mut guard) = self.entries.lock() else {
            return;
        };
        cache_insert(
            &mut guard,
            key,
            value,
            Instant::now(),
            self.ttl,
            self.max_entries,
        );
    }
}

fn prune_cache_entries<V>(
    cache: &mut HashMap<String, TimedCacheEntry<V>>,
    now: Instant,
    ttl: Duration,
) {
    cache.retain(|_, entry| now.saturating_duration_since(entry.inserted_at) < ttl);
}

fn cache_get_cloned<V: Clone>(
    cache: &mut HashMap<String, TimedCacheEntry<V>>,
    key: &str,
    now: Instant,
    ttl: Duration,
) -> Option<V> {
    prune_cache_entries(cache, now, ttl);
    cache.get(key).map(|entry| entry.value.clone())
}

fn cache_insert<V>(
    cache: &mut HashMap<String, TimedCacheEntry<V>>,
    key: String,
    value: V,
    now: Instant,
    ttl: Duration,
    max_entries: usize,
) {
    prune_cache_entries(cache, now, ttl);
    cache.insert(
        key,
        TimedCacheEntry {
            value,
            inserted_at: now,
        },
    );
    if cache.len() <= max_entries {
        return;
    }

    let remove_n = cache.len() - max_entries;
    let mut oldest_keys: Vec<(Instant, String)> = cache
        .iter()
        .map(|(key, entry)| (entry.inserted_at, key.clone()))
        .collect();
    oldest_keys.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    for (_, key) in oldest_keys.into_iter().take(remove_n) {
        cache.remove(&key);
    }
}

fn provider_process_cache_ttl() -> Duration {
    Duration::from_secs(IN_PROCESS_PROVIDER_CACHE_TTL_SECS)
}

fn provider_f64_cache() -> InProcessTtlCache<Option<f64>> {
    InProcessTtlCache::new(
        provider_process_cache_ttl(),
        IN_PROCESS_PROVIDER_CACHE_MAX_ENTRIES,
    )
}

fn provider_u32_cache() -> InProcessTtlCache<Option<u32>> {
    InProcessTtlCache::new(
        provider_process_cache_ttl(),
        IN_PROCESS_PROVIDER_CACHE_MAX_ENTRIES,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FactProvenance {
    Provider,
    Extracted,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfidenceTier {
    High,
    Medium,
    Low,
}

fn classify_fact_provenance(f: &KgFact) -> FactProvenance {
    let evidence_type = f.evidence_type.trim().to_ascii_lowercase();
    let predicate = f.predicate.trim().to_ascii_lowercase();
    let evidence = f.evidence.as_deref().unwrap_or("").to_ascii_lowercase();
    let provider_predicate = predicate.ends_with("_cbioportal")
        || predicate.ends_with("_cosmic")
        || predicate.ends_with("_tcga")
        || predicate.ends_with("_gtex")
        || predicate.ends_with("_chembl")
        || predicate.ends_with("_reactome");
    if evidence_type.contains("provider") || provider_predicate || evidence.contains("provider=") {
        FactProvenance::Provider
    } else if evidence_type.contains("mention")
        || evidence_type.contains("generic")
        || predicate == "mentions"
        || predicate == "associated_with"
    {
        FactProvenance::Generic
    } else {
        FactProvenance::Extracted
    }
}

fn classify_confidence_tier(f: &KgFact, provenance: FactProvenance) -> ConfidenceTier {
    let confidence = (f.confidence as f64).clamp(0.0, 1.0);
    match provenance {
        FactProvenance::Provider => {
            if confidence >= 0.62 {
                ConfidenceTier::High
            } else {
                ConfidenceTier::Medium
            }
        }
        FactProvenance::Generic => {
            if confidence >= 0.80 && !f.predicate.eq_ignore_ascii_case("mentions") {
                ConfidenceTier::Medium
            } else {
                ConfidenceTier::Low
            }
        }
        FactProvenance::Extracted => {
            if confidence >= 0.78 {
                ConfidenceTier::High
            } else if confidence >= 0.52 {
                ConfidenceTier::Medium
            } else {
                ConfidenceTier::Low
            }
        }
    }
}

fn fact_signal_weight(provenance: FactProvenance, tier: ConfidenceTier, predicate: &str) -> f64 {
    let provenance_weight: f64 = match provenance {
        FactProvenance::Provider => 1.28_f64,
        FactProvenance::Extracted => 1.0_f64,
        FactProvenance::Generic => 0.68_f64,
    };
    let tier_weight: f64 = match tier {
        ConfidenceTier::High => 1.18_f64,
        ConfidenceTier::Medium => 1.0_f64,
        ConfidenceTier::Low => 0.74_f64,
    };
    let predicate_lc = predicate.trim().to_ascii_lowercase();
    let predicate_weight: f64 = if predicate_lc == "mentions" {
        0.52_f64
    } else if predicate_lc == "associated_with" {
        0.74_f64
    } else {
        1.0_f64
    };
    (provenance_weight * tier_weight * predicate_weight).clamp(0.18_f64, 2.2_f64)
}

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.len() < needle.len() {
        return false;
    }

    let needle_len = needle.len();
    for (start, _) in haystack.char_indices() {
        let end = start + needle_len;
        if end > haystack.len() {
            break;
        }
        if let Some(segment) = haystack.get(start..end) {
            if segment.eq_ignore_ascii_case(needle) {
                return true;
            }
        }
    }

    haystack
        .get(haystack.len().saturating_sub(needle_len)..)
        .is_some_and(|segment| segment.eq_ignore_ascii_case(needle))
}

fn fields_contain_any_ascii_keyword(fields: &[&str], keywords: &[&str]) -> bool {
    fields.iter().any(|field| {
        keywords
            .iter()
            .any(|keyword| contains_ascii_case_insensitive(field, keyword))
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRefreshRequest {
    pub genes: Vec<String>,
    pub cancer_code: Option<String>,
    pub max_genes: usize,
    pub batch_size: usize,
    pub retries: u8,
    #[serde(default)]
    pub offline_strict: bool,
}

impl Default for ProviderRefreshRequest {
    fn default() -> Self {
        Self {
            genes: Vec::new(),
            cancer_code: None,
            max_genes: 24,
            batch_size: 6,
            retries: 1,
            offline_strict: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderRefreshReport {
    pub genes_requested: usize,
    pub genes_processed: usize,
    pub cbio_attempted: usize,
    pub cbio_success: usize,
    pub cbio_failed: usize,
    pub cbio_skipped: usize,
    pub cosmic_attempted: usize,
    pub cosmic_success: usize,
    pub cosmic_failed: usize,
    pub cosmic_skipped: usize,
    pub gtex_attempted: usize,
    pub gtex_success: usize,
    pub gtex_failed: usize,
    pub gtex_skipped: usize,
    pub tcga_attempted: usize,
    pub tcga_success: usize,
    pub tcga_failed: usize,
    pub tcga_skipped: usize,
    pub chembl_attempted: usize,
    pub chembl_success: usize,
    pub chembl_failed: usize,
    pub chembl_skipped: usize,
    pub reactome_attempted: usize,
    pub reactome_success: usize,
    pub reactome_failed: usize,
    pub reactome_skipped: usize,
    pub provider_decisions: BTreeMap<String, String>,
    pub provider_timing_ms: BTreeMap<String, u64>,
    pub kg_backfill_inserted: usize,
    pub kg_backfill_deleted: usize,
    pub kg_backfill_failed: usize,
    pub duration_ms: u64,
}

impl TargetQueryEngine {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn execute_query(&self, req: QueryRequest) -> anyhow::Result<Vec<QueryResult>> {
        let t0 = Instant::now();
        let kg_repo = KgFactRepository::new(self.db.clone());
        let conflict_repo = KgConflictRepository::new(self.db.clone());

        // Fetch a bounded cohort of facts, then rank unique gene candidates.
        let fact_limit = (req.max_results.saturating_mul(250)).clamp(200, 5000);
        let facts = kg_repo.list(0, fact_limit).await.unwrap_or_default();
        let t_facts = Instant::now();
        let gene_filter = req.gene_symbol.as_deref().unwrap_or("").to_lowercase();
        let cancer_filter = req.cancer_code.as_deref().unwrap_or("").to_lowercase();
        let fact_ids: Vec<_> = facts.iter().map(|f| f.id).collect();
        let conflicts = conflict_repo
            .find_by_fact_ids(&fact_ids)
            .await
            .unwrap_or_default();
        let t_conflicts = Instant::now();
        let mut conflicts_by_fact: HashMap<uuid::Uuid, Vec<_>> = HashMap::new();
        for c in conflicts {
            conflicts_by_fact
                .entry(c.fact_a_id)
                .or_default()
                .push(c.clone());
            conflicts_by_fact.entry(c.fact_b_id).or_default().push(c);
        }

        let mut candidates: HashMap<uuid::Uuid, GeneCandidate> = HashMap::with_capacity(fact_limit);

        for f in facts {
            if f.predicate.eq_ignore_ascii_case("mentions") || !is_gene_like(&f.subject_name) {
                continue;
            }

            let subject_name_lc = (!gene_filter.is_empty() || !cancer_filter.is_empty())
                .then(|| f.subject_name.to_lowercase());
            if !gene_filter.is_empty()
                && !subject_name_lc
                    .as_deref()
                    .is_some_and(|subject| subject.contains(&gene_filter))
            {
                continue;
            }

            let object_name_lc = (!cancer_filter.is_empty()).then(|| f.object_name.to_lowercase());
            let predicate_lc_for_filter =
                (!cancer_filter.is_empty()).then(|| f.predicate.to_lowercase());
            if !cancer_filter.is_empty() {
                let has_cancer_context = subject_name_lc
                    .as_deref()
                    .is_some_and(|subject| subject.contains(&cancer_filter))
                    || object_name_lc
                        .as_deref()
                        .is_some_and(|object| object.contains(&cancer_filter))
                    || predicate_lc_for_filter
                        .as_deref()
                        .is_some_and(|predicate| predicate.contains(&cancer_filter));
                if !has_cancer_context {
                    continue;
                }
            }

            let conflicts = conflicts_by_fact.remove(&f.id).unwrap_or_default();
            let mut include = true;
            let mut confidence_adj = f.confidence as f64;
            let mut disputed = false;

            for conflict in conflicts {
                let net = conflict.net_confidence as f64;
                if net < 0.30 {
                    include = false;
                    break;
                } else if net <= 0.60 {
                    disputed = true;
                    confidence_adj *= 0.70;
                }
            }

            if !include {
                continue;
            }

            let entry = candidates
                .entry(f.subject_id)
                .or_insert_with(|| GeneCandidate::new(f.subject_name.clone()));
            let provenance = classify_fact_provenance(&f);
            let tier = classify_confidence_tier(&f, provenance);
            let signal_weight = fact_signal_weight(provenance, tier, &f.predicate);
            let weighted_confidence =
                (confidence_adj.clamp(0.0, 1.0) * signal_weight).clamp(0.0, 1.0);

            if disputed {
                entry.flags.insert("DISPUTED".to_string());
            }

            entry.fact_count += 1;
            entry.weighted_evidence_sum += signal_weight;
            entry.confidence_sum += confidence_adj.clamp(0.0, 1.0);
            entry.confidence_n += 1;
            entry.weighted_confidence_sum += weighted_confidence;
            entry.weighted_confidence_weight += signal_weight;
            if matches!(provenance, FactProvenance::Provider) {
                entry.provider_fact_count += 1;
            }
            if matches!(tier, ConfidenceTier::High) {
                entry.high_tier_fact_count += 1;
            }
            entry.paper_ids.insert(f.paper_id);

            let predicate_lc =
                predicate_lc_for_filter.unwrap_or_else(|| f.predicate.to_lowercase());
            if predicate_lc.contains("mutation") || predicate_lc == "has_mutation" {
                entry.mutation_mentions += 1;
                entry.mutation_mentions_w += signal_weight;
            }
            if predicate_lc.contains("survival")
                || predicate_lc.contains("hazard")
                || predicate_lc.contains("mortality")
                || predicate_lc.contains("prognosis")
                || predicate_lc.contains("overall_survival")
                || predicate_lc.contains("progression_free")
            {
                entry.survival_mentions += 1;
                entry.survival_mentions_w += signal_weight;
                let survival_fields = [
                    f.subject_name.as_str(),
                    f.object_name.as_str(),
                    f.evidence.as_deref().unwrap_or(""),
                ];
                if fields_contain_any_ascii_keyword(
                    &survival_fields,
                    &["poor", "worse", "decreased", "shorter", "reduced"],
                ) {
                    entry.survival_negative += 1;
                    entry.survival_negative_w += signal_weight;
                } else if fields_contain_any_ascii_keyword(
                    &survival_fields,
                    &["improved", "better", "longer", "favorable", "increased"],
                ) {
                    entry.survival_positive += 1;
                    entry.survival_positive_w += signal_weight;
                }
            }
            if predicate_lc.contains("expression")
                || predicate_lc.contains("overexpress")
                || predicate_lc.contains("underexpress")
                || predicate_lc.contains("upregulat")
                || predicate_lc.contains("downregulat")
            {
                let expression_fields = [
                    f.subject_name.as_str(),
                    f.object_name.as_str(),
                    f.evidence.as_deref().unwrap_or(""),
                ];
                if fields_contain_any_ascii_keyword(
                    &expression_fields,
                    &["normal", "healthy", "non-tumor", "adjacent"],
                ) {
                    entry.expression_normal_mentions += 1;
                    entry.expression_normal_mentions_w += signal_weight;
                } else {
                    entry.expression_tumor_mentions += 1;
                    entry.expression_tumor_mentions_w += signal_weight;
                }
            }
            if predicate_lc.contains("inhibit")
                || predicate_lc.contains("bind")
                || predicate_lc.contains("target")
                || predicate_lc.contains("antagon")
            {
                entry.inhibitor_mentions += 1;
                entry.inhibitor_mentions_w += signal_weight;
            }
            if predicate_lc.contains("pathway") || predicate_lc.contains("reactome") {
                entry.pathway_mentions += 1;
                entry.pathway_mentions_w += signal_weight;
            }
            if predicate_lc.contains("pdb")
                || predicate_lc.contains("alphafold")
                || predicate_lc.contains("structure")
            {
                entry.structural_mentions += 1;
                entry.structural_mentions_w += signal_weight;
            }
            if predicate_lc.contains("pocket") || predicate_lc.contains("drugg") {
                entry.pocket_mentions += 1;
                entry.pocket_mentions_w += signal_weight;
            }

            if is_cancer_like(&f.object_name) {
                entry.cancer_mentions += 1;
                entry.cancer_mentions_w += signal_weight;
                *entry
                    .cancer_codes
                    .entry(f.object_name.to_uppercase())
                    .or_insert(0) += 1;
            }

            if let Some(sample_size) = f.sample_size {
                if sample_size > 0 {
                    entry.sample_size_sum += sample_size as f64;
                    entry.sample_obs += 1;
                }
            }
        }

        let t_scores = Instant::now();

        let ent_repo = EntStageRepository::new(self.db.clone());
        let signal_repo = Phase4SignalRepository::new(self.db.clone());
        let paper_repo = PaperRepository::new(self.db.clone());
        let candidate_count = candidates.len();
        let req_cancer_code = req.cancer_code.clone();
        let normalized_req_provider_cancer = req_cancer_code
            .as_deref()
            .and_then(normalize_provider_cancer_code);
        let fetch_cbio = should_fetch_cbio(candidate_count, &req);
        let fetch_gtex = should_fetch_gtex(candidate_count, &req);
        let fetch_chembl = should_fetch_chembl(candidate_count, &req);
        let fetch_reactome = should_fetch_reactome(candidate_count, &req);
        let symbol_list: Vec<String> = candidates.values().map(|c| c.gene_symbol.clone()).collect();
        let enrichment_by_symbol = ent_repo
            .get_enrichment_by_symbol(&symbol_list)
            .await
            .unwrap_or_default();
        let mut all_paper_ids_set = HashSet::new();
        let all_paper_ids: Vec<uuid::Uuid> = candidates
            .values()
            .flat_map(|c| c.paper_ids.iter().copied())
            .filter(|id| all_paper_ids_set.insert(*id))
            .collect();
        let paper_novelty_signals = paper_repo
            .find_novelty_signals_by_ids(&all_paper_ids)
            .await
            .unwrap_or_default();
        let t_enrich = Instant::now();

        let mut cohort_metrics = Vec::with_capacity(candidate_count);
        let mut by_gene_metrics: HashMap<uuid::Uuid, TargetMetrics> =
            HashMap::with_capacity(candidate_count);
        let mut component_sources_by_gene: HashMap<uuid::Uuid, BTreeMap<String, String>> =
            HashMap::with_capacity(candidate_count);
        let source_backed_only = phase4_source_backed_only();
        let semantic_fallback_enabled = phase4_semantic_fallback_enabled();
        let query_cache_only = phase4_query_cache_only();
        let disable_structural_proxy =
            source_backed_only || should_disable_structural_proxy(candidate_count);
        let allow_live_provider_fetch =
            !query_cache_only && should_allow_live_provider_fetch(candidate_count);
        let mut structural_source_missing: HashSet<uuid::Uuid> =
            HashSet::with_capacity(candidate_count);
        for (gene_id, candidate) in &candidates {
            let candidate_symbol_upper = candidate.gene_symbol.to_uppercase();
            let mut metrics = if source_backed_only {
                source_backed_default_metrics()
            } else {
                candidate.to_target_metrics()
            };
            let mut component_sources = default_component_sources(source_backed_only);
            if !source_backed_only && disable_structural_proxy {
                // For larger cohorts we avoid KG-derived structural proxies and
                // only accept source-backed structural signals.
                metrics.pdb_structure_count = 0;
                metrics.af_plddt_mean = 0.0;
                metrics.fpocket_best_score = 0.0;
                component_sources.insert(
                    "n5_structural_tractability".to_string(),
                    "source_missing".to_string(),
                );
                component_sources.insert(
                    "n6_pocket_detectability".to_string(),
                    "source_missing".to_string(),
                );
            }
            let enrichment = enrichment_by_symbol.get(&candidate_symbol_upper);
            if let Some(enrich) = enrichment {
                if !source_backed_only && enrich.mutation_count > 0 {
                    let source_mutation = (enrich.mutation_count as f64 / 20.0).clamp(0.0, 1.0);
                    metrics.mutation_freq = metrics.mutation_freq.max(source_mutation);
                    component_sources
                        .insert("n1_mutation_freq".to_string(), "ent_stage".to_string());
                }
                if enrich.pdb_structure_count > 0 {
                    metrics.pdb_structure_count =
                        metrics.pdb_structure_count.max(enrich.pdb_structure_count);
                    component_sources.insert(
                        "n5_structural_tractability".to_string(),
                        "ent_stage".to_string(),
                    );
                }
                if let Some(plddt) = enrich.af_plddt_mean {
                    metrics.af_plddt_mean = metrics.af_plddt_mean.max(plddt);
                    component_sources.insert(
                        "n5_structural_tractability".to_string(),
                        "ent_stage".to_string(),
                    );
                }
                if let Some(fpocket) = enrich.fpocket_best_score {
                    metrics.fpocket_best_score = metrics.fpocket_best_score.max(fpocket);
                    component_sources.insert(
                        "n6_pocket_detectability".to_string(),
                        "ent_stage".to_string(),
                    );
                }
                if !source_backed_only && enrich.chembl_inhibitor_count > 0 {
                    metrics.chembl_inhibitor_count = metrics
                        .chembl_inhibitor_count
                        .max(enrich.chembl_inhibitor_count);
                    component_sources
                        .insert("n7_novelty_score".to_string(), "ent_stage".to_string());
                }
                if !source_backed_only && enrich.pathway_count > 0 {
                    metrics.reactome_escape_pathway_count = metrics
                        .reactome_escape_pathway_count
                        .max(enrich.pathway_count);
                    component_sources.insert(
                        "n8_pathway_independence".to_string(),
                        "ent_stage".to_string(),
                    );
                }
            }

            if (source_backed_only || disable_structural_proxy)
                && component_sources
                    .get("n5_structural_tractability")
                    .is_some_and(|v| v == "source_missing")
                && component_sources
                    .get("n6_pocket_detectability")
                    .is_some_and(|v| v == "source_missing")
            {
                structural_source_missing.insert(*gene_id);
            }

            let inferred_cancer = candidate
                .infer_cancer_code()
                .or_else(|| req_cancer_code.clone())
                .unwrap_or_else(|| "UNK".to_string());

            let provider_cancer = normalized_req_provider_cancer
                .clone()
                .or_else(|| normalize_provider_cancer_code(&inferred_cancer));

            if fetch_cbio {
                let mut cbio_source: Option<&str> = None;
                let mut cbio_mutation = None;
                if let Some(cancer_code) = provider_cancer.as_deref() {
                    cbio_mutation = get_cached_cbio_mutation_frequency(
                        &signal_repo,
                        &candidate.gene_symbol,
                        cancer_code,
                        allow_live_provider_fetch,
                    )
                    .await;
                    if cbio_mutation.is_some() {
                        cbio_source = Some("cbioportal_table");
                    }
                }
                if cbio_mutation.is_none() {
                    if let Some(cancer_code) = provider_cancer.as_deref() {
                        cbio_mutation = get_cached_cosmic_mutation_frequency(
                            &signal_repo,
                            &candidate.gene_symbol,
                            cancer_code,
                            allow_live_provider_fetch,
                        )
                        .await;
                        if cbio_mutation.is_some() {
                            cbio_source = Some("cosmic_table");
                        }
                    }
                }
                if cbio_mutation.is_none() {
                    cbio_mutation = get_cached_cbio_mutation_frequency_any_cancer(
                        &signal_repo,
                        &candidate.gene_symbol,
                    )
                    .await;
                    if cbio_mutation.is_some() {
                        cbio_source = Some("cbioportal_table_any_cancer");
                    }
                }
                if cbio_mutation.is_none() {
                    cbio_mutation = get_cached_cosmic_mutation_frequency_any_cancer(
                        &signal_repo,
                        &candidate.gene_symbol,
                        allow_live_provider_fetch,
                    )
                    .await;
                    if cbio_mutation.is_some() {
                        cbio_source = Some("cosmic_table_any_cancer");
                    }
                }
                if let Some(v) = cbio_mutation {
                    metrics.mutation_freq = v;
                    if let Some(src) = cbio_source {
                        component_sources.insert("n1_mutation_freq".to_string(), src.to_string());
                    }
                }
            }

            if let Some((novelty, used_citations)) =
                candidate.source_backed_literature_novelty(&paper_novelty_signals)
            {
                metrics.literature_novelty_velocity = novelty;
                component_sources.insert(
                    "n9_literature_novelty".to_string(),
                    if used_citations {
                        "papers_metadata_citations"
                    } else {
                        "papers_metadata"
                    }
                    .to_string(),
                );
            }

            if let Some(depmap) = depmap_cache() {
                if let Some(ceres) = depmap.get_mean_ceres(&candidate.gene_symbol, &inferred_cancer)
                {
                    metrics.crispr_dependency = ceres;
                    component_sources.insert(
                        "n2_crispr_dependency".to_string(),
                        "depmap_cache".to_string(),
                    );
                }
            }

            if should_fetch_tcga(candidate_count, provider_cancer.is_some()) {
                if let Some(cancer_code) = provider_cancer.as_deref() {
                    if let Some(tcga_survival_score) = get_cached_tcga_survival_score(
                        &signal_repo,
                        &candidate.gene_symbol,
                        cancer_code,
                        allow_live_provider_fetch,
                    )
                    .await
                    {
                        metrics.survival_correlation = tcga_survival_score;
                        component_sources.insert(
                            "n3_survival_correlation".to_string(),
                            "tcga_table".to_string(),
                        );
                    }
                }
            }

            if fetch_gtex {
                if let Some(gtex_expr_score) = get_cached_gtex_expression_score(
                    &signal_repo,
                    &candidate.gene_symbol,
                    allow_live_provider_fetch,
                )
                .await
                {
                    metrics.expression_specificity = (1.0 + 4.0 * gtex_expr_score).clamp(0.5, 5.0);
                    component_sources.insert(
                        "n4_expression_specificity".to_string(),
                        "gtex_table".to_string(),
                    );
                }
            }

            if source_backed_only
                && semantic_fallback_enabled
                && component_sources
                    .get("n3_survival_correlation")
                    .is_some_and(|v| v == "source_missing")
            {
                if let Some(semantic_survival_score) = candidate.semantic_survival_score() {
                    metrics.survival_correlation = semantic_survival_score;
                    component_sources.insert(
                        "n3_survival_correlation".to_string(),
                        "kg_fact_semantic_fallback".to_string(),
                    );
                }
            }
            if source_backed_only
                && semantic_fallback_enabled
                && component_sources
                    .get("n4_expression_specificity")
                    .is_some_and(|v| v == "source_missing")
            {
                if let Some(semantic_expr_score) = candidate.semantic_expression_specificity() {
                    metrics.expression_specificity = semantic_expr_score;
                    component_sources.insert(
                        "n4_expression_specificity".to_string(),
                        "kg_fact_semantic_fallback".to_string(),
                    );
                }
            }

            if fetch_chembl {
                if let Some(chembl_count) = get_cached_chembl_inhibitor_count(
                    &signal_repo,
                    &candidate.gene_symbol,
                    allow_live_provider_fetch,
                )
                .await
                {
                    metrics.chembl_inhibitor_count = if source_backed_only {
                        chembl_count
                    } else {
                        metrics.chembl_inhibitor_count.max(chembl_count)
                    };
                    component_sources
                        .insert("n7_novelty_score".to_string(), "chembl_table".to_string());
                }
            }

            if fetch_reactome {
                if let Some(reactome_count) = get_cached_reactome_pathway_count(
                    &signal_repo,
                    &candidate.gene_symbol,
                    allow_live_provider_fetch,
                )
                .await
                {
                    metrics.reactome_escape_pathway_count = if source_backed_only {
                        reactome_count
                    } else {
                        metrics.reactome_escape_pathway_count.max(reactome_count)
                    };
                    component_sources.insert(
                        "n8_pathway_independence".to_string(),
                        "reactome_table".to_string(),
                    );
                }
            }

            cohort_metrics.push((*gene_id, metrics.clone()));
            by_gene_metrics.insert(*gene_id, metrics);
            component_sources_by_gene.insert(*gene_id, component_sources);
        }

        let cohort_scores = scorer::PrioritizationEngine::calculate_scores(&cohort_metrics);

        let mut results = Vec::with_capacity(candidate_count);

        for (gene_id, candidate) in &candidates {
            if let Some(score_res) = cohort_scores.get(gene_id) {
                let Some(metrics) = by_gene_metrics.get(gene_id) else {
                    continue;
                };
                let candidate_symbol_upper = candidate.gene_symbol.to_uppercase();

                let inferred_cancer = candidate
                    .infer_cancer_code()
                    .or_else(|| req_cancer_code.clone())
                    .unwrap_or_else(|| "UNK".to_string());

                let effective_score = score_res.composite_score.clamp(0.0, 0.98);
                let confidence_factor =
                    (0.55 + 0.45 * candidate.mean_confidence().clamp(0.0, 1.0)).clamp(0.55, 1.0);
                let confidence_adj = (effective_score * confidence_factor).clamp(0.0, 0.95);

                let penalties = scorer::PenaltyInputs {
                    chembl_inhibitor_count: metrics.chembl_inhibitor_count,
                    expression_ratio: metrics.expression_specificity,
                    has_pdb: metrics.pdb_structure_count > 0,
                    alphafold_plddt: Some(metrics.af_plddt_mean),
                };

                let tier = scorer::determine_shortlist_tier(
                    confidence_adj,
                    Some(metrics.mutation_freq),
                    score_res.n5_structural_tractability,
                    &penalties,
                    score_res.n7_novelty_score,
                );

                let shortlist_tier = match tier {
                    scorer::ShortlistTier::Primary => "primary".to_string(),
                    scorer::ShortlistTier::Secondary => "secondary".to_string(),
                    scorer::ShortlistTier::Excluded => "excluded".to_string(),
                };

                let mut flags: Vec<String> = candidate.flags.iter().cloned().collect();
                if metrics.expression_specificity < 1.20 {
                    flags.push("WARNING_LOW_TUMOR_SPECIFICITY".to_string());
                }
                if metrics.pdb_structure_count == 0 && metrics.af_plddt_mean < 50.0 {
                    flags.push("WARNING_STRUCTURALLY_UNRESOLVED".to_string());
                }
                if penalties.chembl_inhibitor_count > 50 && score_res.n7_novelty_score < 0.20 {
                    flags.push("HARD_EXCLUSION_SATURATED_TARGET".to_string());
                }
                if !source_backed_only
                    && !enrichment_by_symbol.contains_key(&candidate_symbol_upper)
                {
                    flags.push("COVERAGE_PROXY_ONLY".to_string());
                }
                if structural_source_missing.contains(gene_id) {
                    flags.push("STRUCTURAL_SOURCE_MISSING".to_string());
                }

                results.push(QueryResult {
                    rank: 0,
                    percentile: None,
                    gene_symbol: candidate.gene_symbol.clone(),
                    cancer_code: inferred_cancer,
                    composite_score: effective_score,
                    confidence_adj,
                    shortlist_tier,
                    flags,
                    component_sources: component_sources_by_gene.get(gene_id).cloned(),
                    component_breakdown: Some(score_res.clone()),
                    metrics: Some(metrics.clone()),
                });
            }
        }

        results.sort_by(|a, b| {
            b.composite_score
                .partial_cmp(&a.composite_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    b.confidence_adj
                        .partial_cmp(&a.confidence_adj)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        let total = results.len().max(1) as f64;
        for (i, res) in results.iter_mut().enumerate() {
            res.rank = i + 1;
            let p = 100.0 * (1.0 - (i as f64 / total));
            res.percentile = Some(p.clamp(0.0, 100.0));
        }
        results.truncate(req.max_results);

        let large_cohort = should_prewarm_large_cohort(candidate_count);
        if query_cache_only || large_cohort {
            let prewarm_take = if large_cohort { 20 } else { 12 };
            let prewarm_genes: Vec<String> = results
                .iter()
                .take(prewarm_take)
                .map(|r| r.gene_symbol.trim().to_uppercase())
                .filter(|s| !s.is_empty())
                .collect();
            if !prewarm_genes.is_empty() {
                let db = self.db.clone();
                let cancer_code = req.cancer_code.clone();
                tokio::spawn(async move {
                    if large_cohort {
                        prewarm_phase4_large_cohort_signals(db, prewarm_genes, cancer_code).await;
                    } else {
                        prewarm_phase4_provider_signals(db, prewarm_genes, cancer_code).await;
                    }
                });
            }
        }

        info!(
            target: "ferrumyx_ranker_perf",
            facts_ms = (t_facts - t0).as_millis() as u64,
            conflicts_ms = (t_conflicts - t_facts).as_millis() as u64,
            scores_ms = (t_scores - t_conflicts).as_millis() as u64,
            enrich_ms = (t_enrich - t_scores).as_millis() as u64,
            rank_ms = (Instant::now() - t_enrich).as_millis() as u64,
            total_ms = (Instant::now() - t0).as_millis() as u64,
            candidate_count = candidates.len(),
            result_count = results.len(),
            "ranker query complete"
        );

        Ok(results)
    }

    pub async fn refresh_provider_signals(
        &self,
        mut request: ProviderRefreshRequest,
    ) -> anyhow::Result<ProviderRefreshReport> {
        let started = Instant::now();
        let mut uniq = HashSet::new();
        let max_genes = request.max_genes.clamp(1, 200);
        let batch_size = request.batch_size.clamp(1, 32);
        let force_refresh = !request.offline_strict;
        let retries = if request.offline_strict {
            0
        } else {
            request.retries.min(3)
        };

        let genes: Vec<String> = request
            .genes
            .drain(..)
            .map(|g| g.trim().to_uppercase())
            .filter(|g| !g.is_empty())
            .filter(|g| uniq.insert(g.clone()))
            .take(max_genes)
            .collect();

        let mut report = ProviderRefreshReport {
            genes_requested: genes.len(),
            ..ProviderRefreshReport::default()
        };

        if genes.is_empty() {
            report.duration_ms = started.elapsed().as_millis() as u64;
            return Ok(report);
        }

        let signal_repo = Phase4SignalRepository::new(self.db.clone());
        let cancer_code = request
            .cancer_code
            .and_then(|c| normalize_provider_cancer_code(&c));
        let refresh_started_at = chrono::Utc::now();
        let cbio_policy = provider_refresh_policy(&signal_repo, "cbioportal").await;
        let cosmic_policy = provider_refresh_policy(&signal_repo, "cosmic").await;
        let gtex_policy = provider_refresh_policy(&signal_repo, "gtex").await;
        let tcga_policy = provider_refresh_policy(&signal_repo, "tcga").await;
        let chembl_policy = provider_refresh_policy(&signal_repo, "chembl").await;
        let reactome_policy = provider_refresh_policy(&signal_repo, "reactome").await;
        report.provider_decisions.insert(
            "cbioportal".to_string(),
            format!(
                "{} (interval={}s)",
                cbio_policy.reason, cbio_policy.interval_secs
            ),
        );
        report.provider_decisions.insert(
            "cosmic".to_string(),
            format!(
                "{} (interval={}s)",
                cosmic_policy.reason, cosmic_policy.interval_secs
            ),
        );
        report.provider_decisions.insert(
            "gtex".to_string(),
            format!(
                "{} (interval={}s)",
                gtex_policy.reason, gtex_policy.interval_secs
            ),
        );
        report.provider_decisions.insert(
            "tcga".to_string(),
            format!(
                "{} (interval={}s)",
                tcga_policy.reason, tcga_policy.interval_secs
            ),
        );
        report.provider_decisions.insert(
            "chembl".to_string(),
            format!(
                "{} (interval={}s)",
                chembl_policy.reason, chembl_policy.interval_secs
            ),
        );
        report.provider_decisions.insert(
            "reactome".to_string(),
            format!(
                "{} (interval={}s)",
                reactome_policy.reason, reactome_policy.interval_secs
            ),
        );

        for (batch_index, batch) in genes.chunks(batch_size).enumerate() {
            let mut tasks = tokio::task::JoinSet::new();
            let batch_start = batch_index * batch_size;
            for (offset, gene) in batch.iter().enumerate() {
                tasks.spawn(refresh_provider_signals_for_gene(
                    self.db.clone(),
                    batch_start + offset,
                    gene.clone(),
                    cancer_code.clone(),
                    cbio_policy.clone(),
                    cosmic_policy.clone(),
                    gtex_policy.clone(),
                    tcga_policy.clone(),
                    chembl_policy.clone(),
                    reactome_policy.clone(),
                    force_refresh,
                    retries,
                ));
            }

            let mut batch_results = Vec::with_capacity(batch.len());
            while let Some(result) = tasks.join_next().await {
                let gene_result =
                    result.map_err(|err| anyhow::anyhow!("provider refresh task failed: {err}"))?;
                batch_results.push(gene_result);
            }

            batch_results.sort_by_key(|result| result.gene_index);
            for gene_result in batch_results {
                apply_gene_provider_refresh_result(&mut report, gene_result);
            }
        }

        match materialize_provider_cache_facts_to_kg(
            self.db.clone(),
            &genes,
            cancer_code.as_deref(),
        )
        .await
        {
            Ok(stats) => {
                report.kg_backfill_inserted = stats.inserted;
                report.kg_backfill_deleted = stats.deleted;
            }
            Err(err) => {
                report.kg_backfill_failed += 1;
                warn!(
                    target: "ferrumyx_provider_refresh",
                    error = %err,
                    "failed to backfill provider cache rows into KG facts"
                );
            }
        }

        report.duration_ms = started.elapsed().as_millis() as u64;
        let refresh_finished_at = chrono::Utc::now();
        let mut policies = HashMap::new();
        policies.insert("cbioportal", cbio_policy);
        policies.insert("cosmic", cosmic_policy);
        policies.insert("gtex", gtex_policy);
        policies.insert("tcga", tcga_policy);
        policies.insert("chembl", chembl_policy);
        policies.insert("reactome", reactome_policy);
        persist_provider_refresh_history(
            &signal_repo,
            &report,
            &policies,
            refresh_started_at,
            refresh_finished_at,
        )
        .await;
        info!(
            target: "ferrumyx_provider_refresh",
            genes_processed = report.genes_processed,
            cbio_success = report.cbio_success,
            cbio_failed = report.cbio_failed,
            cbio_skipped = report.cbio_skipped,
            cosmic_success = report.cosmic_success,
            cosmic_failed = report.cosmic_failed,
            cosmic_skipped = report.cosmic_skipped,
            gtex_success = report.gtex_success,
            gtex_failed = report.gtex_failed,
            gtex_skipped = report.gtex_skipped,
            tcga_success = report.tcga_success,
            tcga_failed = report.tcga_failed,
            chembl_success = report.chembl_success,
            chembl_failed = report.chembl_failed,
            chembl_skipped = report.chembl_skipped,
            reactome_success = report.reactome_success,
            reactome_failed = report.reactome_failed,
            reactome_skipped = report.reactome_skipped,
            kg_backfill_inserted = report.kg_backfill_inserted,
            kg_backfill_deleted = report.kg_backfill_deleted,
            kg_backfill_failed = report.kg_backfill_failed,
            duration_ms = report.duration_ms,
            "provider signal refresh complete"
        );

        Ok(report)
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct ProviderKgBackfillStats {
    inserted: usize,
    deleted: usize,
}

const PROVIDER_BACKFILL_PREDICATES: &[&str] = &[
    "mutation_frequency_cbioportal",
    "mutation_frequency_cosmic",
    "survival_association_tcga",
    "expression_signal_gtex",
    "inhibitor_evidence_chembl",
    "pathway_membership_reactome",
];

async fn materialize_provider_cache_facts_to_kg(
    db: Arc<Database>,
    genes: &[String],
    cancer_code_hint: Option<&str>,
) -> anyhow::Result<ProviderKgBackfillStats> {
    if genes.is_empty() {
        return Ok(ProviderKgBackfillStats::default());
    }

    let signal_repo = Phase4SignalRepository::new(db.clone());
    let entity_repo = EntityRepository::new(db.clone());
    let fact_repo = KgFactRepository::new(db);
    let mut stats = ProviderKgBackfillStats::default();

    let mut dedup = HashSet::new();
    for gene in genes {
        let gene_symbol = gene.trim().to_uppercase();
        if gene_symbol.is_empty() || !dedup.insert(gene_symbol.clone()) {
            continue;
        }

        let Some(gene_id) = ensure_entity_id(
            &entity_repo,
            DbEntityType::Gene,
            &gene_symbol,
            "provider_cache",
        )
        .await?
        else {
            continue;
        };

        let existing = fact_repo.find_by_subject(gene_id).await.unwrap_or_default();
        for fact in existing {
            if PROVIDER_BACKFILL_PREDICATES
                .iter()
                .any(|p| fact.predicate.eq_ignore_ascii_case(p))
            {
                if fact_repo.delete(fact.id).await.is_ok() {
                    stats.deleted += 1;
                }
            }
        }

        let mut new_facts: Vec<KgFact> = Vec::new();

        let cbio = if let Some(cc) = cancer_code_hint {
            signal_repo
                .find_cbio_mutation_frequency(&gene_symbol, cc)
                .await
                .ok()
                .flatten()
        } else {
            signal_repo
                .find_cbio_mutation_frequency_any_cancer(&gene_symbol)
                .await
                .ok()
                .flatten()
        };
        if let Some(signal) = cbio {
            let cc = signal.cancer_code.trim().to_uppercase();
            if let Some(cancer_id) =
                ensure_entity_id(&entity_repo, DbEntityType::CancerType, &cc, "cbioportal").await?
            {
                let mut fact = KgFact::new(
                    uuid::Uuid::nil(),
                    gene_id,
                    gene_symbol.clone(),
                    "mutation_frequency_cbioportal".to_string(),
                    cancer_id,
                    cc,
                );
                fact.confidence = signal.mutation_frequency.clamp(0.0, 1.0) as f32;
                fact.evidence_type = "provider_fact".to_string();
                fact.evidence = Some(format!(
                    "provider=cbioportal;study={};mutated={};profiled={};freq={:.5};fetched_at={}",
                    signal.study_id,
                    signal.mutated_sample_count,
                    signal.profiled_sample_count,
                    signal.mutation_frequency,
                    signal.fetched_at
                ));
                new_facts.push(fact);
            }
        }

        let cosmic = if let Some(cc) = cancer_code_hint {
            signal_repo
                .find_cosmic_mutation_frequency(&gene_symbol, cc)
                .await
                .ok()
                .flatten()
        } else {
            signal_repo
                .find_cosmic_mutation_frequency_any_cancer(&gene_symbol)
                .await
                .ok()
                .flatten()
        };
        if let Some(signal) = cosmic {
            let cc = signal.cancer_code.trim().to_uppercase();
            if let Some(cancer_id) =
                ensure_entity_id(&entity_repo, DbEntityType::CancerType, &cc, "cosmic").await?
            {
                let mut fact = KgFact::new(
                    uuid::Uuid::nil(),
                    gene_id,
                    gene_symbol.clone(),
                    "mutation_frequency_cosmic".to_string(),
                    cancer_id,
                    cc,
                );
                fact.confidence = signal.mutation_frequency.clamp(0.0, 1.0) as f32;
                fact.evidence_type = "provider_fact".to_string();
                fact.evidence = Some(format!(
                    "provider=cosmic;mutated={};profiled={};freq={:.5};fetched_at={}",
                    signal.mutated_sample_count,
                    signal.profiled_sample_count,
                    signal.mutation_frequency,
                    signal.fetched_at
                ));
                new_facts.push(fact);
            }
        }

        if let Some(cc) = cancer_code_hint {
            if let Some(signal) = signal_repo
                .find_tcga_survival(&gene_symbol, cc)
                .await
                .ok()
                .flatten()
            {
                let cancer_code = signal.cancer_code.trim().to_uppercase();
                if let Some(cancer_id) =
                    ensure_entity_id(&entity_repo, DbEntityType::CancerType, &cancer_code, "tcga")
                        .await?
                {
                    let mut fact = KgFact::new(
                        uuid::Uuid::nil(),
                        gene_id,
                        gene_symbol.clone(),
                        "survival_association_tcga".to_string(),
                        cancer_id,
                        cancer_code,
                    );
                    fact.confidence = signal.survival_score.abs().clamp(0.0, 1.0) as f32;
                    fact.evidence_type = "provider_fact".to_string();
                    fact.evidence = Some(format!(
                        "provider=tcga;project={};score={:.5};fetched_at={}",
                        signal.tcga_project_id, signal.survival_score, signal.fetched_at
                    ));
                    new_facts.push(fact);
                }
            }
        }

        if let Some(signal) = signal_repo
            .find_gtex_expression(&gene_symbol)
            .await
            .ok()
            .flatten()
        {
            if let Some(obj_id) = ensure_entity_id(
                &entity_repo,
                DbEntityType::Disease,
                "GTEX_PAN_TISSUE",
                "gtex",
            )
            .await?
            {
                let mut fact = KgFact::new(
                    uuid::Uuid::nil(),
                    gene_id,
                    gene_symbol.clone(),
                    "expression_signal_gtex".to_string(),
                    obj_id,
                    "GTEX_PAN_TISSUE".to_string(),
                );
                fact.confidence = signal.expression_score.abs().min(20.0) as f32 / 20.0;
                fact.evidence_type = "provider_fact".to_string();
                fact.evidence = Some(format!(
                    "provider=gtex;score={:.5};fetched_at={}",
                    signal.expression_score, signal.fetched_at
                ));
                new_facts.push(fact);
            }
        }

        if let Some(signal) = signal_repo
            .find_chembl_target(&gene_symbol)
            .await
            .ok()
            .flatten()
        {
            if let Some(obj_id) = ensure_entity_id(
                &entity_repo,
                DbEntityType::Chemical,
                "CHEMBL_TARGET",
                "chembl",
            )
            .await?
            {
                let mut fact = KgFact::new(
                    uuid::Uuid::nil(),
                    gene_id,
                    gene_symbol.clone(),
                    "inhibitor_evidence_chembl".to_string(),
                    obj_id,
                    "CHEMBL_TARGET".to_string(),
                );
                fact.confidence = (signal.inhibitor_count as f64).min(25.0) as f32 / 25.0;
                fact.evidence_type = "provider_fact".to_string();
                fact.evidence = Some(format!(
                    "provider=chembl;inhibitor_count={};fetched_at={}",
                    signal.inhibitor_count, signal.fetched_at
                ));
                new_facts.push(fact);
            }
        }

        if let Some(signal) = signal_repo
            .find_reactome_gene(&gene_symbol)
            .await
            .ok()
            .flatten()
        {
            if let Some(obj_id) = ensure_entity_id(
                &entity_repo,
                DbEntityType::Pathway,
                "REACTOME_PATHWAYS",
                "reactome",
            )
            .await?
            {
                let mut fact = KgFact::new(
                    uuid::Uuid::nil(),
                    gene_id,
                    gene_symbol.clone(),
                    "pathway_membership_reactome".to_string(),
                    obj_id,
                    "REACTOME_PATHWAYS".to_string(),
                );
                fact.confidence = (signal.pathway_count as f64).min(20.0) as f32 / 20.0;
                fact.evidence_type = "provider_fact".to_string();
                fact.evidence = Some(format!(
                    "provider=reactome;pathway_count={};fetched_at={}",
                    signal.pathway_count, signal.fetched_at
                ));
                new_facts.push(fact);
            }
        }

        for fact in new_facts {
            if fact_repo.insert(&fact).await.is_ok() {
                stats.inserted += 1;
            }
        }
    }

    Ok(stats)
}

async fn ensure_entity_id(
    entity_repo: &EntityRepository,
    entity_type: DbEntityType,
    display_name: &str,
    source_db: &str,
) -> anyhow::Result<Option<uuid::Uuid>> {
    let trimmed = display_name.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let canonical = canonical_entity_key(entity_type, trimmed);
    let external_id = format!("FERRUMYX:{}", canonical);
    if let Some(existing) = entity_repo
        .find_by_external_id(&external_id)
        .await?
        .into_iter()
        .next()
    {
        return Ok(Some(existing.id));
    }

    let mut entity = DbEntity::new(
        entity_type,
        trimmed.to_string(),
        external_id,
        source_db.to_string(),
    );
    entity.canonical_name = Some(trimmed.to_uppercase());
    entity_repo.insert(&entity).await?;
    Ok(Some(entity.id))
}

fn canonical_entity_key(entity_type: DbEntityType, name: &str) -> String {
    let mut normalized = name.trim().to_uppercase();
    normalized = normalized
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    while normalized.contains("__") {
        normalized = normalized.replace("__", "_");
    }
    format!("{}:{}", entity_type, normalized.trim_matches('_'))
}

fn depmap_cache() -> Option<&'static DepMapCache> {
    static CACHE: OnceLock<Option<DepMapCache>> = OnceLock::new();
    CACHE
        .get_or_init(|| DepMapCache::load_default().ok())
        .as_ref()
}

fn normalize_provider_cancer_code(cancer_code: &str) -> Option<String> {
    let mut code = cancer_code.trim().to_uppercase();
    if code.is_empty() {
        return None;
    }
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

fn should_fetch_gtex(candidate_count: usize, req: &QueryRequest) -> bool {
    if candidate_count == 0 {
        return false;
    }
    let _ = req;
    true
}

fn phase4_source_backed_only() -> bool {
    !std::env::var("FERRUMYX_PHASE4_SOURCE_BACKED_ONLY")
        .ok()
        .is_some_and(|v| v == "0" || v.eq_ignore_ascii_case("false"))
}

fn phase4_semantic_fallback_enabled() -> bool {
    std::env::var("FERRUMYX_PHASE4_N3N4_SEMANTIC_FALLBACK")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn phase4_query_cache_only() -> bool {
    std::env::var("FERRUMYX_PHASE4_QUERY_CACHE_ONLY")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn should_disable_structural_proxy(candidate_count: usize) -> bool {
    if std::env::var("FERRUMYX_PHASE4_STRUCTURAL_SOURCE_ONLY")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    {
        return true;
    }
    candidate_count > 8
}

fn should_fetch_cbio(candidate_count: usize, req: &QueryRequest) -> bool {
    if candidate_count == 0 {
        return false;
    }
    let _ = req;
    true
}

fn should_fetch_tcga(candidate_count: usize, has_cancer_context: bool) -> bool {
    if candidate_count == 0 {
        return false;
    }
    has_cancer_context
}

fn should_fetch_chembl(candidate_count: usize, _req: &QueryRequest) -> bool {
    candidate_count > 0
}

fn should_fetch_reactome(candidate_count: usize, _req: &QueryRequest) -> bool {
    candidate_count > 0
}

fn should_allow_live_provider_fetch(candidate_count: usize) -> bool {
    let max_candidates = std::env::var("FERRUMYX_PHASE4_PROVIDER_LIVE_FETCH_MAX_CANDIDATES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|v| v.clamp(1, 128))
        .unwrap_or(8);
    candidate_count > 0 && candidate_count <= max_candidates
}

fn should_prewarm_large_cohort(candidate_count: usize) -> bool {
    (9..=1000).contains(&candidate_count)
}

async fn prewarm_phase4_provider_signals(
    db: Arc<Database>,
    genes: Vec<String>,
    cancer_code: Option<String>,
) {
    let engine = TargetQueryEngine::new(db);
    let _ = engine
        .refresh_provider_signals(ProviderRefreshRequest {
            genes,
            cancer_code,
            max_genes: 24,
            batch_size: 6,
            retries: 1,
            offline_strict: false,
        })
        .await;
}

async fn prewarm_phase4_large_cohort_signals(
    db: Arc<Database>,
    genes: Vec<String>,
    cancer_code: Option<String>,
) {
    prewarm_phase4_provider_signals(db.clone(), genes.clone(), cancer_code).await;
    prewarm_structural_signals(db, genes).await;
}

fn structural_prewarm_enabled() -> bool {
    !std::env::var("FERRUMYX_PHASE4_STRUCTURAL_PREWARM_ENABLED")
        .ok()
        .is_some_and(|v| v == "0" || v.eq_ignore_ascii_case("false"))
}

fn structural_prewarm_max_genes() -> usize {
    std::env::var("FERRUMYX_PHASE4_STRUCTURAL_PREWARM_MAX_GENES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|v| v.clamp(1, 32))
        .unwrap_or(8)
}

fn structural_cache_dir() -> PathBuf {
    std::env::var("FERRUMYX_STRUCTURAL_CACHE_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/structural_cache"))
}

async fn prewarm_structural_signals(db: Arc<Database>, genes: Vec<String>) {
    if !structural_prewarm_enabled() {
        return;
    }

    let mut seen = HashSet::new();
    let clean: Vec<String> = genes
        .into_iter()
        .map(|g| g.trim().to_uppercase())
        .filter(|g| !g.is_empty() && seen.insert(g.clone()))
        .take(structural_prewarm_max_genes())
        .collect();
    if clean.is_empty() {
        return;
    }

    let ent_repo = EntStageRepository::new(db);
    let gene_rows = match ent_repo.find_genes_by_symbol(&clean).await {
        Ok(rows) => rows,
        Err(e) => {
            warn!(
                target: "ferrumyx_ranker_structural",
                error = %e,
                "structural prewarm skipped: failed to read gene metadata"
            );
            return;
        }
    };
    if gene_rows.is_empty() {
        return;
    }

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
    {
        Ok(c) => c,
        Err(_) => reqwest::Client::new(),
    };

    let cache_dir = structural_cache_dir();

    for symbol in clean {
        let Some(gene) = gene_rows.get(&symbol) else {
            continue;
        };
        let Some(uniprot_id) = gene
            .uniprot_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };

        let pdb_path = match fetch_alphafold_pdb_cached(&client, &cache_dir, uniprot_id).await {
            Ok(path) => path,
            Err(e) => {
                warn!(
                    target: "ferrumyx_ranker_structural",
                    gene = %symbol,
                    uniprot = %uniprot_id,
                    error = %e,
                    "structural prewarm: alphafold fetch failed"
                );
                continue;
            }
        };

        let plddt_mean = parse_plddt_mean_from_pdb(&pdb_path).map(|v| v as f32);
        let structure_id = uuid::Uuid::new_v4();
        let structure = EntStructure {
            id: structure_id,
            gene_id: gene.id,
            pdb_ids: None,
            best_resolution: None,
            exp_method: Some("alphafold_model".to_string()),
            af_accession: Some(uniprot_id.to_string()),
            af_plddt_mean: plddt_mean,
            af_plddt_active: plddt_mean,
            has_pdb: false,
            has_alphafold: true,
            updated_at: chrono::Utc::now(),
        };
        if let Err(e) = ent_repo.upsert_structure_signal(&structure).await {
            warn!(
                target: "ferrumyx_ranker_structural",
                gene = %symbol,
                error = %e,
                "structural prewarm: failed to upsert ent_structure"
            );
            continue;
        }

        if let Some(fpocket_score) = run_fpocket_best_score(&pdb_path).await {
            let drug = EntDruggability {
                id: uuid::Uuid::new_v4(),
                structure_id,
                fpocket_score: Some(fpocket_score as f32),
                fpocket_volume: None,
                fpocket_pocket_count: None,
                dogsitescorer: None,
                overall_assessment: Some("fpocket_cli".to_string()),
                assessed_at: chrono::Utc::now(),
            };
            if let Err(e) = ent_repo.upsert_druggability_signal(&drug).await {
                warn!(
                    target: "ferrumyx_ranker_structural",
                    gene = %symbol,
                    error = %e,
                    "structural prewarm: failed to upsert ent_druggability"
                );
            }
        }
    }
}

async fn fetch_alphafold_pdb_cached(
    client: &reqwest::Client,
    cache_dir: &Path,
    uniprot_id: &str,
) -> anyhow::Result<PathBuf> {
    let uniprot = uniprot_id.trim().to_uppercase();
    if uniprot.is_empty() {
        anyhow::bail!("empty uniprot id");
    }

    tokio::fs::create_dir_all(cache_dir).await?;
    let file_name = format!("AF-{}-F1-model_v4.pdb", uniprot);
    let file_path = cache_dir.join(&file_name);
    if file_path.exists() {
        return Ok(file_path);
    }

    let url = format!("https://alphafold.ebi.ac.uk/files/{}", file_name);
    let resp = client.get(&url).send().await?.error_for_status()?;
    let bytes = resp.bytes().await?;
    tokio::fs::write(&file_path, bytes).await?;
    Ok(file_path)
}

fn parse_plddt_mean_from_pdb(path: &Path) -> Option<f64> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut sum = 0.0f64;
    let mut count = 0usize;
    for line in text.lines() {
        if !(line.starts_with("ATOM") || line.starts_with("HETATM")) {
            continue;
        }
        let Some(raw) = line.get(60..66) else {
            continue;
        };
        let Ok(v) = raw.trim().parse::<f64>() else {
            continue;
        };
        sum += v;
        count += 1;
    }
    if count == 0 {
        None
    } else {
        Some((sum / count as f64).clamp(0.0, 100.0))
    }
}

fn fpocket_enabled() -> bool {
    !std::env::var("FERRUMYX_STRUCTURAL_FPOCKET_ENABLED")
        .ok()
        .is_some_and(|v| v == "0" || v.eq_ignore_ascii_case("false"))
}

async fn run_fpocket_best_score(pdb_path: &Path) -> Option<f64> {
    if !fpocket_enabled() {
        return None;
    }

    let bin = std::env::var("FERRUMYX_FPOCKET_BIN").unwrap_or_else(|_| "fpocket".to_string());
    let output = tokio::process::Command::new(&bin)
        .arg("-f")
        .arg(pdb_path)
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stem = pdb_path.file_stem()?.to_string_lossy();
    let out_dir = pdb_path.with_file_name(format!("{}_out", stem));
    parse_fpocket_best_score(&out_dir)
}

fn parse_fpocket_best_score(out_dir: &Path) -> Option<f64> {
    let mut candidates = Vec::new();
    let info_file = out_dir.join(format!(
        "{}_info.txt",
        out_dir
            .file_name()?
            .to_string_lossy()
            .trim_end_matches("_out")
    ));
    if info_file.exists() {
        candidates.push(info_file);
    }
    if let Ok(entries) = std::fs::read_dir(out_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("txt") {
                candidates.push(p);
            }
        }
    }

    let mut best: Option<f64> = None;
    for file in candidates {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        for line in text.lines() {
            let line_lc = line.to_ascii_lowercase();
            if !line_lc.contains("score") || line_lc.contains("volume") {
                continue;
            }
            if let Some(v) = extract_first_float(line) {
                if (0.0..=1.5).contains(&v) {
                    best = Some(best.map_or(v, |cur| cur.max(v)));
                }
            }
        }
    }
    best.map(|v| v.clamp(0.0, 1.0))
}

fn extract_first_float(input: &str) -> Option<f64> {
    let mut buf = String::new();
    for ch in input.chars() {
        if ch.is_ascii_digit() || matches!(ch, '.' | '-' | '+') {
            buf.push(ch);
        } else if !buf.is_empty() {
            if let Ok(v) = buf.parse::<f64>() {
                return Some(v);
            }
            buf.clear();
        }
    }
    if buf.is_empty() {
        None
    } else {
        buf.parse::<f64>().ok()
    }
}

async fn retry_fetch_f64<F, Fut>(retries: u8, mut op: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<f64>>,
{
    for attempt in 0..=retries {
        if op().await.is_some() {
            return true;
        }
        if attempt < retries {
            tokio::time::sleep(std::time::Duration::from_millis(150 * (attempt as u64 + 1))).await;
        }
    }
    false
}

async fn retry_fetch_u32<F, Fut>(retries: u8, mut op: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<u32>>,
{
    for attempt in 0..=retries {
        if op().await.is_some() {
            return true;
        }
        if attempt < retries {
            tokio::time::sleep(std::time::Duration::from_millis(150 * (attempt as u64 + 1))).await;
        }
    }
    false
}

#[derive(Debug, Clone, Copy)]
struct ProviderAttemptResult {
    provider: &'static str,
    attempted: bool,
    success: bool,
    duration_ms: u64,
}

#[derive(Debug)]
struct GeneProviderRefreshResult {
    gene_index: usize,
    provider_results: [ProviderAttemptResult; 6],
}

impl ProviderAttemptResult {
    fn skipped(provider: &'static str, started: Instant) -> Self {
        Self {
            provider,
            attempted: false,
            success: false,
            duration_ms: started.elapsed().as_millis() as u64,
        }
    }

    fn attempted(provider: &'static str, success: bool, started: Instant) -> Self {
        Self {
            provider,
            attempted: true,
            success,
            duration_ms: started.elapsed().as_millis() as u64,
        }
    }
}

async fn run_refresh_f64_attempt<F, Fut>(
    provider: &'static str,
    should_run: bool,
    retries: u8,
    op: F,
) -> ProviderAttemptResult
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<f64>>,
{
    let started = Instant::now();
    if !should_run {
        return ProviderAttemptResult::skipped(provider, started);
    }
    let success = retry_fetch_f64(retries, op).await;
    ProviderAttemptResult::attempted(provider, success, started)
}

async fn run_refresh_u32_attempt<F, Fut>(
    provider: &'static str,
    should_run: bool,
    retries: u8,
    op: F,
) -> ProviderAttemptResult
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<u32>>,
{
    let started = Instant::now();
    if !should_run {
        return ProviderAttemptResult::skipped(provider, started);
    }
    let success = retry_fetch_u32(retries, op).await;
    ProviderAttemptResult::attempted(provider, success, started)
}

async fn refresh_provider_signals_for_gene(
    db: Arc<Database>,
    gene_index: usize,
    gene: String,
    cancer_code: Option<String>,
    cbio_policy: ProviderRefreshPolicy,
    cosmic_policy: ProviderRefreshPolicy,
    gtex_policy: ProviderRefreshPolicy,
    tcga_policy: ProviderRefreshPolicy,
    chembl_policy: ProviderRefreshPolicy,
    reactome_policy: ProviderRefreshPolicy,
    force_refresh: bool,
    retries: u8,
) -> GeneProviderRefreshResult {
    let signal_repo = Phase4SignalRepository::new(db);
    let cbio_cancer = cancer_code.clone();
    let cosmic_cancer = cancer_code.clone();
    let tcga_cancer = cancer_code;

    let cbio = async {
        run_refresh_f64_attempt(
            "cbioportal",
            cbio_policy.run_now && cbio_cancer.is_some(),
            retries,
            || async {
                let cc = cbio_cancer
                    .as_deref()
                    .expect("cbio cancer code checked before fetch");
                get_cached_cbio_mutation_frequency(&signal_repo, &gene, cc, force_refresh).await
            },
        )
        .await
    };

    let cosmic = async {
        run_refresh_f64_attempt("cosmic", cosmic_policy.run_now, retries, || async {
            if let Some(cc) = cosmic_cancer.as_deref() {
                get_cached_cosmic_mutation_frequency(&signal_repo, &gene, cc, force_refresh).await
            } else {
                get_cached_cosmic_mutation_frequency_any_cancer(&signal_repo, &gene, force_refresh)
                    .await
            }
        })
        .await
    };

    let gtex = async {
        run_refresh_f64_attempt("gtex", gtex_policy.run_now, retries, || async {
            get_cached_gtex_expression_score(&signal_repo, &gene, force_refresh).await
        })
        .await
    };

    let tcga = async {
        run_refresh_f64_attempt(
            "tcga",
            tcga_policy.run_now && tcga_cancer.is_some(),
            retries,
            || async {
                let cc = tcga_cancer
                    .as_deref()
                    .expect("tcga cancer code checked before fetch");
                get_cached_tcga_survival_score(&signal_repo, &gene, cc, force_refresh).await
            },
        )
        .await
    };

    let chembl = async {
        run_refresh_u32_attempt("chembl", chembl_policy.run_now, retries, || async {
            get_cached_chembl_inhibitor_count(&signal_repo, &gene, force_refresh).await
        })
        .await
    };

    let reactome = async {
        run_refresh_u32_attempt("reactome", reactome_policy.run_now, retries, || async {
            get_cached_reactome_pathway_count(&signal_repo, &gene, force_refresh).await
        })
        .await
    };

    let (cbio, cosmic, gtex, tcga, chembl, reactome) =
        tokio::join!(cbio, cosmic, gtex, tcga, chembl, reactome);

    GeneProviderRefreshResult {
        gene_index,
        provider_results: [cbio, cosmic, gtex, tcga, chembl, reactome],
    }
}

fn apply_gene_provider_refresh_result(
    report: &mut ProviderRefreshReport,
    gene_result: GeneProviderRefreshResult,
) {
    report.genes_processed += 1;
    for provider_result in gene_result.provider_results {
        *report
            .provider_timing_ms
            .entry(provider_result.provider.to_string())
            .or_insert(0) += provider_result.duration_ms;

        match provider_result.provider {
            "cbioportal" => apply_provider_attempt(
                provider_result,
                &mut report.cbio_attempted,
                &mut report.cbio_success,
                &mut report.cbio_failed,
                &mut report.cbio_skipped,
            ),
            "cosmic" => apply_provider_attempt(
                provider_result,
                &mut report.cosmic_attempted,
                &mut report.cosmic_success,
                &mut report.cosmic_failed,
                &mut report.cosmic_skipped,
            ),
            "gtex" => apply_provider_attempt(
                provider_result,
                &mut report.gtex_attempted,
                &mut report.gtex_success,
                &mut report.gtex_failed,
                &mut report.gtex_skipped,
            ),
            "tcga" => apply_provider_attempt(
                provider_result,
                &mut report.tcga_attempted,
                &mut report.tcga_success,
                &mut report.tcga_failed,
                &mut report.tcga_skipped,
            ),
            "chembl" => apply_provider_attempt(
                provider_result,
                &mut report.chembl_attempted,
                &mut report.chembl_success,
                &mut report.chembl_failed,
                &mut report.chembl_skipped,
            ),
            "reactome" => apply_provider_attempt(
                provider_result,
                &mut report.reactome_attempted,
                &mut report.reactome_success,
                &mut report.reactome_failed,
                &mut report.reactome_skipped,
            ),
            _ => {}
        }
    }
}

fn apply_provider_attempt(
    result: ProviderAttemptResult,
    attempted: &mut usize,
    success: &mut usize,
    failed: &mut usize,
    skipped: &mut usize,
) {
    if result.attempted {
        *attempted += 1;
        if result.success {
            *success += 1;
        } else {
            *failed += 1;
        }
    } else {
        *skipped += 1;
    }
}

async fn get_cached_cbio_mutation_frequency(
    signal_repo: &Phase4SignalRepository,
    gene_symbol: &str,
    cancer_code: &str,
    allow_live_fetch: bool,
) -> Option<f64> {
    static CBIO_CACHE: OnceLock<InProcessTtlCache<Option<f64>>> = OnceLock::new();
    let gene = gene_symbol.trim().to_uppercase();
    let cancer = normalize_provider_cancer_code(cancer_code)?;
    let key = format!("{gene}|{cancer}");
    let cache = CBIO_CACHE.get_or_init(provider_f64_cache);
    if let Some(v) = cache.get_cloned(&key) {
        return v;
    }

    if let Ok(Some(row)) = signal_repo
        .find_cbio_mutation_frequency_fresh(&gene, &cancer, PROVIDER_SIGNAL_TTL_DAYS)
        .await
    {
        let value = Some(row.mutation_frequency.clamp(0.0, 1.0));
        cache.insert(key, value);
        return value;
    }

    let fetched = if allow_live_fetch {
        let gene_for_fetch = gene.clone();
        let cancer_for_fetch = cancer.clone();
        tokio::time::timeout(std::time::Duration::from_secs(10), async move {
            let client = CbioPortalClient::new();
            let row = client
                .get_mutation_frequency(&gene_for_fetch, &cancer_for_fetch)
                .await
                .ok()
                .flatten()?;
            Some(row)
        })
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    if let Some(row) = fetched {
        let value = row.mutation_frequency.clamp(0.0, 1.0);
        let _ = signal_repo
            .upsert_cbio_mutation_frequency(&EntCbioMutationFrequency {
                id: uuid::Uuid::new_v4(),
                gene_symbol: gene.clone(),
                cancer_code: cancer.clone(),
                study_id: row.study_id,
                molecular_profile_id: row.molecular_profile_id,
                sample_list_id: row.sample_list_id,
                mutated_sample_count: row.mutated_sample_count as i64,
                profiled_sample_count: row.profiled_sample_count as i64,
                mutation_frequency: value,
                source: "cbioportal_api".to_string(),
                fetched_at: chrono::Utc::now(),
            })
            .await;
        cache.insert(key, Some(value));
        return Some(value);
    }

    let resolved = signal_repo
        .find_cbio_mutation_frequency(&gene, &cancer)
        .await
        .ok()
        .flatten()
        .map(|row| row.mutation_frequency.clamp(0.0, 1.0));

    cache.insert(key, resolved);
    resolved
}

async fn get_cached_cbio_mutation_frequency_any_cancer(
    signal_repo: &Phase4SignalRepository,
    gene_symbol: &str,
) -> Option<f64> {
    static CBIO_ANY_CACHE: OnceLock<InProcessTtlCache<Option<f64>>> = OnceLock::new();
    let gene = gene_symbol.trim().to_uppercase();
    if gene.is_empty() {
        return None;
    }
    let cache = CBIO_ANY_CACHE.get_or_init(provider_f64_cache);
    if let Some(v) = cache.get_cloned(&gene) {
        return v;
    }

    let resolved = signal_repo
        .find_cbio_mutation_frequency_any_cancer(&gene)
        .await
        .ok()
        .flatten()
        .map(|row| row.mutation_frequency.clamp(0.0, 1.0));

    cache.insert(gene, resolved);
    resolved
}

async fn get_cached_cosmic_mutation_frequency(
    signal_repo: &Phase4SignalRepository,
    gene_symbol: &str,
    cancer_code: &str,
    allow_live_fetch: bool,
) -> Option<f64> {
    static COSMIC_CACHE: OnceLock<InProcessTtlCache<Option<f64>>> = OnceLock::new();
    let gene = gene_symbol.trim().to_uppercase();
    let cancer = normalize_provider_cancer_code(cancer_code)
        .unwrap_or_else(|| cancer_code.trim().to_uppercase());
    if gene.is_empty() || cancer.is_empty() {
        return None;
    }
    let key = format!("{gene}|{cancer}");
    let cache = COSMIC_CACHE.get_or_init(provider_f64_cache);
    if let Some(v) = cache.get_cloned(&key) {
        return v;
    }

    if let Ok(Some(row)) = signal_repo
        .find_cosmic_mutation_frequency_fresh(&gene, &cancer, PROVIDER_SIGNAL_TTL_DAYS)
        .await
    {
        let value = Some(row.mutation_frequency.clamp(0.0, 1.0));
        cache.insert(key, value);
        return value;
    }

    let fetched = if allow_live_fetch {
        let gene_for_fetch = gene.clone();
        let cancer_for_fetch = cancer.clone();
        tokio::time::timeout(std::time::Duration::from_secs(10), async move {
            let client = CosmicClient::new();
            client
                .get_mutation_frequency(&gene_for_fetch, &cancer_for_fetch)
                .await
                .ok()
                .flatten()
        })
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    if let Some(row) = fetched {
        let value = row.mutation_frequency.clamp(0.0, 1.0);
        let _ = signal_repo
            .upsert_cosmic_mutation_frequency(&EntCosmicMutationFrequency {
                id: uuid::Uuid::new_v4(),
                gene_symbol: gene.clone(),
                cancer_code: cancer.clone(),
                mutated_sample_count: row.mutated_sample_count as i64,
                profiled_sample_count: row.profiled_sample_count as i64,
                mutation_frequency: value,
                source: row.source,
                fetched_at: chrono::Utc::now(),
            })
            .await;
        cache.insert(key, Some(value));
        return Some(value);
    }

    let resolved = signal_repo
        .find_cosmic_mutation_frequency(&gene, &cancer)
        .await
        .ok()
        .flatten()
        .map(|row| row.mutation_frequency.clamp(0.0, 1.0));
    cache.insert(key, resolved);
    resolved
}

async fn get_cached_cosmic_mutation_frequency_any_cancer(
    signal_repo: &Phase4SignalRepository,
    gene_symbol: &str,
    allow_live_fetch: bool,
) -> Option<f64> {
    static COSMIC_ANY_CACHE: OnceLock<InProcessTtlCache<Option<f64>>> = OnceLock::new();
    let gene = gene_symbol.trim().to_uppercase();
    if gene.is_empty() {
        return None;
    }
    let cache = COSMIC_ANY_CACHE.get_or_init(provider_f64_cache);
    if let Some(v) = cache.get_cloned(&gene) {
        return v;
    }

    if let Ok(Some(row)) = signal_repo
        .find_cosmic_mutation_frequency_any_cancer(&gene)
        .await
    {
        let value = Some(row.mutation_frequency.clamp(0.0, 1.0));
        cache.insert(gene.clone(), value);
        return value;
    }

    let fetched = if allow_live_fetch {
        let gene_for_fetch = gene.clone();
        tokio::time::timeout(std::time::Duration::from_secs(10), async move {
            let client = CosmicClient::new();
            client
                .get_mutation_frequency_any_cancer(&gene_for_fetch)
                .await
                .ok()
                .flatten()
        })
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    if let Some(row) = fetched {
        let value = row.mutation_frequency.clamp(0.0, 1.0);
        let _ = signal_repo
            .upsert_cosmic_mutation_frequency(&EntCosmicMutationFrequency {
                id: uuid::Uuid::new_v4(),
                gene_symbol: gene.clone(),
                cancer_code: row.cancer_code,
                mutated_sample_count: row.mutated_sample_count as i64,
                profiled_sample_count: row.profiled_sample_count as i64,
                mutation_frequency: value,
                source: row.source,
                fetched_at: chrono::Utc::now(),
            })
            .await;
        cache.insert(gene, Some(value));
        return Some(value);
    }

    cache.insert(gene, None);
    None
}

async fn get_cached_gtex_expression_score(
    signal_repo: &Phase4SignalRepository,
    gene_symbol: &str,
    allow_live_fetch: bool,
) -> Option<f64> {
    static GTEX_CACHE: OnceLock<InProcessTtlCache<Option<f64>>> = OnceLock::new();
    let key = gene_symbol.trim().to_uppercase();
    if key.is_empty() {
        return None;
    }

    let cache = GTEX_CACHE.get_or_init(provider_f64_cache);
    if let Some(v) = cache.get_cloned(&key) {
        return v;
    }

    if let Ok(Some(row)) = signal_repo
        .find_gtex_expression_fresh(&key, PROVIDER_SIGNAL_TTL_DAYS)
        .await
    {
        let value = Some(row.expression_score.clamp(0.0, 1.0));
        cache.insert(key, value);
        return value;
    }

    // Bounded network call with timeout so ranker latency stays controlled.
    let fetched = if allow_live_fetch {
        tokio::time::timeout(std::time::Duration::from_secs(8), async {
            let client = GtexClient::new();
            let map = client.get_median_expression(&key).await.ok()?;
            if map.is_empty() {
                return None;
            }
            let baseline = map.values().copied().sum::<f64>() / map.len() as f64;
            // Lower normal baseline expression => higher therapeutic window proxy.
            Some((1.0 / (1.0 + baseline.ln_1p())).clamp(0.0, 1.0))
        })
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    if let Some(expression_score) = fetched {
        let _ = signal_repo
            .upsert_gtex_expression(&EntGtexExpression {
                id: uuid::Uuid::new_v4(),
                gene_symbol: key.clone(),
                expression_score,
                source: "gtex_api".to_string(),
                fetched_at: chrono::Utc::now(),
            })
            .await;
    }

    let resolved = if fetched.is_some() {
        fetched
    } else {
        signal_repo
            .find_gtex_expression(&key)
            .await
            .ok()
            .flatten()
            .map(|row| row.expression_score.clamp(0.0, 1.0))
    };

    cache.insert(key, resolved);
    resolved
}

fn to_tcga_project_id(cancer_code: &str) -> Option<String> {
    let code = cancer_code.trim().to_uppercase();
    if code.is_empty() {
        return None;
    }
    let mapped = match code.as_str() {
        "NSCLC" => "LUAD",
        "SKCM" | "PAAD" | "LUAD" | "LUSC" | "BRCA" | "COAD" | "READ" | "GBM" | "HNSC" | "OV"
        | "KIRC" | "KIRP" | "THCA" | "STAD" | "BLCA" | "UCEC" | "LIHC" | "PRAD" => code.as_str(),
        other if other.len() == 4 && other.chars().all(|c| c.is_ascii_uppercase()) => other,
        _ => return None,
    };
    Some(format!("TCGA-{}", mapped))
}

async fn get_cached_tcga_survival_score(
    signal_repo: &Phase4SignalRepository,
    gene_symbol: &str,
    cancer_code: &str,
    allow_live_fetch: bool,
) -> Option<f64> {
    static TCGA_CACHE: OnceLock<InProcessTtlCache<Option<f64>>> = OnceLock::new();
    let gene = gene_symbol.trim().to_uppercase();
    let project = to_tcga_project_id(cancer_code)?;
    let normalized_cancer = cancer_code.trim().to_uppercase();
    let key = format!("{}|{}", gene, project);

    let cache = TCGA_CACHE.get_or_init(provider_f64_cache);
    if let Some(v) = cache.get_cloned(&key) {
        return v;
    }

    if let Ok(Some(row)) = signal_repo
        .find_tcga_survival_fresh(&gene, &normalized_cancer, PROVIDER_SIGNAL_TTL_DAYS)
        .await
    {
        let value = Some(row.survival_score.clamp(0.0, 1.0));
        cache.insert(key, value);
        return value;
    }

    let fetched = if allow_live_fetch {
        tokio::time::timeout(std::time::Duration::from_secs(8), async {
            let client = TcgaClient::new();
            let corr = client
                .get_survival_correlation(&gene, &project)
                .await
                .ok()
                .flatten()?;
            Some(((corr + 1.0) / 2.0).clamp(0.0, 1.0))
        })
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    if let Some(survival_score) = fetched {
        let _ = signal_repo
            .upsert_tcga_survival(&EntTcgaSurvival {
                id: uuid::Uuid::new_v4(),
                gene_symbol: gene.clone(),
                cancer_code: normalized_cancer.clone(),
                tcga_project_id: project.clone(),
                survival_score,
                source: "tcga_api".to_string(),
                fetched_at: chrono::Utc::now(),
            })
            .await;
    }

    let resolved = if fetched.is_some() {
        fetched
    } else {
        signal_repo
            .find_tcga_survival(&gene, &normalized_cancer)
            .await
            .ok()
            .flatten()
            .map(|row| row.survival_score.clamp(0.0, 1.0))
    };

    cache.insert(key, resolved);
    resolved
}

#[derive(Debug, Deserialize)]
struct ReactomeProjectionResponse {
    #[serde(rename = "pathwaysFound")]
    pathways_found: Option<u64>,
    pathways: Option<Vec<serde_json::Value>>,
}

async fn get_cached_chembl_inhibitor_count(
    signal_repo: &Phase4SignalRepository,
    gene_symbol: &str,
    allow_live_fetch: bool,
) -> Option<u32> {
    static CHEMBL_CACHE: OnceLock<InProcessTtlCache<Option<u32>>> = OnceLock::new();
    let key = gene_symbol.trim().to_uppercase();
    if key.is_empty() {
        return None;
    }
    let cache = CHEMBL_CACHE.get_or_init(provider_u32_cache);
    if let Some(v) = cache.get_cloned(&key) {
        return v;
    }

    if let Ok(Some(row)) = signal_repo
        .find_chembl_target_fresh(&key, PROVIDER_SIGNAL_TTL_DAYS)
        .await
    {
        let value = Some(row.inhibitor_count.max(0) as u32);
        cache.insert(key, value);
        return value;
    }

    let fetched = if allow_live_fetch {
        let key_for_fetch = key.clone();
        tokio::time::timeout(std::time::Duration::from_secs(8), async move {
            let client = ChemblClient::new();
            let targets = client.search_targets_by_gene(&key_for_fetch).await.ok()?;
            if targets.is_empty() {
                return Some(0u32);
            }

            let mut unique_compounds: HashSet<String> = HashSet::new();
            for target in targets.iter().take(3) {
                let acts = client
                    .fetch_target_activities(&target.chembl_id, None, 250)
                    .await
                    .ok()?;
                for act in acts {
                    if !act.compound_id.trim().is_empty() {
                        unique_compounds.insert(act.compound_id);
                    }
                }
                if unique_compounds.len() >= 1000 {
                    break;
                }
            }
            Some(unique_compounds.len() as u32)
        })
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    if let Some(inhibitor_count) = fetched {
        let _ = signal_repo
            .upsert_chembl_target(&EntChemblTarget {
                id: uuid::Uuid::new_v4(),
                gene_symbol: key.clone(),
                inhibitor_count: inhibitor_count as i64,
                source: "chembl_api".to_string(),
                fetched_at: chrono::Utc::now(),
            })
            .await;
    }

    let resolved = if fetched.is_some() {
        fetched
    } else {
        signal_repo
            .find_chembl_target(&key)
            .await
            .ok()
            .flatten()
            .map(|row| row.inhibitor_count.max(0) as u32)
    };

    cache.insert(key, resolved);
    resolved
}

async fn get_cached_reactome_pathway_count(
    signal_repo: &Phase4SignalRepository,
    gene_symbol: &str,
    allow_live_fetch: bool,
) -> Option<u32> {
    static REACTOME_CACHE: OnceLock<InProcessTtlCache<Option<u32>>> = OnceLock::new();
    let key = gene_symbol.trim().to_uppercase();
    if key.is_empty() {
        return None;
    }
    let cache = REACTOME_CACHE.get_or_init(provider_u32_cache);
    if let Some(v) = cache.get_cloned(&key) {
        return v;
    }

    if let Ok(Some(row)) = signal_repo
        .find_reactome_gene_fresh(&key, PROVIDER_SIGNAL_TTL_DAYS)
        .await
    {
        let value = Some(row.pathway_count.max(0) as u32);
        cache.insert(key, value);
        return value;
    }

    let fetched = if allow_live_fetch {
        let key_for_fetch = key.clone();
        tokio::time::timeout(std::time::Duration::from_secs(8), async move {
            let client = Client::new();
            let resp = client
                .post(
                    "https://reactome.org/AnalysisService/identifiers/projection?pageSize=200&page=1",
                )
                .header("Content-Type", "text/plain")
                .body(key_for_fetch)
                .send()
                .await
                .ok()?;
            if !resp.status().is_success() {
                return None;
            }
            let body = resp.json::<ReactomeProjectionResponse>().await.ok()?;
            if let Some(pathways) = body.pathways {
                return Some(pathways.len() as u32);
            }
            body.pathways_found.map(|v| v as u32)
        })
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    if let Some(pathway_count) = fetched {
        let _ = signal_repo
            .upsert_reactome_gene(&EntReactomeGene {
                id: uuid::Uuid::new_v4(),
                gene_symbol: key.clone(),
                pathway_count: pathway_count as i64,
                source: "reactome_api".to_string(),
                fetched_at: chrono::Utc::now(),
            })
            .await;
    }

    let resolved = if fetched.is_some() {
        fetched
    } else {
        signal_repo
            .find_reactome_gene(&key)
            .await
            .ok()
            .flatten()
            .map(|row| row.pathway_count.max(0) as u32)
    };

    cache.insert(key, resolved);
    resolved
}

fn source_backed_default_metrics() -> TargetMetrics {
    TargetMetrics {
        mutation_freq: 0.0,
        crispr_dependency: -1.0,
        survival_correlation: 0.5,
        expression_specificity: 1.5,
        pdb_structure_count: 0,
        af_plddt_mean: 50.0,
        fpocket_best_score: 0.0,
        chembl_inhibitor_count: 1,
        reactome_escape_pathway_count: 1,
        literature_novelty_velocity: 0.5,
    }
}

fn default_component_sources(source_backed_only: bool) -> BTreeMap<String, String> {
    let default_source = if source_backed_only {
        "source_missing".to_string()
    } else {
        "proxy_kg".to_string()
    };
    let mut out = BTreeMap::new();
    out.insert("n1_mutation_freq".to_string(), default_source.clone());
    out.insert("n2_crispr_dependency".to_string(), default_source.clone());
    out.insert(
        "n3_survival_correlation".to_string(),
        default_source.clone(),
    );
    out.insert(
        "n4_expression_specificity".to_string(),
        default_source.clone(),
    );
    out.insert(
        "n5_structural_tractability".to_string(),
        default_source.clone(),
    );
    out.insert(
        "n6_pocket_detectability".to_string(),
        default_source.clone(),
    );
    out.insert("n7_novelty_score".to_string(), default_source.clone());
    out.insert(
        "n8_pathway_independence".to_string(),
        default_source.clone(),
    );
    out.insert("n9_literature_novelty".to_string(), default_source);
    out
}

#[derive(Debug, Clone)]
struct GeneCandidate {
    gene_symbol: String,
    flags: HashSet<String>,
    paper_ids: HashSet<uuid::Uuid>,
    fact_count: u32,
    weighted_evidence_sum: f64,
    confidence_sum: f64,
    confidence_n: u32,
    weighted_confidence_sum: f64,
    weighted_confidence_weight: f64,
    provider_fact_count: u32,
    high_tier_fact_count: u32,
    mutation_mentions: u32,
    mutation_mentions_w: f64,
    survival_mentions: u32,
    survival_mentions_w: f64,
    survival_positive: u32,
    survival_positive_w: f64,
    survival_negative: u32,
    survival_negative_w: f64,
    expression_tumor_mentions: u32,
    expression_tumor_mentions_w: f64,
    expression_normal_mentions: u32,
    expression_normal_mentions_w: f64,
    cancer_mentions: u32,
    cancer_mentions_w: f64,
    inhibitor_mentions: u32,
    inhibitor_mentions_w: f64,
    pathway_mentions: u32,
    pathway_mentions_w: f64,
    structural_mentions: u32,
    structural_mentions_w: f64,
    pocket_mentions: u32,
    pocket_mentions_w: f64,
    sample_size_sum: f64,
    sample_obs: u32,
    cancer_codes: HashMap<String, u32>,
}

impl GeneCandidate {
    fn new(gene_symbol: String) -> Self {
        Self {
            gene_symbol,
            flags: HashSet::new(),
            paper_ids: HashSet::new(),
            fact_count: 0,
            weighted_evidence_sum: 0.0,
            confidence_sum: 0.0,
            confidence_n: 0,
            weighted_confidence_sum: 0.0,
            weighted_confidence_weight: 0.0,
            provider_fact_count: 0,
            high_tier_fact_count: 0,
            mutation_mentions: 0,
            mutation_mentions_w: 0.0,
            survival_mentions: 0,
            survival_mentions_w: 0.0,
            survival_positive: 0,
            survival_positive_w: 0.0,
            survival_negative: 0,
            survival_negative_w: 0.0,
            expression_tumor_mentions: 0,
            expression_tumor_mentions_w: 0.0,
            expression_normal_mentions: 0,
            expression_normal_mentions_w: 0.0,
            cancer_mentions: 0,
            cancer_mentions_w: 0.0,
            inhibitor_mentions: 0,
            inhibitor_mentions_w: 0.0,
            pathway_mentions: 0,
            pathway_mentions_w: 0.0,
            structural_mentions: 0,
            structural_mentions_w: 0.0,
            pocket_mentions: 0,
            pocket_mentions_w: 0.0,
            sample_size_sum: 0.0,
            sample_obs: 0,
            cancer_codes: HashMap::new(),
        }
    }

    fn mean_confidence(&self) -> f64 {
        if self.weighted_confidence_weight > 0.0 {
            let weighted =
                (self.weighted_confidence_sum / self.weighted_confidence_weight).clamp(0.0, 1.0);
            let provider_bonus = if self.provider_fact_count > 0 {
                0.02
            } else {
                0.0
            };
            let tier_bonus = if self.high_tier_fact_count > 0 {
                0.02
            } else {
                0.0
            };
            return (weighted + provider_bonus + tier_bonus).clamp(0.0, 1.0);
        }
        if self.confidence_n == 0 {
            return 0.5;
        }
        (self.confidence_sum / self.confidence_n as f64).clamp(0.0, 1.0)
    }

    fn infer_cancer_code(&self) -> Option<String> {
        self.cancer_codes
            .iter()
            .max_by_key(|(_, cnt)| **cnt)
            .map(|(code, _)| code.clone())
    }

    fn semantic_survival_score(&self) -> Option<f64> {
        let mentions = if self.survival_mentions_w > 0.0 {
            self.survival_mentions_w
        } else {
            self.survival_mentions as f64
        };
        if mentions <= 0.0 {
            return None;
        }
        let positive = if self.survival_positive_w > 0.0 {
            self.survival_positive_w
        } else {
            self.survival_positive as f64
        };
        let negative = if self.survival_negative_w > 0.0 {
            self.survival_negative_w
        } else {
            self.survival_negative as f64
        };
        let signed = (positive - negative) / mentions;
        Some(((signed + 1.0) / 2.0).clamp(0.0, 1.0))
    }

    fn semantic_expression_specificity(&self) -> Option<f64> {
        let tumor = if self.expression_tumor_mentions_w > 0.0 {
            self.expression_tumor_mentions_w
        } else {
            self.expression_tumor_mentions as f64
        };
        let normal = if self.expression_normal_mentions_w > 0.0 {
            self.expression_normal_mentions_w
        } else {
            self.expression_normal_mentions as f64
        };
        if tumor + normal <= 0.0 {
            return None;
        }
        Some(((tumor + 1.0) / (normal + 1.0)).clamp(0.5, 5.0))
    }

    fn to_target_metrics(&self) -> TargetMetrics {
        let evidence = if self.weighted_evidence_sum > 0.0 {
            self.weighted_evidence_sum.max(1.0)
        } else {
            self.fact_count.max(1) as f64
        };
        let confidence_mean = self.mean_confidence();

        let mutation_mentions = if self.mutation_mentions_w > 0.0 {
            self.mutation_mentions_w
        } else {
            self.mutation_mentions as f64
        };
        let mutation_freq = (mutation_mentions / evidence).clamp(0.0, 1.0);
        let crispr_dependency = (-2.0 * confidence_mean).clamp(-2.0, 0.0);

        // Source-derived from survival semantics when present; confidence/sample fallback otherwise.
        let survival_correlation = self
            .semantic_survival_score()
            .or_else(|| {
                if self.sample_obs > 0 {
                    Some(
                        (self.sample_size_sum / self.sample_obs as f64)
                            .ln_1p()
                            .min(10.0)
                            / 10.0,
                    )
                } else {
                    None
                }
            })
            .unwrap_or(confidence_mean);

        // Source-derived from expression predicates (tumor/normal ratio) when present.
        let expression_specificity = self.semantic_expression_specificity().unwrap_or_else(|| {
            let cancer_mentions = if self.cancer_mentions_w > 0.0 {
                self.cancer_mentions_w
            } else {
                self.cancer_mentions as f64
            };
            (1.0 + 4.0 * (cancer_mentions / evidence)).clamp(0.5, 5.0)
        });

        let structural_mentions = if self.structural_mentions_w > 0.0 {
            self.structural_mentions_w
        } else {
            self.structural_mentions as f64
        };
        let pocket_mentions = if self.pocket_mentions_w > 0.0 {
            self.pocket_mentions_w
        } else {
            self.pocket_mentions as f64
        };
        let inhibitor_mentions = if self.inhibitor_mentions_w > 0.0 {
            self.inhibitor_mentions_w
        } else {
            self.inhibitor_mentions as f64
        };
        let pathway_mentions = if self.pathway_mentions_w > 0.0 {
            self.pathway_mentions_w
        } else {
            self.pathway_mentions as f64
        };

        let pdb_structure_count = structural_mentions.round().clamp(0.0, u32::MAX as f64) as u32;
        let af_plddt_mean = if pdb_structure_count > 0 {
            (70.0 + 25.0 * confidence_mean).clamp(0.0, 100.0)
        } else {
            (45.0 + 20.0 * confidence_mean).clamp(0.0, 100.0)
        };

        let fpocket_best_score = (0.2 + 0.8 * (pocket_mentions / evidence)).clamp(0.0, 1.0);

        let chembl_inhibitor_count = inhibitor_mentions.round().clamp(0.0, u32::MAX as f64) as u32;
        let reactome_escape_pathway_count =
            pathway_mentions.round().clamp(0.0, u32::MAX as f64) as u32;

        // Inverted evidence velocity proxy: underexplored genes score higher.
        let literature_novelty_velocity = 1.0 / (1.0 + evidence.ln_1p());

        TargetMetrics {
            mutation_freq,
            crispr_dependency,
            survival_correlation,
            expression_specificity,
            pdb_structure_count,
            af_plddt_mean,
            fpocket_best_score,
            chembl_inhibitor_count,
            reactome_escape_pathway_count,
            literature_novelty_velocity,
        }
    }

    fn source_backed_literature_novelty(
        &self,
        signals_by_paper: &HashMap<uuid::Uuid, PaperNoveltySignal>,
    ) -> Option<(f64, bool)> {
        if self.paper_ids.is_empty() {
            return None;
        }

        let now = chrono::Utc::now();
        let mut total = 0usize;
        let mut sum = 0.0f64;
        let mut used_citations = false;
        for paper_id in &self.paper_ids {
            let Some(signal) = signals_by_paper.get(paper_id) else {
                continue;
            };
            let recency = signal
                .published_at
                .map(|ts| {
                    let age_days = (now - ts).num_days().max(0) as f64;
                    (1.0 / (1.0 + age_days / 365.0)).clamp(0.0, 1.0)
                })
                .unwrap_or(0.5);
            let citation_novelty = signal
                .citation_count
                .map(|c| {
                    used_citations = true;
                    (1.0 / (1.0 + (c as f64).ln_1p())).clamp(0.0, 1.0)
                })
                .unwrap_or(recency);
            let novelty = (0.55 * citation_novelty + 0.45 * recency).clamp(0.0, 1.0);
            total += 1;
            sum += novelty;
        }

        if total == 0 {
            return None;
        }

        let score = (sum / total as f64).clamp(0.0, 1.0);
        Some((score, used_citations))
    }
}

#[derive(Debug, Clone)]
struct ProviderRefreshPolicy {
    run_now: bool,
    interval_secs: u64,
    reason: String,
}

impl ProviderRefreshPolicy {
    fn run(interval_secs: u64, reason: impl Into<String>) -> Self {
        Self {
            run_now: true,
            interval_secs,
            reason: reason.into(),
        }
    }

    fn skip(interval_secs: u64, reason: impl Into<String>) -> Self {
        Self {
            run_now: false,
            interval_secs,
            reason: reason.into(),
        }
    }
}

async fn provider_refresh_policy(
    signal_repo: &Phase4SignalRepository,
    provider: &str,
) -> ProviderRefreshPolicy {
    let adaptive_enabled = phase4_provider_refresh_adaptive_enabled();
    if !adaptive_enabled {
        return ProviderRefreshPolicy::run(
            phase4_provider_refresh_base_interval_secs(),
            "adaptive_disabled",
        );
    }

    let base = phase4_provider_refresh_base_interval_secs();
    let min = phase4_provider_refresh_min_interval_secs().min(base.max(1));
    let max = phase4_provider_refresh_max_interval_secs().max(base);
    let recent_limit = phase4_provider_refresh_recent_runs();
    let high = phase4_provider_refresh_high_error_threshold();
    let severe = phase4_provider_refresh_severe_error_threshold();
    let backoff = phase4_provider_refresh_backoff_factor();
    let accelerate_div = phase4_provider_refresh_success_accelerate_divisor();

    let recent = signal_repo
        .list_provider_refresh_runs(provider, recent_limit)
        .await
        .unwrap_or_default();
    if recent.is_empty() {
        return ProviderRefreshPolicy::run(base, "cold_start");
    }

    let attempts: i64 = recent.iter().map(|r| r.attempted.max(0)).sum();
    let failed: i64 = recent.iter().map(|r| r.failed.max(0)).sum();
    let success: i64 = recent.iter().map(|r| r.success.max(0)).sum();
    let error_rate = if attempts > 0 {
        failed as f64 / attempts as f64
    } else if success > 0 {
        0.0
    } else {
        1.0
    };

    let interval = if error_rate >= severe {
        (base.saturating_mul(backoff.saturating_mul(2))).clamp(min, max)
    } else if error_rate >= high {
        (base.saturating_mul(backoff)).clamp(min, max)
    } else if error_rate <= 0.10 {
        (base / accelerate_div.max(1)).clamp(min, max)
    } else {
        base.clamp(min, max)
    };

    let last = recent
        .iter()
        .max_by_key(|r| r.finished_at)
        .map(|r| r.finished_at)
        .unwrap_or_else(chrono::Utc::now);
    let elapsed = (chrono::Utc::now() - last).num_seconds().max(0) as u64;
    let stale_force_after = phase4_provider_refresh_stale_force_after_secs();

    if elapsed >= stale_force_after {
        return ProviderRefreshPolicy::run(
            interval,
            format!(
                "stale_force elapsed={}s stale_after={}s err={:.2}",
                elapsed, stale_force_after, error_rate
            ),
        );
    }

    if elapsed >= interval {
        ProviderRefreshPolicy::run(
            interval,
            format!(
                "due elapsed={}s interval={}s err={:.2}",
                elapsed, interval, error_rate
            ),
        )
    } else {
        ProviderRefreshPolicy::skip(
            interval,
            format!(
                "deferred elapsed={}s interval={}s err={:.2}",
                elapsed, interval, error_rate
            ),
        )
    }
}

fn phase4_provider_refresh_adaptive_enabled() -> bool {
    !std::env::var("FERRUMYX_PHASE4_PROVIDER_REFRESH_ADAPTIVE_ENABLED")
        .ok()
        .is_some_and(|v| v == "0" || v.eq_ignore_ascii_case("false"))
}

fn phase4_provider_refresh_base_interval_secs() -> u64 {
    std::env::var("FERRUMYX_PHASE4_PROVIDER_REFRESH_BASE_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v.clamp(60, 86_400))
        .unwrap_or(900)
}

fn phase4_provider_refresh_min_interval_secs() -> u64 {
    std::env::var("FERRUMYX_PHASE4_PROVIDER_REFRESH_MIN_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v.clamp(30, 43_200))
        .unwrap_or(180)
}

fn phase4_provider_refresh_max_interval_secs() -> u64 {
    std::env::var("FERRUMYX_PHASE4_PROVIDER_REFRESH_MAX_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v.clamp(300, 604_800))
        .unwrap_or(21_600)
}

fn phase4_provider_refresh_backoff_factor() -> u64 {
    std::env::var("FERRUMYX_PHASE4_PROVIDER_REFRESH_BACKOFF_FACTOR")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v.clamp(1, 8))
        .unwrap_or(2)
}

fn phase4_provider_refresh_success_accelerate_divisor() -> u64 {
    std::env::var("FERRUMYX_PHASE4_PROVIDER_REFRESH_SUCCESS_ACCEL_DIV")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v.clamp(1, 8))
        .unwrap_or(2)
}

fn phase4_provider_refresh_recent_runs() -> usize {
    std::env::var("FERRUMYX_PHASE4_PROVIDER_REFRESH_RECENT_RUNS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|v| v.clamp(1, 128))
        .unwrap_or(8)
}

fn phase4_provider_refresh_stale_force_after_secs() -> u64 {
    std::env::var("FERRUMYX_PHASE4_PROVIDER_REFRESH_STALE_FORCE_AFTER_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v.clamp(300, 2_592_000))
        .unwrap_or(86_400)
}

fn phase4_provider_refresh_high_error_threshold() -> f64 {
    std::env::var("FERRUMYX_PHASE4_PROVIDER_REFRESH_HIGH_ERROR_THRESHOLD")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| v.clamp(0.05, 0.95))
        .unwrap_or(0.35)
}

fn phase4_provider_refresh_severe_error_threshold() -> f64 {
    std::env::var("FERRUMYX_PHASE4_PROVIDER_REFRESH_SEVERE_ERROR_THRESHOLD")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| v.clamp(0.05, 0.99))
        .unwrap_or(0.65)
}

async fn persist_provider_refresh_history(
    signal_repo: &Phase4SignalRepository,
    report: &ProviderRefreshReport,
    policies: &HashMap<&'static str, ProviderRefreshPolicy>,
    started_at: chrono::DateTime<chrono::Utc>,
    finished_at: chrono::DateTime<chrono::Utc>,
) {
    let duration_ms = (finished_at - started_at).num_milliseconds().max(0);
    let rows = [
        (
            "cbioportal",
            report.cbio_attempted,
            report.cbio_success,
            report.cbio_failed,
            report.cbio_skipped,
        ),
        (
            "cosmic",
            report.cosmic_attempted,
            report.cosmic_success,
            report.cosmic_failed,
            report.cosmic_skipped,
        ),
        (
            "gtex",
            report.gtex_attempted,
            report.gtex_success,
            report.gtex_failed,
            report.gtex_skipped,
        ),
        (
            "tcga",
            report.tcga_attempted,
            report.tcga_success,
            report.tcga_failed,
            report.tcga_skipped,
        ),
        (
            "chembl",
            report.chembl_attempted,
            report.chembl_success,
            report.chembl_failed,
            report.chembl_skipped,
        ),
        (
            "reactome",
            report.reactome_attempted,
            report.reactome_success,
            report.reactome_failed,
            report.reactome_skipped,
        ),
    ];
    for (provider, attempted, success, failed, skipped) in rows {
        let Some(policy) = policies.get(provider) else {
            continue;
        };
        let attempted_i64 = attempted as i64;
        let failed_i64 = failed as i64;
        let error_rate = if attempted_i64 > 0 {
            failed_i64 as f64 / attempted_i64 as f64
        } else {
            0.0
        };
        let row = EntProviderRefreshRun {
            id: uuid::Uuid::new_v4(),
            provider: provider.to_string(),
            started_at,
            finished_at,
            genes_requested: report.genes_requested as i64,
            genes_processed: report.genes_processed as i64,
            attempted: attempted as i64,
            success: success as i64,
            failed: failed as i64,
            skipped: skipped as i64,
            duration_ms,
            error_rate,
            cadence_interval_secs: policy.interval_secs as i64,
            trigger_reason: policy.reason.clone(),
        };
        let _ = signal_repo.append_provider_refresh_run(&row).await;
    }
}

fn is_gene_like(name: &str) -> bool {
    let n = name.trim();
    if n.is_empty() || n.len() > 16 || n.contains(' ') {
        return false;
    }
    n.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && n.chars().any(|c| c.is_ascii_uppercase())
}

fn is_cancer_like(name: &str) -> bool {
    let n = name.trim();
    if n.is_empty() {
        return false;
    }
    let code_like = n.len() <= 8
        && n.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    if code_like {
        return true;
    }
    let lc = n.to_lowercase();
    lc.contains("cancer")
        || lc.contains("carcinoma")
        || lc.contains("sarcoma")
        || lc.contains("lymphoma")
        || lc.contains("leukemia")
        || lc.contains("tumor")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn mk_temp_dir(name: &str) -> PathBuf {
        let p =
            std::env::temp_dir().join(format!("ferrumyx_ranker_{}_{}", name, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn extract_first_float_parses_numeric_token() {
        let v = extract_first_float("Score : 0.572").unwrap();
        assert!((v - 0.572).abs() < 1e-6);
    }

    #[test]
    fn parse_plddt_mean_from_pdb_reads_bfactor_column() {
        let dir = mk_temp_dir("plddt");
        let path = dir.join("AF-P01116-F1-model_v4.pdb");
        std::fs::write(
            &path,
            "ATOM      1  N   MET A   1      11.104  13.207   6.204  1.00 75.00           N\n\
             ATOM      2  CA  MET A   1      12.400  13.800   6.700  1.00 85.00           C\n",
        )
        .unwrap();
        let mean = parse_plddt_mean_from_pdb(&path).unwrap();
        assert!((mean - 80.0).abs() < 1e-6);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_fpocket_best_score_from_info_file() {
        let dir = mk_temp_dir("fpocket");
        let out_dir = dir.join("AF-P01116-F1-model_v4_out");
        std::fs::create_dir_all(&out_dir).unwrap();
        std::fs::write(
            out_dir.join("AF-P01116-F1-model_v4_info.txt"),
            "Pocket 1\nScore : 0.41\nPocket 2\nScore : 0.63\n",
        )
        .unwrap();
        let best = parse_fpocket_best_score(&out_dir).unwrap();
        assert!((best - 0.63).abs() < 1e-6);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_entries_expire_after_ttl() {
        let mut cache = HashMap::new();
        let started = Instant::now();
        let ttl = Duration::from_secs(5);

        cache_insert(
            &mut cache,
            "TP53".to_string(),
            Some(0.42f64),
            started,
            ttl,
            4,
        );
        assert_eq!(
            cache_get_cloned(&mut cache, "TP53", started + Duration::from_secs(4), ttl),
            Some(Some(0.42))
        );
        assert_eq!(
            cache_get_cloned(&mut cache, "TP53", started + Duration::from_secs(5), ttl),
            None
        );
        assert!(cache.is_empty());
    }

    #[test]
    fn cache_insert_prunes_oldest_entries_when_full() {
        let mut cache = HashMap::new();
        let started = Instant::now();
        let ttl = Duration::from_secs(60);

        cache_insert(&mut cache, "A".to_string(), Some(1u32), started, ttl, 2);
        cache_insert(
            &mut cache,
            "B".to_string(),
            Some(2u32),
            started + Duration::from_secs(1),
            ttl,
            2,
        );
        cache_insert(
            &mut cache,
            "C".to_string(),
            Some(3u32),
            started + Duration::from_secs(2),
            ttl,
            2,
        );

        assert_eq!(cache.len(), 2);
        assert!(cache.contains_key("B"));
        assert!(cache.contains_key("C"));
        assert!(!cache.contains_key("A"));
    }

    #[test]
    fn contains_ascii_case_insensitive_matches_without_allocating_joined_context() {
        assert!(contains_ascii_case_insensitive(
            "Overall survival improved in tumors",
            "IMPROVED"
        ));
        assert!(contains_ascii_case_insensitive(
            "adjacent normal tissue",
            "normal"
        ));
        assert!(!contains_ascii_case_insensitive(
            "tumor selective",
            "healthy"
        ));
    }

    #[test]
    fn fields_contain_any_ascii_keyword_scans_across_multiple_fields() {
        let fields = [
            "KRAS",
            "lung adenocarcinoma",
            "Associated with shorter survival",
        ];
        assert!(fields_contain_any_ascii_keyword(
            &fields,
            &["better", "shorter"]
        ));
        assert!(!fields_contain_any_ascii_keyword(
            &fields,
            &["healthy", "adjacent"]
        ));
    }
}
