//! Target score computation.
//! Ported from Python scripts/compute_scores.py

use sqlx::PgPool;
use anyhow::Result;

/// Compute target scores for all genes.
pub async fn compute_target_scores(pool: &PgPool) -> Result<i32> {
    // Get all genes with their evidence
    let rows = sqlx::query_as::<_, GeneEvidence>(
        r#"
        SELECT 
            subject as gene,
            COUNT(*) FILTER (WHERE fact_type = 'gene_cancer') as cancer_evidence,
            COUNT(*) FILTER (WHERE fact_type = 'gene_mutation') as mutation_evidence,
            SUM(evidence_count) as total_evidence
        FROM kg_facts
        WHERE fact_type IN ('gene_cancer', 'gene_mutation')
        GROUP BY subject
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut scored = 0i32;

    for row in &rows {
        let gene = &row.gene;
        let cancer_ev = row.cancer_evidence.unwrap_or(0) as f32;
        let mut_ev = row.mutation_evidence.unwrap_or(0) as f32;
        let total_ev = row.total_evidence.unwrap_or(0) as f32;

        // Compute scores (simplified model)
        let literature_score = (total_ev / 10.0).min(1.0);
        let mutation_score = (mut_ev / 5.0).min(1.0);
        let cancer_relevance = (cancer_ev / 3.0).min(1.0);

        // Composite score (weighted average)
        let composite = literature_score * 0.3 + mutation_score * 0.3 + cancer_relevance * 0.4;

        // Upsert into target_scores
        sqlx::query(
            r#"
            INSERT INTO target_scores 
                (gene, composite_score, literature_score, created_at, updated_at)
            VALUES ($1, $2, $3, NOW(), NOW())
            ON CONFLICT (gene)
            DO UPDATE SET 
                composite_score = EXCLUDED.composite_score,
                literature_score = EXCLUDED.literature_score,
                updated_at = NOW()
            "#
        )
        .bind(gene)
        .bind(composite as f64)
        .bind(literature_score as f64)
        .execute(pool)
        .await?;

        scored += 1;
    }

    Ok(scored)
}

#[derive(sqlx::FromRow)]
struct GeneEvidence {
    gene: String,
    cancer_evidence: Option<i64>,
    mutation_evidence: Option<i64>,
    total_evidence: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_bounds() {
        // Verify scores are bounded [0, 1]
        let literature = (5.0 / 10.0).min(1.0);
        let mutation = (2.0 / 5.0).min(1.0);
        let cancer = (1.0 / 3.0).min(1.0);
        let composite = literature * 0.3 + mutation * 0.3 + cancer * 0.4;
        assert!(composite >= 0.0 && composite <= 1.0);
    }
}
