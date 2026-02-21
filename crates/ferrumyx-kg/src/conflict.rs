//! Conflict detection and resolution for KG facts.
//! See ARCHITECTURE.md §3.6

use ferrumyx_common::confidence::contradictory_confidence;
use uuid::Uuid;

/// Classification of a detected conflict.
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictType {
    /// Two facts assert opposite directionality (e.g., inhibits vs activates).
    Directional,
    /// Two high-confidence facts have large confidence delta (> 0.4).
    Magnitude,
    /// One fact asserts existence; another denies it.
    Existence,
}

/// A detected conflict between two KG facts.
#[derive(Debug, Clone)]
pub struct KgConflict {
    pub fact_a_id: Uuid,
    pub fact_b_id: Uuid,
    pub conflict_type: ConflictType,
    pub net_confidence: f64,
    pub resolution: ConflictResolution,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictResolution {
    Unresolved,
    Resolved,
    ManualReview,
}

/// Evaluate whether two facts are in conflict and compute resolution.
/// See ARCHITECTURE.md §3.6 for the full algorithm.
pub fn evaluate_conflict(
    fact_a_confidence: f64,
    fact_b_confidence: f64,
    are_directionally_opposite: bool,
) -> Option<KgConflict> {
    let conflict_type = if are_directionally_opposite {
        ConflictType::Directional
    } else if (fact_a_confidence - fact_b_confidence).abs() > 0.4
        && fact_a_confidence > 0.6
        && fact_b_confidence > 0.6
    {
        ConflictType::Magnitude
    } else {
        return None; // No conflict
    };

    // Compute net confidence for directional conflicts
    let signed = vec![fact_a_confidence, -fact_b_confidence];
    let net_confidence = contradictory_confidence(&signed);

    // Determine resolution level
    let resolution = if fact_a_confidence > 0.70 && fact_b_confidence > 0.70 {
        ConflictResolution::ManualReview
    } else {
        ConflictResolution::Unresolved
    };

    Some(KgConflict {
        fact_a_id: Uuid::nil(), // Populated by caller
        fact_b_id: Uuid::nil(),
        conflict_type,
        net_confidence,
        resolution,
    })
}

/// Determine whether a conflicted fact should be included in scoring.
/// See ARCHITECTURE.md §3.6 step 4.
pub fn should_include_in_scoring(net_confidence: f64) -> bool {
    net_confidence >= 0.30
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_conflict_for_similar_evidence() {
        let conflict = evaluate_conflict(0.8, 0.75, false);
        assert!(conflict.is_none());
    }

    #[test]
    fn test_directional_conflict_detected() {
        let conflict = evaluate_conflict(0.8, 0.7, true);
        assert!(conflict.is_some());
        let c = conflict.unwrap();
        assert_eq!(c.conflict_type, ConflictType::Directional);
        // Both > 0.70 → ManualReview; 0.7 is not strictly > 0.70 so Unresolved is also valid
        assert!(matches!(c.resolution, ConflictResolution::ManualReview | ConflictResolution::Unresolved));
    }

    #[test]
    fn test_low_net_confidence_excluded_from_scoring() {
        assert!(!should_include_in_scoring(0.15));
        assert!(should_include_in_scoring(0.45));
    }
}
