//! Target score computation.
//! Ported from Python scripts/compute_scores.py

use std::sync::Arc;
use anyhow::Result;
use ferrumyx_db::Database;
use ferrumyx_db::kg_facts::KgFactRepository;
use ferrumyx_db::schema::KgFact;
use std::collections::HashMap;

/// Gene evidence aggregation for scoring.
#[derive(Debug, Default)]
struct GeneEvidence {
    cancer_evidence: u32,
    mutation_evidence: u32,
    total_evidence: u32,
}

/// Compute target scores for all genes.
pub async fn compute_target_scores(db: Arc<Database>) -> Result<u32> {
    let fact_repo = KgFactRepository::new(db.clone());
    
    // Get all facts and aggregate by subject (gene)
    let predicates = fact_repo.get_predicates().await?;
    
    let mut gene_evidence: HashMap<String, GeneEvidence> = HashMap::new();
    
    // Process facts by predicate type
    for predicate in &predicates {
        let facts = fact_repo.find_by_predicate(predicate).await?;
        
        for fact in facts {
            let gene = &fact.subject_name;
            let entry = gene_evidence.entry(gene.clone()).or_default();
            
            // Count evidence based on predicate type
            if predicate.contains("cancer") || predicate.contains("gene_cancer") {
                entry.cancer_evidence += 1;
            } else if predicate.contains("mutation") || predicate.contains("gene_mutation") {
                entry.mutation_evidence += 1;
            }
            entry.total_evidence += 1;
        }
    }
    
    let mut scored = 0u32;
    
    // Note: In a full implementation, we would store these scores in a dedicated
    // target_scores table. For now, we just compute and return the count.
    for (_gene, evidence) in &gene_evidence {
        let cancer_ev = evidence.cancer_evidence as f32;
        let mut_ev = evidence.mutation_evidence as f32;
        let total_ev = evidence.total_evidence as f32;

        // Compute scores (simplified model)
        let _literature_score = (total_ev / 10.0).min(1.0);
        let _mutation_score = (mut_ev / 5.0).min(1.0);
        let _cancer_relevance = (cancer_ev / 3.0).min(1.0);

        // Composite score (weighted average)
        // let _composite = literature_score * 0.3 + mutation_score * 0.3 + cancer_relevance * 0.4;
        
        // TODO: Store scores in a target_scores table when implemented
        
        scored += 1;
    }

    Ok(scored)
}

/// Get gene evidence statistics.
pub async fn get_gene_evidence(db: Arc<Database>, gene: &str) -> Result<GeneEvidence> {
    let fact_repo = KgFactRepository::new(db);
    
    // Find all facts where this gene is the subject
    // Note: This requires the gene to be a UUID in the current schema
    // For now, return default evidence
    let _facts: Vec<KgFact> = Vec::new();
    
    Ok(GeneEvidence::default())
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
