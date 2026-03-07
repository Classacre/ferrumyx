use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct QueryRequest {
    pub query_text: String,
    pub cancer_code: Option<String>,
    pub gene_symbol: Option<String>,
    pub mutation: Option<String>,
    pub max_results: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResult {
    pub rank: usize,
    pub gene_symbol: String,
    pub cancer_code: String,
    pub composite_score: f64,
    pub confidence_adj: f64,
    pub shortlist_tier: String,
    pub flags: Vec<String>,
    pub metrics: Option<TargetMetrics>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TargetMetrics {
    pub mutation_freq: f64,
    pub crispr_dependency: f64,
    pub survival_correlation: f64,
    pub expression_specificity: f64,
    pub pdb_structure_count: u32,
    pub af_plddt_mean: f64,
    pub fpocket_best_score: f64,
    pub chembl_inhibitor_count: u32,
    pub reactome_escape_pathway_count: u32,
    pub literature_novelty_velocity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetScoreResult {
    pub n1_mutation_freq: f64,
    pub n2_crispr_dependency: f64,
    pub n3_survival_correlation: f64,
    pub n4_expression_specificity: f64,
    pub n5_structural_tractability: f64,
    pub n6_pocket_detectability: f64,
    pub n7_novelty_score: f64,
    pub n8_pathway_independence: f64,
    pub n9_literature_novelty: f64,
    pub penalty: f64,
    pub composite_score: f64,
    pub is_disputed: bool,
}
