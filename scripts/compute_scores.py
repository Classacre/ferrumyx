#!/usr/bin/env python3
"""
Compute target scores from KG facts.
9-component scoring system from ARCHITECTURE.md §3.
"""

import os
import psycopg2
from datetime import datetime

DB_URL = os.environ.get(
    "DATABASE_URL",
    "postgresql://ferrumyx:ferrumyx_dev@localhost:5432/ferrumyx"
)

def get_connection():
    return psycopg2.connect(DB_URL)

def compute_scores(conn):
    """Compute target scores for all genes."""
    print("Computing target scores...")

    with conn.cursor() as cur:
        # Get all genes with their evidence
        cur.execute("""
            SELECT 
                subject as gene,
                COUNT(*) FILTER (WHERE fact_type = 'gene_cancer') as cancer_evidence,
                COUNT(*) FILTER (WHERE fact_type = 'gene_mutation') as mutation_evidence,
                SUM(evidence_count) as total_evidence
            FROM kg_facts
            WHERE fact_type IN ('gene_cancer', 'gene_mutation')
            GROUP BY subject
        """)

        genes = cur.fetchall()

        for gene, cancer_ev, mut_ev, total_ev in genes:
            # Compute composite score (simplified - full model needs external data)
            # Components from ARCHITECTURE.md §3:
            # 1. CRISPR dependency (DepMap) - not yet integrated
            # 2. Mutation frequency - from KG
            # 3. Expression specificity - not yet integrated
            # 4. Pathway centrality - not yet integrated
            # 5. Clinical trial count - not yet integrated
            # 6. Druggability score - not yet integrated
            # 7. Literature momentum - from KG
            # 8. Survival association - not yet integrated
            # 9. Synthetic lethality - not yet integrated

            # For now, use literature-based score
            literature_score = min(total_ev / 10.0, 1.0) if total_ev else 0.0
            mutation_score = min(mut_ev / 5.0, 1.0) if mut_ev else 0.0
            cancer_relevance = min(cancer_ev / 3.0, 1.0) if cancer_ev else 0.0

            # Composite score (weighted average)
            composite = (
                literature_score * 0.3 +
                mutation_score * 0.3 +
                cancer_relevance * 0.4
            )

            # Insert into target_scores
            cur.execute("""
                INSERT INTO target_scores 
                    (gene, composite_score, literature_score, created_at, updated_at)
                VALUES (%s, %s, %s, NOW(), NOW())
                ON CONFLICT (gene)
                DO UPDATE SET 
                    composite_score = EXCLUDED.composite_score,
                    literature_score = EXCLUDED.literature_score,
                    updated_at = NOW()
            """, (gene, composite, literature_score))

        conn.commit()

    print(f"Scored {len(genes)} genes")

def main():
    conn = get_connection()
    compute_scores(conn)

    # Show top targets
    with conn.cursor() as cur:
        cur.execute("""
            SELECT gene, composite_score, literature_score
            FROM target_scores
            ORDER BY composite_score DESC
            LIMIT 10
        """)
        print("\nTop 10 Target Genes:")
        for gene, comp, lit in cur.fetchall():
            print(f"  {gene}: {comp:.3f} (literature: {lit:.3f})")

    conn.close()
    print("\nDone computing target scores")

if __name__ == "__main__":
    main()
