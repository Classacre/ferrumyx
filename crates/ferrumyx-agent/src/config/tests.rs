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
    fn test_default_llm_is_local_only() {
        let mode = default_llm_mode();
        assert_eq!(mode, "local_only");
    }

    #[test]
    fn test_default_llm_limits_alert_below_max() {
        let limits = LlmLimits {
            max_tokens_per_day_openai: default_500k(),
            max_tokens_per_day_anthropic: default_500k(),
            max_cost_per_day_usd: default_cost_limit(),
            alert_cost_threshold_usd: default_alert_threshold(),
        };
        assert!(limits.max_cost_per_day_usd > limits.alert_cost_threshold_usd,
            "Alert threshold ({}) should be below max cost ({})",
            limits.alert_cost_threshold_usd, limits.max_cost_per_day_usd);
    }
}
