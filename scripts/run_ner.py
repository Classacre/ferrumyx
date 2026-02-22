#!/usr/bin/env python3
"""
Run NER extraction on all unprocessed papers.
Populates entities and kg_facts tables.
"""

import os
import sys
import requests
import psycopg2
from psycopg2.extras import execute_values

DB_URL = os.environ.get(
    "DATABASE_URL",
    "postgresql://ferrumyx:ferrumyx_dev@localhost:5432/ferrumyx"
)
NER_URL = os.environ.get("NER_URL", "http://localhost:8001")

def get_connection():
    return psycopg2.connect(DB_URL)

def get_unprocessed_papers(conn, limit=100):
    """Get papers that haven't had NER run yet."""
    with conn.cursor() as cur:
        cur.execute("""
            SELECT p.id, p.abstract_text, p.title
            FROM papers p
            WHERE p.abstract_text IS NOT NULL
            AND NOT EXISTS (
                SELECT 1 FROM entities e WHERE e.paper_id = p.id
            )
            ORDER BY p.ingested_at DESC
            LIMIT %s
        """, (limit,))
        return cur.fetchall()

def extract_entities(text):
    """Call NER service."""
    try:
        resp = requests.post(
            f"{NER_URL}/extract",
            json={"text": text},
            timeout=30
        )
        resp.raise_for_status()
        return resp.json().get("entities", [])
    except Exception as e:
        print(f"NER error: {e}", file=sys.stderr)
        return []

def insert_entities(conn, paper_id, entities):
    """Insert entities into database."""
    if not entities:
        return

    with conn.cursor() as cur:
        for ent in entities:
            # Insert into entities table
            cur.execute("""
                INSERT INTO entities (paper_id, entity_type, text, start_pos, end_pos, source)
                VALUES (%s, %s, %s, %s, %s, %s)
                ON CONFLICT DO NOTHING
            """, (
                paper_id,
                ent.get("type", "UNKNOWN"),
                ent.get("text", ""),
                ent.get("start"),
                ent.get("end"),
                "scispacy"
            ))

            # If it's a gene, insert into ent_genes
            if ent.get("type") in ["GENE", "GENE_OR_GENE_PRODUCT"]:
                cur.execute("""
                    INSERT INTO ent_genes (paper_id, gene_symbol, mention_text, confidence)
                    VALUES (%s, %s, %s, %s)
                    ON CONFLICT DO NOTHING
                """, (
                    paper_id,
                    ent.get("text", "").upper(),
                    ent.get("text", ""),
                    ent.get("score", 1.0)
                ))

    conn.commit()

def main():
    print("Starting NER extraction...")
    conn = get_connection()

    batch_size = int(os.environ.get("BATCH_SIZE", 50))
    total = 0

    while True:
        papers = get_unprocessed_papers(conn, limit=batch_size)
        if not papers:
            print("No more papers to process.")
            break

        for paper_id, abstract, title in papers:
            text = f"{title or ''}\n\n{abstract or ''}"
            entities = extract_entities(text)
            insert_entities(conn, paper_id, entities)
            total += 1
            print(f"Processed {total} papers: {paper_id} ({len(entities)} entities)")

    conn.close()
    print(f"Done. Total papers processed: {total}")

if __name__ == "__main__":
    main()
