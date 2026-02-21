//! Score normalisation functions.
//! See ARCHITECTURE.md §4.2 — rank-based normalisation.

/// Rank-based normalisation: assign rank r in [1, N], then n = r/N.
/// Handles ties by averaging ranks.
/// Returns normalised scores in the same order as input.
pub fn rank_normalise(raw_scores: &[f64], higher_is_better: bool) -> Vec<f64> {
    let n = raw_scores.len();
    if n == 0 {
        return vec![];
    }

    // Create (original_index, score) pairs and sort
    let mut indexed: Vec<(usize, f64)> = raw_scores
        .iter()
        .copied()
        .enumerate()
        .collect();

    // Sort ascending or descending based on direction
    if higher_is_better {
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    } else {
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    }

    // Assign ranks (1-indexed), handle ties by averaging
    let mut ranks = vec![0.0f64; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        // Find group of equal scores
        while j < n - 1 && (indexed[j].1 - indexed[j + 1].1).abs() < 1e-10 {
            j += 1;
        }
        // Average rank for ties
        let avg_rank = (i + 1 + j + 1) as f64 / 2.0;
        for k in i..=j {
            ranks[indexed[k].0] = avg_rank;
        }
        i = j + 1;
    }

    // Normalise: rank / N → [1/N, 1.0]
    ranks.iter().map(|&r| r / n as f64).collect()
}

/// Min-max normalisation within a given range [min_val, max_val].
/// Used for CRISPR CERES scores (see ARCHITECTURE.md §4.2).
pub fn minmax_normalise(value: f64, min_val: f64, max_val: f64) -> f64 {
    if (max_val - min_val).abs() < 1e-10 {
        return 0.5; // degenerate case
    }
    ((value - min_val) / (max_val - min_val)).clamp(0.0, 1.0)
}

/// Normalise a CERES score for use as component n2.
/// CERES range: [-2.0, 0.0]; more negative = more essential = higher score.
/// See ARCHITECTURE.md §4.2
pub fn normalise_ceres(ceres_score: f64) -> f64 {
    let clamped = ceres_score.clamp(-2.0, 0.0);
    let norm = minmax_normalise(clamped, -2.0, 0.0);
    1.0 - norm  // invert: more essential (more negative) → higher normalised score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rank_normalise_basic() {
        let scores = vec![10.0, 20.0, 30.0];
        let normed = rank_normalise(&scores, true);
        // 30.0 is rank 1 (best) → 1/3 ≈ 0.333 ... wait, rank 1/3 of 3 = 0.333
        // Actually: rank 1 → 1/3, rank 2 → 2/3, rank 3 → 3/3 = 1.0
        // 30.0 is best (rank 1), 20.0 rank 2, 10.0 rank 3
        assert!((normed[2] - 1.0/3.0).abs() < 1e-6); // 30.0 → rank 1 → 1/3
        assert!((normed[1] - 2.0/3.0).abs() < 1e-6); // 20.0 → rank 2 → 2/3
        assert!((normed[0] - 3.0/3.0).abs() < 1e-6); // 10.0 → rank 3 → 3/3
    }

    #[test]
    fn test_ceres_normalisation() {
        // -2.0 (maximally essential) should → 1.0
        assert!((normalise_ceres(-2.0) - 1.0).abs() < 1e-6);
        // 0.0 (not essential) should → 0.0
        assert!((normalise_ceres(0.0) - 0.0).abs() < 1e-6);
        // -1.0 (moderate) should → 0.5
        assert!((normalise_ceres(-1.0) - 0.5).abs() < 1e-6);
    }
}
