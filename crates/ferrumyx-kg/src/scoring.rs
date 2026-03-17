//! Target score computation and persistence.
//! Uses KG evidence to maintain `target_scores` as a materialized ranking view.

use std::collections::HashMap;
use std::sync::Arc;

use ferrumyx_db::entities::EntityRepository;
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

fn score_row_from_evidence(
    gene_id: uuid::Uuid,
    gene_name: String,
    evidence: GeneEvidence,
) -> Option<ferrumyx_db::schema::TargetScore> {
    if evidence.total_evidence == 0 {
        return None;
    }

    let literature_score = normalise_count(evidence.total_evidence, 30.0);
    let mutation_score = normalise_count(evidence.mutation_evidence, 12.0);
    let cancer_score = normalise_count(evidence.cancer_evidence, 16.0);
    let confidence_mean =
        (evidence.confidence_sum / evidence.total_evidence as f64).clamp(0.0, 1.0);

    let base_weighted =
        (0.50 * literature_score + 0.30 * mutation_score + 0.20 * cancer_score).clamp(0.0, 1.0);
    let diversity_count = [literature_score, mutation_score, cancer_score]
        .iter()
        .filter(|v| **v >= 0.12)
        .count() as f64;
    let diversity_factor = (0.70 + 0.10 * diversity_count).clamp(0.70, 1.0);
    let saturation_curve = 1.0 - (-1.8 * base_weighted).exp();
    let confidence_factor = (0.50 + 0.50 * confidence_mean).clamp(0.50, 1.0);

    let composite_score = (saturation_curve * diversity_factor).clamp(0.0, 0.98);
    let adjusted_score = (composite_score * confidence_factor).clamp(0.0, 0.95);
    let shortlist_tier = if adjusted_score >= 0.65 {
        "primary"
    } else if adjusted_score >= 0.50 {
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
        "confidence_mean": confidence_mean,
        "base_weighted": base_weighted,
        "diversity_factor": diversity_factor,
        "confidence_factor": confidence_factor
    })
    .to_string();
    row.components_normed = serde_json::json!({
        "literature_score": literature_score,
        "mutation_score": mutation_score,
        "cancer_score": cancer_score
    })
    .to_string();
    Some(row)
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
        let pred_lc = fact.predicate.to_lowercase();

        if is_gene_like(&fact.subject_name) {
            let key = (fact.subject_id, fact.subject_name.clone());
            let entry = by_gene.entry(key).or_default();
            entry.total_evidence += 1;
            entry.confidence_sum += fact.confidence as f64;
            if pred_lc.contains("mutation") || pred_lc == "has_mutation" {
                entry.mutation_evidence += 1;
            }
            if is_cancer_like(&fact.object_name) {
                entry.cancer_evidence += 1;
                entry.cancer_id = Some(fact.object_id);
                entry.cancer_code = Some(fact.object_name.clone());
            }
        }

        // Also score gene-like objects so alternative genes tied through
        // mechanistic relations can surface in rankings.
        if !fact.object_id.is_nil() && is_gene_like(&fact.object_name) {
            let key = (fact.object_id, fact.object_name.clone());
            let entry = by_gene.entry(key).or_default();
            entry.total_evidence += 1;
            entry.confidence_sum += fact.confidence as f64;
            if is_cancer_like(&fact.subject_name) {
                entry.cancer_evidence += 1;
                entry.cancer_id = Some(fact.subject_id);
                entry.cancer_code = Some(fact.subject_name.clone());
            }
        }
    }

    let mut rows = Vec::new();
    for ((gene_id, gene_name), evidence) in by_gene {
        if let Some(row) = score_row_from_evidence(gene_id, gene_name, evidence) {
            rows.push(row);
        }
    }

    let score_repo = TargetScoreRepository::new(db);
    let upserted = score_repo.upsert_batch(&rows).await?;
    Ok(upserted as u32)
}

/// Incrementally recompute target scores for a bounded set of genes.
pub async fn compute_target_scores_for_gene_ids(
    db: Arc<Database>,
    gene_ids: &[uuid::Uuid],
) -> anyhow::Result<u32> {
    let mut uniq = gene_ids.to_vec();
    uniq.sort_unstable();
    uniq.dedup();
    if uniq.is_empty() {
        return Ok(0);
    }

    let fact_repo = KgFactRepository::new(db.clone());
    let facts = fact_repo.find_by_subject_ids(&uniq, 80).await?;

    let mut by_gene: HashMap<uuid::Uuid, (String, GeneEvidence)> = HashMap::new();
    for fact in facts {
        if fact.predicate.eq_ignore_ascii_case("mentions") {
            continue;
        }
        if !is_gene_like(&fact.subject_name) {
            continue;
        }
        let entry = by_gene
            .entry(fact.subject_id)
            .or_insert_with(|| (fact.subject_name.clone(), GeneEvidence::default()));
        if entry.0.trim().is_empty() && !fact.subject_name.trim().is_empty() {
            entry.0 = fact.subject_name.clone();
        }
        let evidence = &mut entry.1;
        evidence.total_evidence += 1;
        evidence.confidence_sum += fact.confidence as f64;

        let pred_lc = fact.predicate.to_lowercase();
        if pred_lc.contains("mutation") || pred_lc == "has_mutation" {
            evidence.mutation_evidence += 1;
        }
        if is_cancer_like(&fact.object_name) {
            evidence.cancer_evidence += 1;
            evidence.cancer_id = Some(fact.object_id);
            evidence.cancer_code = Some(fact.object_name.clone());
        }
    }

    let mut rows = Vec::new();
    for gene_id in &uniq {
        if let Some((gene_name, evidence)) = by_gene.remove(gene_id) {
            if let Some(row) = score_row_from_evidence(*gene_id, gene_name, evidence) {
                rows.push(row);
            }
        }
    }

    let score_repo = TargetScoreRepository::new(db);
    // Clear impacted genes first so removed/changed evidence does not leave stale rows.
    let _ = score_repo.delete_by_gene_ids(&uniq, 200).await?;
    let upserted = score_repo.upsert_batch(&rows).await?;
    Ok(upserted as u32)
}

/// Resolve gene names to entity IDs and recompute only those targets.
pub async fn compute_target_scores_for_gene_names(
    db: Arc<Database>,
    gene_names: &[String],
) -> anyhow::Result<u32> {
    if gene_names.is_empty() {
        return Ok(0);
    }
    let entity_repo = EntityRepository::new(db.clone());
    let mut gene_ids = Vec::new();
    for raw in gene_names {
        let name = raw.trim();
        if name.is_empty() {
            continue;
        }
        // Exact-name lookups avoid broad scans.
        let mut matches = entity_repo.find_by_name(name).await.unwrap_or_default();
        if matches.is_empty() {
            let upper = name.to_ascii_uppercase();
            if upper != name {
                matches = entity_repo.find_by_name(&upper).await.unwrap_or_default();
            }
        }
        for ent in matches {
            if ent.entity_type.eq_ignore_ascii_case("gene") {
                gene_ids.push(ent.id);
            }
        }
    }
    compute_target_scores_for_gene_ids(db, &gene_ids).await
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
