#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_default_scoring_thresholds() {
        let scoring = ScoringConfig {
            focus_cancer: default_focus_cancer(),
            focus_mutation: default_focus_mutation(),
            primary_shortlist_threshold: default_primary_threshold(),
            secondary_shortlist_threshold: default_secondary_threshold(),
        };
        assert_eq!(scoring.focus_cancer, "PAAD");
        assert!(scoring.primary_shortlist_threshold > scoring.secondary_shortlist_threshold);
    }

    #[test]
    fn test_default_llm_mode_is_any() {
        // Mode changed from "local_only" to "any" to support API backends
        let mode = default_llm_mode();
        assert_eq!(mode, "any");
    }

    #[test]
    fn test_default_llm_limits_alert_below_max() {
        let limits = LlmLimits {
            max_tokens_per_day_openai:    default_500k(),
            max_tokens_per_day_anthropic: default_500k(),
            max_tokens_per_day_gemini:    default_1m(),
            max_cost_per_day_usd:         default_cost_limit(),
            alert_cost_threshold_usd:     default_alert_threshold(),
        };
        assert!(limits.max_cost_per_day_usd > limits.alert_cost_threshold_usd,
            "Alert threshold ({}) should be below max cost ({})",
            limits.alert_cost_threshold_usd, limits.max_cost_per_day_usd);
        assert_eq!(limits.max_tokens_per_day_gemini, 1_000_000);
    }

    #[test]
    fn test_api_backend_config_defaults() {
        let cfg = OllamaBackendConfig {
            base_url: default_ollama_url(),
            model:    default_ollama_model(),
        };
        assert!(cfg.base_url.starts_with("http"));
        assert!(!cfg.model.is_empty());
    }

    #[test]
    fn test_embedding_backend_defaults() {
        let emb = EmbeddingConfig {
            backend:         default_embed_backend(),
            api_key:         String::new(),
            embedding_model: default_embed_model(),
            embedding_dim:   default_embed_dim(),
            batch_size:      default_batch_size(),
            biomedbert_url:  default_biomedbert_url(),
        };
        assert_eq!(emb.backend, "openai");
        assert_eq!(emb.embedding_model, "text-embedding-3-small");
        assert_eq!(emb.embedding_dim, 1536);
    }
}
