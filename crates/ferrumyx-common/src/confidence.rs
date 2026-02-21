/// Confidence scoring logic for KG facts.
/// Implements the model defined in ARCHITECTURE.md §3.2

/// Confidence modifiers based on study metadata.
#[derive(Debug, Clone, Default)]
pub struct ConfidenceModifiers {
    pub sample_size: Option<i32>,
    pub replicated_count: Option<i32>, // number of independent studies
    pub journal_impact_factor: Option<f64>,
    pub is_preprint: bool,
    pub is_single_cell_line_only: bool,
    pub is_retracted: bool,
}

/// Compute final confidence given base weight and modifiers.
/// Returns value in [0.0, 1.0].
pub fn compute_confidence(base_weight: f64, mods: &ConfidenceModifiers) -> f64 {
    if mods.is_retracted {
        return 0.0;
    }

    let mut confidence = base_weight;

    // Sample size modifier
    if let Some(n) = mods.sample_size {
        if n > 1000 {
            confidence *= 1.20;
        }
    }

    // Replication modifier
    if let Some(reps) = mods.replicated_count {
        if reps >= 2 {
            confidence *= 1.15;
        }
    }

    // Journal IF modifier
    if let Some(if_score) = mods.journal_impact_factor {
        if if_score > 10.0 {
            confidence *= 1.05;
        }
    }

    // Preprint penalty
    if mods.is_preprint {
        confidence *= 0.70;
    }

    // Single cell-line penalty
    if mods.is_single_cell_line_only {
        confidence *= 0.85;
    }

    // Cap at 1.0
    confidence.min(1.0)
}

/// Aggregate confidence from multiple independent evidence sources
/// using the noisy-OR model: p = 1 - Π(1 - p_i)
pub fn aggregate_confidence(confidences: &[f64]) -> f64 {
    if confidences.is_empty() {
        return 0.0;
    }
    let product: f64 = confidences.iter().map(|&p| 1.0 - p).product();
    1.0 - product
}

/// Handle contradictory evidence: compute net confidence
/// and apply contradiction penalty (×0.70).
/// signed_confidences: positive = supporting, negative = contradicting
pub fn contradictory_confidence(signed_confidences: &[f64]) -> f64 {
    let net: f64 = signed_confidences.iter().sum::<f64>().abs();
    // Apply contradiction penalty
    (net * 0.70).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retracted_is_zero() {
        let mods = ConfidenceModifiers { is_retracted: true, ..Default::default() };
        assert_eq!(compute_confidence(0.9, &mods), 0.0);
    }

    #[test]
    fn test_preprint_penalty() {
        let mods = ConfidenceModifiers { is_preprint: true, ..Default::default() };
        let c = compute_confidence(0.85, &mods);
        assert!((c - 0.595).abs() < 1e-6);
    }

    #[test]
    fn test_aggregate_noisy_or() {
        // Two independent pieces of evidence at 0.7 each
        // Expected: 1 - (0.3 * 0.3) = 0.91
        let agg = aggregate_confidence(&[0.7, 0.7]);
        assert!((agg - 0.91).abs() < 1e-6);
    }

    #[test]
    fn test_capped_at_one() {
        let mods = ConfidenceModifiers {
            sample_size: Some(5000),
            replicated_count: Some(5),
            journal_impact_factor: Some(50.0),
            ..Default::default()
        };
        assert!(compute_confidence(1.0, &mods) <= 1.0);
    }
}
