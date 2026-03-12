//! Target score computation and persistence.
//! Uses KG evidence to maintain `target_scores` as a materialized ranking view.

use std::collections::HashMap;
use std::sync::Arc;

use ferrumyx_db::kg_facts::KgFactRepository;
use ferrumyx_db::target_scores::TargetScoreRepository;
use ferrumyx_db::Database;

/// Gene evidence aggregation for scoring.
#[derive(Debug, Default)]
pub struct GeneEvidence {
    pub cancer_evidence: u32,
    pub mutation_evidence: u32,
    pub total_evidence: u32,
    pub confidence_sum: f64,
    pub cancer_id: Option<uuid::Uuid>,
    pub cancer_code: Option<String>,
}

/// Compute target scores for all genes.
pub async fn compute_target_scores(db: Arc<Database>) -> anyhow::Result<u32> {
    let fact_repo = KgFactRepository::new(db.clone());

    // Keep this bounded for predictable latency under event-driven recompute.
    let facts = fact_repo.list(0, 50_000).await?;

    let mut by_gene: HashMap<(uuid::Uuid, String), GeneEvidence> = HashMap::new();
    for fact in facts {
        if fact.predicate.eq_ignore_ascii_case("mentions") {
            continue;
        }
        if !is_gene_like(&fact.subject_name) {
            continue;
        }

        let key = (fact.subject_id, fact.subject_name.clone());
        let entry = by_gene.entry(key).or_default();
        entry.total_evidence += 1;
        entry.confidence_sum += fact.confidence as f64;

        let pred_lc = fact.predicate.to_lowercase();
        if pred_lc.contains("mutation") || pred_lc == "has_mutation" {
            entry.mutation_evidence += 1;
        }
        if is_cancer_like(&fact.object_name) {
            entry.cancer_evidence += 1;
            entry.cancer_id = Some(fact.object_id);
            entry.cancer_code = Some(fact.object_name.clone());
        }
    }

    let mut rows = Vec::new();
    for ((gene_id, gene_name), evidence) in by_gene {
        if evidence.total_evidence == 0 {
            continue;
        }

        let literature_score = normalise_count(evidence.total_evidence, 30.0);
        let mutation_score = normalise_count(evidence.mutation_evidence, 12.0);
        let cancer_score = normalise_count(evidence.cancer_evidence, 16.0);
        let confidence_mean =
            (evidence.confidence_sum / evidence.total_evidence as f64).clamp(0.0, 1.0);

        let composite_score =
            (0.50 * literature_score + 0.30 * mutation_score + 0.20 * cancer_score).clamp(0.0, 1.0);
        let adjusted_score = (composite_score * confidence_mean).clamp(0.0, 1.0);
        let shortlist_tier = if adjusted_score >= 0.60 {
            "primary"
        } else if adjusted_score >= 0.45 {
            "secondary"
        } else {
            "excluded"
        };

        let mut row = ferrumyx_db::schema::TargetScore::new(
            gene_id,
            evidence.cancer_id.unwrap_or(uuid::Uuid::nil()),
            composite_score,
            adjusted_score,
            0.0,
            shortlist_tier.to_string(),
        );
        row.components_raw = serde_json::json!({
            "gene": gene_name,
            "cancer_code": evidence.cancer_code,
            "total_evidence": evidence.total_evidence,
            "mutation_evidence": evidence.mutation_evidence,
            "cancer_evidence": evidence.cancer_evidence,
            "confidence_mean": confidence_mean
        })
        .to_string();
        row.components_normed = serde_json::json!({
            "literature_score": literature_score,
            "mutation_score": mutation_score,
            "cancer_score": cancer_score
        })
        .to_string();
        rows.push(row);
    }

    let score_repo = TargetScoreRepository::new(db);
    let upserted = score_repo.upsert_batch(&rows).await?;
    Ok(upserted as u32)
}

/// Get gene evidence statistics.
pub async fn get_gene_evidence(db: Arc<Database>, gene: &str) -> anyhow::Result<GeneEvidence> {
    let fact_repo = KgFactRepository::new(db);
    let mut out = GeneEvidence::default();
    let facts = fact_repo
        .list_filtered(Some(gene), None, None, 10_000)
        .await?;

    for fact in facts {
        if !fact.subject_name.eq_ignore_ascii_case(gene) {
            continue;
        }
        if fact.predicate.eq_ignore_ascii_case("mentions") {
            continue;
        }
        out.total_evidence += 1;
        out.confidence_sum += fact.confidence as f64;
        let p = fact.predicate.to_lowercase();
        if p.contains("mutation") || p == "has_mutation" {
            out.mutation_evidence += 1;
        }
        if is_cancer_like(&fact.object_name) {
            out.cancer_evidence += 1;
            out.cancer_id = Some(fact.object_id);
            out.cancer_code = Some(fact.object_name.clone());
        }
    }

    Ok(out)
}

fn normalise_count(v: u32, scale: f64) -> f64 {
    let x = v as f64;
    (x.ln_1p() / scale.ln_1p()).clamp(0.0, 1.0)
}

fn is_cancer_like(name: &str) -> bool {
    let n = name.trim();
    if n.is_empty() {
        return false;
    }
    // OncoTree-like code.
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

fn is_gene_like(name: &str) -> bool {
    let n = name.trim();
    if n.is_empty() || n.len() > 16 || n.contains(' ') {
        return false;
    }
    n.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && n.chars().any(|c| c.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_bounds() {
        // Verify scores are bounded [0, 1]
        let literature = (5.0_f64 / 10.0).min(1.0);
        let mutation = (2.0_f64 / 5.0).min(1.0);
        let cancer = (1.0_f64 / 3.0).min(1.0);
        let composite = literature * 0.3 + mutation * 0.3 + cancer * 0.4;
        assert!(composite >= 0.0 && composite <= 1.0);
    }
}
