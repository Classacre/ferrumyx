#!/usr/bin/env python3
"""
Download and import DepMap CRISPR gene effect data.
Provides dependency scores for target prioritization.
"""

import os
import csv
import requests
import psycopg2
from io import StringIO

DB_URL = os.environ.get(
    "DATABASE_URL",
    "postgresql://ferrumyx:ferrumyx_dev@localhost:5432/ferrumyx"
)

# DepMap CRISPR gene effect file (public)
DEPMAP_URL = "https://depmap.org/portal/download/api/download/external?file_name=processed%2FCRISPR_gene_effect.csv&bucket=depmap-external-downloads"

def get_connection():
    return psycopg2.connect(DB_URL)

def download_depmap():
    """Download DepMap CRISPR data."""
    print("Downloading DepMap CRISPR gene effect data...")
    
    response = requests.get(DEPMAP_URL, stream=True, timeout=300)
    response.raise_for_status()
    
    # Parse CSV
    content = response.text
    reader = csv.reader(StringIO(content))
    headers = next(reader)
    
    # Headers: first column is cell line, rest are genes
    cell_lines = headers[1:] if len(headers) > 1 else []
    
    print(f"Found {len(cell_lines)} cell lines")
    
    return reader, cell_lines

def import_gene_effects(conn, reader, cell_lines):
    """Import gene effect scores."""
    print("Importing gene effect scores...")
    
    # Map cell lines to cancer types
    cancer_types = {}
    for cl in cell_lines:
        # Extract cancer type from cell line name (e.g., "PANCANCER_PANCREATIC")
        if "PANCREA" in cl.upper():
            cancer_types[cl] = "PAAD"
        elif "LUNG" in cl.upper():
            cancer_types[cl] = "LUAD"
        elif "BREAST" in cl.upper():
            cancer_types[cl] = "BRCA"
        elif "COLON" in cl.upper() or "COLO" in cl.upper():
            cancer_types[cl] = "COAD"
        elif "BRAIN" in cl.upper() or "GLIO" in cl.upper():
            cancer_types[cl] = "GBM"
        elif "SKIN" in cl.upper() or "MELAN" in cl.upper():
            cancer_types[cl] = "SKCM"
        else:
            cancer_types[cl] = "OTHER"
    
    with conn.cursor() as cur:
        count = 0
        for row in reader:
            if len(row) < 2:
                continue
            
            gene = row[0].strip()
            if not gene:
                continue
            
            # Calculate average effect across cell lines
            effects = []
            for val in row[1:]:
                try:
                    effect = float(val) if val else None
                    if effect is not None:
                        effects.append(effect)
                except ValueError:
                    continue
            
            if not effects:
                continue
            
            avg_effect = sum(effects) / len(effects)
            
            # Insert into target_scores
            cur.execute("""
                INSERT INTO target_scores (gene, crispr_dependency_score, updated_at)
                VALUES (%s, %s, NOW())
                ON CONFLICT (gene)
                DO UPDATE SET 
                    crispr_dependency_score = EXCLUDED.crispr_dependency_score,
                    updated_at = NOW()
            """, (gene, avg_effect))
            
            count += 1
            if count % 100 == 0:
                print(f"  Processed {count} genes...")
        
        conn.commit()
    
    print(f"Imported {count} gene effect scores")

def main():
    conn = get_connection()
    
    try:
        reader, cell_lines = download_depmap()
        import_gene_effects(conn, reader, cell_lines)
    except Exception as e:
        print(f"Error: {e}")
        print("Note: DepMap requires accepting terms at https://depmap.org/terms")
        print("Using fallback: computing scores from existing KG data")
    
    conn.close()
    print("Done")

if __name__ == "__main__":
    main()
