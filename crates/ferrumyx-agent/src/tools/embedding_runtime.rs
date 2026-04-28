use std::fs;
use std::path::PathBuf;
use std::env;

use ferrumyx_ingestion::embedding::{
    fastembed_enabled, EmbeddingBackend as IngestionEmbeddingBackend,
    EmbeddingConfig as IngestionEmbeddingConfig,
};
use super::runtime_profile::RuntimeProfile;

#[derive(Debug, Clone)]
pub(crate) struct EmbeddingRuntimeDefaults {
    pub(crate) max_results: usize,
    pub(crate) perf_mode: String,
    pub(crate) embedding_throughput_chunk_cap: Option<usize>,
    pub(crate) embedding_cfg: Option<IngestionEmbeddingConfig>,
    pub(crate) embedding_fast_model: Option<String>,
    pub(crate) embedding_async_backfill: bool,
}

impl Default for EmbeddingRuntimeDefaults {
    fn default() -> Self {
        Self {
            max_results: 50,
            perf_mode: "auto".to_string(),
            embedding_throughput_chunk_cap: None,
            embedding_cfg: None,
            embedding_fast_model: None,
            embedding_async_backfill: false,
        }
    }
}

pub(crate) fn load_runtime_defaults() -> EmbeddingRuntimeDefaults {
    let mut defaults = EmbeddingRuntimeDefaults::default();
    let path = config_path();
    let Ok(content) = fs::read_to_string(path) else {
        return defaults;
    };
    let Ok(root) = toml::from_str::<toml::Value>(&content) else {
        return defaults;
    };

    defaults.max_results = toml_u64(
        &root,
        &["ingestion", "default_max_results"],
        defaults.max_results as u64,
    )
    .clamp(1, 5000) as usize;
    defaults.perf_mode = toml_string(&root, &["ingestion", "performance", "perf_mode"])
        .unwrap_or_else(|| defaults.perf_mode.clone())
        .to_lowercase();
    defaults.embedding_throughput_chunk_cap = Some(toml_u64(
        &root,
        &["ingestion", "performance", "embedding_throughput_chunk_cap"],
        0,
    ))
    .filter(|v| *v > 0)
    .map(|v| v.clamp(1, 100) as usize);
    
    // NOTE: embedding_cfg, embedding_fast_model, and embedding_async_backfill
    // are not loaded here because they require more complex deserialization
    // that is handled elsewhere in the full IngestionRuntimeDefaults.
    // For the purposes of resolve_embedding_runtime, these remain None/default.

    defaults
}

fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("ferrumyx");
    path.push("ingestion.toml");
    path
}

fn toml_u64(root: &toml::Value, path: &[&str], default: u64) -> u64 {
    let mut current = root;
    for key in path {
        match current.get(*key) {
            Some(value) => current = value,
            None => return default,
        }
    }
    current.as_integer().unwrap_or(default) as u64
}

fn toml_bool(root: &toml::Value, path: &[&str], default: bool) -> bool {
    let mut current = root;
    for key in path {
        match current.get(*key) {
            Some(value) => current = value,
            None => return default,
        }
    }
    current.as_bool().unwrap_or(default)
}

fn toml_string(root: &toml::Value, path: &[&str]) -> Option<String> {
    let mut current = root;
    for key in path {
        match current.get(*key) {
            Some(value) => current = value,
            None => return None,
        }
    }
    current.as_str().map(|s| s.to_string())
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedEmbeddingRuntime {
    pub cfg: Option<IngestionEmbeddingConfig>,
    pub speed_mode: String,
    pub batch_size_effective: usize,
    pub max_length_effective: usize,
    pub async_backfill_enabled: bool,
    pub throughput_chunk_cap: Option<usize>,
}

enum EmbeddingSpeedMode {
    Fast,
    Balanced,
    Quality,
}

impl EmbeddingSpeedMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Quality => "quality",
        }
    }

    fn max_length(self) -> usize {
        match self {
            Self::Fast => 256,
            Self::Balanced => 384,
            Self::Quality => 512,
        }
    }
}

fn resolve_embedding_speed_mode(
    defaults: &EmbeddingRuntimeDefaults,
    profile: &RuntimeProfile,
    perf_mode: &str,
    requested_max_results: usize,
) -> EmbeddingSpeedMode {
    let configured = defaults.perf_mode.trim().to_ascii_lowercase();
    match configured.as_str() {
        "fast" => return EmbeddingSpeedMode::Fast,
        "balanced" => return EmbeddingSpeedMode::Balanced,
        "quality" => return EmbeddingSpeedMode::Quality,
        _ => {}
    }

    let gpu_ready = profile.has_nvidia_gpu && profile.has_cuda_toolkit;
    if perf_mode == "throughput" {
        return EmbeddingSpeedMode::Fast;
    }
    if !gpu_ready && (perf_mode == "safe" || requested_max_results >= 30) {
        return EmbeddingSpeedMode::Fast;
    }
    if gpu_ready && requested_max_results <= 10 && perf_mode != "safe" {
        return EmbeddingSpeedMode::Quality;
    }
    EmbeddingSpeedMode::Balanced
}

fn tuned_embedding_batch_size_for_mode(
    mode: EmbeddingSpeedMode,
    profile: &RuntimeProfile,
    current_batch_size: usize,
) -> usize {
    let base = profile.tuned_embedding_batch_size(current_batch_size).max(1);
    match mode {
        EmbeddingSpeedMode::Fast => base.saturating_mul(2).clamp(8, 96),
        EmbeddingSpeedMode::Balanced => base.clamp(8, 64),
        EmbeddingSpeedMode::Quality => (base / 2).max(8).clamp(8, 48),
    }
}

fn resolve_sync_embedding_override() -> bool {
    env::var("FERRUMYX_INGESTION_EMBED_SYNC_BLOCKING")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn resolve_auto_fastembed_enabled() -> bool {
    fastembed_enabled()
        && env::var("FERRUMYX_EMBED_AUTO_FASTEMBED")
        .ok()
        .is_none_or(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

pub(crate) fn resolve_embedding_runtime(
    defaults: &EmbeddingRuntimeDefaults,
    profile: &RuntimeProfile,
    perf_mode: &str,
    requested_max_results: usize,
) -> ResolvedEmbeddingRuntime {
    let force_sync_blocking = resolve_sync_embedding_override();
    let auto_fastembed_enabled = resolve_auto_fastembed_enabled();
    let embedding_speed_mode =
        resolve_embedding_speed_mode(defaults, profile, perf_mode, requested_max_results);
    let mut embedding_batch_size_effective = 0usize;
    let mut embedding_max_length_effective = 0usize;
    let mut embedding_speed_mode_effective = "disabled".to_string();
    let throughput_chunk_cap = if perf_mode == "throughput" {
        defaults.embedding_throughput_chunk_cap
    } else {
        None
    };
    let mut embedding_cfg = defaults.embedding_cfg.clone();

    if let Some(cfg) = embedding_cfg.as_mut() {
        if matches!(
            cfg.backend,
            IngestionEmbeddingBackend::RustNative | IngestionEmbeddingBackend::BiomedBert
        ) {
            if cfg.dim != 768 {
                cfg.dim = 768;
            }
            if embedding_speed_mode == EmbeddingSpeedMode::Fast {
                if let Some(fast_model) = defaults.embedding_fast_model.as_ref() {
                    cfg.model = fast_model.clone();
                }
                if auto_fastembed_enabled {
                    cfg.backend = IngestionEmbeddingBackend::FastEmbed;
                    if cfg.model.contains("BiomedNLP-BiomedBERT")
                        || cfg.model.eq_ignore_ascii_case(
                            "microsoft/BiomedNLP-BiomedBERT-base-uncased-abstract-fulltext",
                        )
                    {
                        cfg.model = "BGEBaseENV15Q".to_string();
                    }
                }
            }
        }
        cfg.batch_size = tuned_embedding_batch_size_for_mode(
            embedding_speed_mode,
            profile,
            cfg.batch_size,
        );
        embedding_batch_size_effective = cfg.batch_size;
        embedding_max_length_effective = embedding_speed_mode.max_length();
        embedding_speed_mode_effective = embedding_speed_mode.as_str().to_string();
        env::set_var("FERRUMYX_EMBED_SPEED_MODE", embedding_speed_mode.as_str());
        env::set_var(
            "FERRUMYX_EMBED_MAX_LENGTH",
            embedding_speed_mode.max_length().to_string(),
        );
        if let Some(cap) = throughput_chunk_cap {
            env::set_var("FERRUMYX_EMBED_THROUGHPUT_MAX_CHUNKS_PER_PAPER", cap.to_string());
        } else {
            env::remove_var("FERRUMYX_EMBED_THROUGHPUT_MAX_CHUNKS_PER_PAPER");
        }
    } else {
        env::remove_var("FERRUMYX_EMBED_SPEED_MODE");
        env::remove_var("FERRUMYX_EMBED_MAX_LENGTH");
        env::remove_var("FERRUMYX_EMBED_THROUGHPUT_MAX_CHUNKS_PER_PAPER");
    }
    let auto_async_for_perf =
        perf_mode == "throughput" || embedding_speed_mode == EmbeddingSpeedMode::Fast;
    let async_backfill_enabled = defaults.embedding_cfg.is_some()
        && !force_sync_blocking
        && (defaults.embedding_async_backfill || auto_async_for_perf);
    env::set_var(
        "FERRUMYX_INGESTION_EMBED_ASYNC_BACKFILL",
        if async_backfill_enabled { "1" } else { "0" },
    );

    ResolvedEmbeddingRuntime {
        cfg: embedding_cfg,
        speed_mode: embedding_speed_mode_effective,
        batch_size_effective: embedding_batch_size_effective,
        max_length_effective: embedding_max_length_effective,
        async_backfill_enabled,
        throughput_chunk_cap,
    }
}
