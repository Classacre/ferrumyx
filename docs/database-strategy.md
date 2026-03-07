# Ferrumyx Database Strategy

## Speed Requirements

For a fast oncology research engine, we need:
- **Sub-second vector similarity search** (embeddings)
- **Fast full-text search** (paper content)
- **Efficient joins** (papers → entities → facts)
- **Bulk inserts** (ingestion pipeline)

## Available Databases (Pre-built)

### 1. **PubMed Central Open Access Subset**
- **URL:** https://www.ncbi.nlm.nih.gov/pmc/tools/oai/
- **Size:** ~3.5 million full-text articles
- **Format:** XML (JATS)
- **Free:** Yes
- **Update:** Continuous (add new papers incrementally)

```bash
# Download bulk
wget https://ftp.ncbi.nlm.nih.gov/pub/pmc/oa_bulk/*.tar.gz
```

### 2. **Europe PMC REST API**
- **URL:** https://europepmc.org/RestfulWebService
- **Size:** 40+ million abstracts, 6+ million full-text
- **Rate limit:** None with API key
- **Free:** Yes

### 3. **Semantic Scholar Corpus**
- **URL:** https://www.semanticscholar.org/product/api
- **Size:** 200+ million papers
- **Features:** Pre-computed citations, S2ORC full-text
- **Free:** Yes (with rate limits)

### 4. **DepMap (Dependency Map)**
- **URL:** https://depmap.org/portal/download/all/
- **Data:** CRISPR gene essentiality, RNAi, expression, mutations
- **Size:** ~20,000 genes × ~1,000 cell lines
- **Free:** Yes (academic)

```bash
# Key files
CRISPR_gene_effect.csv      # CERES scores
Model.csv                   # Cell line metadata
OmicsExpressionProteinGenes.csv  # Expression
```

### 5. **COSMIC (Catalogue of Somatic Mutations in Cancer)**
- **URL:** https://cancer.sanger.ac.uk/cosmic/download
- **Data:** 6+ million coding mutations
- **Free:** Academic (requires registration)

### 6. **ChEMBL**
- **URL:** https://www.ebi.ac.uk/chembl/
- **Data:** 2+ million bioactive compounds
- **Free:** Yes

### 7. **AlphaFold Protein Structure Database**
- **URL:** https://alphafold.ebi.ac.uk/
- **Data:** 200+ million protein structures
- **Free:** Yes

## Database Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Lancedb Embedded Vector DB                   │
│                    (Single source of truth)                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │   papers    │  │  entities   │  │      kg_facts           │ │
│  │  (millions) │  │ (genes,     │  │  (relationships)        │ │
│  │             │  │  diseases,  │  │                         │ │
│  │  + FTS      │  │  compounds) │  │  source → target → type │ │
│  │  + vectors  │  │             │  │                         │ │
│  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘ │
│         │                │                     │               │
│         └────────────────┼─────────────────────┘               │
│                          │                                     │
│  ┌───────────────────────▼─────────────────────────────────────┐│
│  │              entity_mentions (link table)                   ││
│  │  paper_id → entity_id + context + confidence                ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │              Pre-computed Indexes                           ││
│  │  • IVF-PQ for vector search (fast similarity)               ││
│  │  • LanceDB native FTS                                       ││
│  │  • Monotonic b-lining for entity lookups                    ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Incremental Update Strategy

Instead of re-building from scratch:

```rust
// Store last sync timestamp
let last_sync = get_last_sync_timestamp("pubmed");

// Only fetch new papers
let query = format!(
    "KRAS[tiab] AND pancreatic cancer[tiab] AND {}[edat]",
    last_sync.format("%Y/%m/%d")
);

// Incremental ingestion
ingest_new_papers(query).await?;

// Update sync timestamp
set_last_sync_timestamp("pubmed", Utc::now());
```

## Performance Optimizations

### 1. **Vector Index (IVF-PQ)**
```rust
// Fast approximate nearest neighbor search via LanceDB
let table = db.open_table("paper_chunks").await?;
let query = table
    .search(query_vector)
    .limit(20)
    .execute()
    .await?;
```

### 2. **Full-Text Search (LanceDB Native)**
```rust
// Combined with vector for hybrid search
let query = table
    .search(query_vector)
    .limit(100)
    .execute()
    .await?;
// RRF (Reciprocal Rank Fusion) logic handles FTS intersection
```

### 3. **Materialized Views for Common Queries**
```sql
-- Pre-computed gene-cancer associations
CREATE MATERIALIZED VIEW gene_cancer_summary AS
SELECT 
    e.canonical_id AS gene,
    ct.name AS cancer_type,
    COUNT(DISTINCT p.id) AS paper_count,
    AVG(kf.confidence) AS avg_confidence
FROM entities e
JOIN entity_mentions em ON e.id = em.entity_id
JOIN papers p ON em.paper_id = p.id
JOIN kg_facts kf ON e.id = kf.subject_id
JOIN ent_cancer_types ct ON kf.object_id = ct.id
GROUP BY e.canonical_id, ct.name;

-- Refresh nightly
REFRESH MATERIALIZED VIEW gene_cancer_summary;
```

### 4. **Connection Pooling**
```rust
// Use PgPool with optimal settings
let pool = PgPoolOptions::new()
    .max_connections(20)           // Concurrent queries
    .min_connections(5)            // Keep warm
    .acquire_timeout(Duration::from_secs(5))
    .connect(&database_url)
    .await?;
```

## Pre-seeding Strategy

### Option A: Start Empty, Grow Organically
- Begin with zero papers
- Ingest based on user queries
- Pros: No wasted storage, targeted data
- Cons: Slow initial results

### Option B: Pre-seed Core Oncology Data
```bash
# 1. Download PMC Open Access oncology subset
# ~500k papers, ~50GB
wget https://ftp.ncbi.nlm.nih.gov/pub/pmc/oa_bulk/PMC_Open_Access_Subset_JATS.zip

# 2. Filter for oncology keywords
grep -l "cancer\|tumor\|oncology\|carcinoma" *.xml | wc -l

# 3. Bulk ingest
cargo run --bin bulk_ingest -- --source pmc --path ./pmc_oncology/
```

### Option C: Hybrid (Recommended)
1. Pre-seed top 1000 oncology genes + top 20 cancer types
2. Estimate: ~100k papers, ~5GB database
3. Incremental updates for user-specific queries

```rust
// Pre-seed script
let seed_genes = ["KRAS", "TP53", "EGFR", "BRAF", "PIK3CA", ...];
let seed_cancers = ["PAAD", "LUAD", "BRCA", "COAD", "GBM", ...];

for gene in seed_genes {
    for cancer in seed_cancers {
        let query = format!("{}[tiab] AND {}[tiab]", gene, cancer);
        ingest_papers(query, max=500).await?;
    }
}
```

## Backup & Recovery

```bash
# Daily backup
pg_dump ferrumyx | gzip > backup_$(date +%Y%m%d).sql.gz

# Point-in-time recovery
pg_restore --clean --dbname=ferrumyx backup_20260222.sql.gz
```

## Next Steps

1. ✅ Database running (LanceDB Embedded)
2. ✅ Schema migrated (LanceDB tables)
3. 🔜 Create bulk ingestion script
4. 🔜 Pre-seed core oncology data (100k papers)
5. 🔜 Set up incremental sync jobs

## Cost Estimate (Cloud)

| Resource | Local | AWS RDS | Neon |
|----------|-------|---------|------|
| 100GB storage | Free | $15/mo | $0 |
| 1M queries/mo | Free | $50/mo | $20/mo |
| Vector search | Free | $30/mo | Included |

**Recommendation:** Start local, scale to Neon (serverless Postgres) for production.
