#!/usr/bin/env python3
"""
Recalculate composite scores with CRISPR dependency.
"""

import os
import psycopg2

DB_URL = os.environ.get(
    "DATABASE_URL",
    "postgresql://ferrumyx:ferrumyx_dev@localhost:5432/ferrumyx"
)

def get_connection():
    return psycopg2.connect(DB_URL)

def recalculate_scores(conn):
    """Recalculate composite scores with all available components."""
    print("Recalculating composite scores...")

    with conn.cursor() as cur:
        # Update composite score using all available components
        # Formula: weighted average of available scores
        cur.execute("""
            UPDATE target_scores
            SET composite_score = (
                COALESCE(literature_score, 0) * 0.3 +
                COALESCE(ABS(crispr_dependency_score), 0) * 0.4 +
                COALESCE(mutation_frequency_score, 0) * 0.3
            ) / (
                0.3 + 
                CASE WHEN crispr_dependency_score IS NOT NULL THEN 0.4 ELSE 0 END +
                CASE WHEN mutation_frequency_score IS NOT NULL THEN 0.3 ELSE 0 END
            ),
            updated_at = NOW()
            WHERE literature_score IS NOT NULL 
               OR crispr_dependency_score IS NOT NULL
               OR mutation_frequency_score IS NOT NULL
        """)

        conn.commit()

    print("Composite scores updated")

def main():
    conn = get_connection()
    recalculate_scores(conn)

    # Show updated rankings
    with conn.cursor() as cur:
        cur.execute("""
            SELECT gene, composite_score, literature_score, crispr_dependency_score
            FROM target_scores
            WHERE composite_score IS NOT NULL
            ORDER BY composite_score DESC
            LIMIT 15
        """)
        print("\nUpdated Top 15 Targets:")
        print(f"{'Gene':<12} {'Composite':<12} {'Literature':<12} {'CRISPR Dep':<12}")
        print("-" * 50)
        for gene, comp, lit, crispr in cur.fetchall():
            crispr_str = f"{crispr:.3f}" if crispr else "N/A"
            print(f"{gene:<12} {comp:.3f}       {lit:.3f if lit else 0:.3f}       {crispr_str}")

    conn.close()

if __name__ == "__main__":
    main()
