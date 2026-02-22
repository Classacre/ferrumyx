#!/usr/bin/env python3
"""
Build knowledge graph facts from extracted entities.
Creates gene-cancer, gene-mutation, gene-pathway relationships.
"""

import os
import re
import psycopg2
from collections import defaultdict

DB_URL = os.environ.get(
    "DATABASE_URL",
    "postgresql://ferrumyx:ferrumyx_dev@localhost:5432/ferrumyx"
)

# Cancer type mappings
CANCER_KEYWORDS = {
    "pancreatic": "PAAD",
    "pancreas": "PAAD",
    "lung": "LUAD",
    "breast": "BRCA",
    "colorectal": "COAD",
    "colon": "COAD",
    "melanoma": "SKCM",
    "glioblastoma": "GBM",
    "brain": "GBM",
    "ovarian": "OV",
    "prostate": "PRAD",
    "liver": "LIHC",
    "hepatocellular": "LIHC",
}

# Mutation patterns
MUTATION_PATTERN = re.compile(r'\b([A-Z]\d+[A-Z])\b', re.IGNORECASE)

def get_connection():
    return psycopg2.connect(DB_URL)

def get_gene_mentions(conn):
    """Get all gene mentions with paper abstracts."""
    with conn.cursor() as cur:
        cur.execute("""
            SELECT eg.id, eg.gene_symbol, eg.paper_id, p.abstract_text, p.title
            FROM ent_genes eg
            JOIN papers p ON p.id = eg.paper_id
            WHERE p.abstract_text IS NOT NULL
        """)
        return cur.fetchall()

def extract_cancer_type(text):
    """Extract cancer type from text."""
    text_lower = text.lower()
    for keyword, cancer_code in CANCER_KEYWORDS.items():
        if keyword in text_lower:
            return cancer_code
    return None

def extract_mutations(text):
    """Extract mutation patterns (e.g., G12D, V600E)."""
    return MUTATION_PATTERN.findall(text.upper())

def build_kg_facts(conn):
    """Build knowledge graph facts."""
    print("Building KG facts...")

    mentions = get_gene_mentions(conn)
    facts = defaultdict(int)

    for mention_id, gene, paper_id, abstract, title in mentions:
        text = f"{title or ''} {abstract or ''}"

        # Gene-Cancer relationship
        cancer = extract_cancer_type(text)
        if cancer:
            facts[("gene_cancer", gene, cancer)] += 1

        # Gene-Mutation relationship
        mutations = extract_mutations(text)
        for mut in mutations:
            facts[("gene_mutation", gene, mut)] += 1

    # Insert facts into kg_facts
    with conn.cursor() as cur:
        for (fact_type, subject, obj), count in facts.items():
            cur.execute("""
                INSERT INTO kg_facts (fact_type, subject, object, evidence_count, source)
                VALUES (%s, %s, %s, %s, %s)
                ON CONFLICT (fact_type, subject, object)
                DO UPDATE SET evidence_count = kg_facts.evidence_count + %s
            """, (fact_type, subject, obj, count, "ner_extraction", count))

        conn.commit()

    print(f"Inserted {len(facts)} facts")

def main():
    conn = get_connection()
    build_kg_facts(conn)
    conn.close()
    print("Done building KG facts")

if __name__ == "__main__":
    main()
