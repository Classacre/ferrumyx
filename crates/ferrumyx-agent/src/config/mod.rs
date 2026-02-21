//! Configuration loading for Ferrumyx.
//! Reads ferrumyx.toml from the current directory or path in FERRUMYX_CONFIG env var.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub llm: LlmConfig,
    pub ingestion: IngestionConfig,
    pub embedding: EmbeddingConfig,
    pub ner: NerConfig,
    pub scoring: ScoringConfig,
    pub structural: StructuralConfig,
    pub security: SecurityConfig,
    pub workspace: WorkspaceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
}

fn default_max_connections() -> u32 { 10 }
fn default_min_connections() -> u32 { 2 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// "local_only" | "prefer_local" | "any"
    #[serde(default = "default_llm_mode")]
    pub mode: String,
    /// Backend name for PUBLIC data (e.g. "openai", "anthropic", "gemini")
    #[serde(default = "default_default_backend")]
    pub default_backend: String,
    /// Backend for CONFIDENTIAL/INTERNAL data (must be local)
    #[serde(default = "default_local_backend")]
    pub local_backend: String,
    pub ollama:            Option<OllamaBackendConfig>,
    pub openai:            Option<ApiBackendConfig>,
    pub anthropic:         Option<ApiBackendConfig>,
    pub gemini:            Option<ApiBackendConfig>,
    pub openai_compatible: Option<OpenAiCompatibleConfig>,
    #[serde(default)]
    pub limits: LlmLimits,
    #[serde(default)]
    pub rate_limits: LlmRateLimits,
}

fn default_llm_mode()       -> String { "any".to_string() }
fn default_default_backend()-> String { "openai".to_string() }
fn default_local_backend()  -> String { "ollama".to_string() }

/// Config for Ollama (local endpoint).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaBackendConfig {
    #[serde(default = "default_ollama_url")]
    pub base_url: String,
    #[serde(default = "default_ollama_model")]
    pub model: String,
}

fn default_ollama_url()   -> String { "http://localhost:11434".to_string() }
fn default_ollama_model() -> String { "llama3.1:8b".to_string() }

/// Config for API-based providers (OpenAI, Anthropic, Gemini).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiBackendConfig {
    /// API key (plain text — use env var override in production)
    #[serde(default)]
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub embedding_model: Option<String>,
}

/// Config for OpenAI-compatible providers (Groq, TogetherAI, LMStudio, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatibleConfig {
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub embedding_model: Option<String>,
}

/// Legacy alias kept for backwards compat.
pub type LlmBackendConfig = ApiBackendConfig;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmLimits {
    #[serde(default = "default_500k")]
    pub max_tokens_per_day_openai: u64,
    #[serde(default = "default_500k")]
    pub max_tokens_per_day_anthropic: u64,
    #[serde(default = "default_1m")]
    pub max_tokens_per_day_gemini: u64,
    #[serde(default = "default_cost_limit")]
    pub max_cost_per_day_usd: f64,
    #[serde(default = "default_alert_threshold")]
    pub alert_cost_threshold_usd: f64,
}

fn default_1m() -> u64 { 1_000_000 }

fn default_500k()          -> u64 { 500_000 }
fn default_cost_limit()    -> f64 { 20.0 }
fn default_alert_threshold() -> f64 { 15.0 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmRateLimits {
    #[serde(default = "default_openai_rpm")]
    pub openai_rpm: u32,
    #[serde(default = "default_anthropic_rpm")]
    pub anthropic_rpm: u32,
    #[serde(default = "default_gemini_rpm")]
    pub gemini_rpm: u32,
    #[serde(default = "default_compat_rpm")]
    pub compat_rpm: u32,
    #[serde(default = "default_ollama_rpm")]
    pub ollama_rpm: u32,
}

fn default_openai_rpm()    -> u32 { 60 }
fn default_anthropic_rpm() -> u32 { 40 }
fn default_gemini_rpm()    -> u32 { 60 }
fn default_compat_rpm()    -> u32 { 60 }
fn default_ollama_rpm()    -> u32 { 120 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionConfig {
    #[serde(default = "default_sources")]
    pub sources: Vec<String>,
    pub pubmed: Option<SourceConfig>,
    pub europepmc: Option<SourceConfig>,
    pub semanticscholar: Option<SourceConfig>,
}

fn default_sources() -> Vec<String> {
    vec!["pubmed".to_string(), "europepmc".to_string()]
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceConfig {
    pub api_key_secret: Option<String>,
    #[serde(default = "default_rps")]
    pub requests_per_second: u32,
}

fn default_rps() -> u32 { 3 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// "openai" | "gemini" | "openai_compatible" | "biomedbert" | "ollama"
    #[serde(default = "default_embed_backend")]
    pub backend: String,
    /// Optional API key (blank = inherit from provider config)
    #[serde(default)]
    pub api_key: String,
    /// Model name used for embeddings
    #[serde(default = "default_embed_model")]
    pub embedding_model: String,
    /// Vector dimension (must match the model)
    #[serde(default = "default_embed_dim")]
    pub embedding_dim: usize,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// Local BiomedBERT service URL
    #[serde(default = "default_biomedbert_url")]
    pub biomedbert_url: String,
}

fn default_embed_backend()  -> String { "openai".to_string() }
fn default_embed_model()    -> String { "text-embedding-3-small".to_string() }
fn default_embed_dim()      -> usize  { 1536 }
fn default_batch_size()     -> usize  { 32 }
fn default_biomedbert_url() -> String { "http://localhost:8002".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NerConfig {
    #[serde(default = "default_ner_primary")]
    pub primary: String,
    #[serde(default = "default_bern2_threshold")]
    pub bern2_citation_threshold: u32,
    #[serde(default = "default_scispacy_image")]
    pub scispacy_docker_image: String,
    #[serde(default = "default_bern2_image")]
    pub bern2_docker_image: String,
    #[serde(default = "default_ner_url")]
    pub service_url: String,
}

fn default_ner_primary()    -> String { "scispacy".to_string() }
fn default_bern2_threshold()-> u32    { 50 }
fn default_scispacy_image() -> String { "ferrumyx/scispacy-ner:latest".to_string() }
fn default_bern2_image()    -> String { "ferrumyx/bern2-ner:latest".to_string() }
fn default_ner_url()        -> String { "http://localhost:8001".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConfig {
    #[serde(default = "default_focus_cancer")]
    pub focus_cancer: String,
    #[serde(default = "default_focus_mutation")]
    pub focus_mutation: String,
    #[serde(default = "default_primary_threshold")]
    pub primary_shortlist_threshold: f64,
    #[serde(default = "default_secondary_threshold")]
    pub secondary_shortlist_threshold: f64,
}

fn default_focus_cancer()       -> String { "PAAD".to_string() }
fn default_focus_mutation()     -> String { "KRAS_G12D".to_string() }
fn default_primary_threshold()  -> f64    { 0.60 }
fn default_secondary_threshold() -> f64   { 0.45 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralConfig {
    #[serde(default = "default_rps")]
    pub pdb_requests_per_second: u32,
    #[serde(default = "default_rps")]
    pub alphafold_requests_per_second: u32,
    #[serde(default = "default_fpocket_image")]
    pub fpocket_docker_image: String,
    #[serde(default = "default_vina_image")]
    pub vina_docker_image: String,
    #[serde(default = "default_rdkit_image")]
    pub rdkit_docker_image: String,
    #[serde(default = "default_admet_image")]
    pub admet_docker_image: String,
}

fn default_fpocket_image() -> String { "ferrumyx/fpocket:latest".to_string() }
fn default_vina_image()    -> String { "ferrumyx/autodock-vina:latest".to_string() }
fn default_rdkit_image()   -> String { "ferrumyx/rdkit-service:latest".to_string() }
fn default_admet_image()   -> String { "ferrumyx/admet-ai:latest".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    #[serde(default = "bool_true")]
    pub audit_llm_calls: bool,
    #[serde(default = "bool_true")]
    pub enforce_data_classification: bool,
    #[serde(default = "default_retention")]
    pub audit_log_retention_days: u32,
}

fn bool_true()          -> bool { true }
fn default_retention()  -> u32  { 90 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default = "default_workspace_path")]
    pub path: String,
}

fn default_workspace_path() -> String { "./workspace".to_string() }

mod tests;

impl Config {
    /// Load configuration from ferrumyx.toml.
    /// Checks FERRUMYX_CONFIG env var first, then current directory.
    pub fn load() -> anyhow::Result<Self> {
        let path = std::env::var("FERRUMYX_CONFIG")
            .unwrap_or_else(|_| "ferrumyx.toml".to_string());

        if !Path::new(&path).exists() {
            anyhow::bail!(
                "Config file not found: {}\n\
                 Copy ferrumyx.example.toml to ferrumyx.toml and edit it.",
                path
            );
        }

        let content = std::fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
