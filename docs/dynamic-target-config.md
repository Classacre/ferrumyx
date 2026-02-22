# Ferrumyx Dynamic Target Configuration

## Concept

Instead of hardcoding "KRAS G12D in Pancreatic Cancer", users define their own research targets via:
1. Web GUI form
2. YAML/JSON config file
3. API request

## Target Configuration Schema

```yaml
# target_config.yaml
target:
  gene: "KRAS"
  mutation: "G12D"                    # Optional
  cancer_type: "Pancreatic Adenocarcinoma"
  cancer_code: "PAAD"                 # DepMap code
  
  # Alternative: multiple genes (synthetic lethality)
  genes:
    - gene: "KRAS"
      mutation: "G12D"
    - gene: "STK11"
      mutation: null

  # Alternative: pathway-based
  pathway: "MAPK signaling"
  
constraints:
  # Druggability requirements
  min_druggability_score: 0.5
  require_structure: true             # Must have PDB/AlphaFold
  
  # Dependency evidence
  min_ceres_score: -0.5               # DepMap threshold
  min_cancer_specificity: 0.3         # How cancer-specific
  
  # Literature evidence
  min_papers: 10
  max_publication_age_years: 5
  
  # Clinical status
  exclude_clinical_stage: ["Phase 3", "Approved"]
  
data_sources:
  # Enable/disable sources
  pubmed: true
  europe_pmc: true
  biorxiv: true
  clinical_trials: true
  depmap: true
  cosmic: true
  chembl: true
  
  # Custom sources (user-provided)
  custom:
    - name: "Internal_Data"
      type: "csv"
      path: "/data/internal_mutations.csv"
      mapping:
        gene_column: "Gene_Symbol"
        mutation_column: "Protein_Change"
        cancer_column: "Cancer_Type"

scoring:
  # Component weights (user-customizable)
  weights:
    crispr_dependency: 0.25
    mutation_frequency: 0.15
    expression_dysregulation: 0.15
    literature_evidence: 0.15
    druggability: 0.15
    pathway_position: 0.10
    clinical_status: 0.05
    
  # Custom scoring functions (advanced)
  custom_scorer: null                 # Path to Rust plugin or WASM

output:
  format: "json"                      # json, csv, html report
  top_n: 50
  include_evidence: true              # Link to papers, data sources
  generate_report: true
  
execution:
  mode: "full"                        # full, quick, custom
  quick_mode_sources: ["pubmed", "depmap"]
  max_runtime_minutes: 60
  parallel_workers: 4
```

## Web GUI Mockup

```
┌─────────────────────────────────────────────────────────────────┐
│  🎯 New Target Investigation                                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Gene Symbol:        [KRAS        ▼]                            │
│  Mutation (opt):     [G12D           ]                          │
│  Cancer Type:        [Pancreatic... ▼]                          │
│                                                                  │
│  ─── Or use Pathway Mode ────                                   │
│  Pathway:            [MAPK signaling ▼]                         │
│                                                                  │
│  ─── Constraints ─────────────                                  │
│  ☑ Require druggable structure                                  │
│  ☑ Minimum 10 supporting papers                                 │
│  ☐ Exclude targets in Phase 3+                                  │
│  Min DepMap CERES:   [-0.5      ] (more negative = more essential)│
│                                                                  │
│  ─── Data Sources ─────────────                                 │
│  ☑ PubMed    ☑ Europe PMC    ☑ bioRxiv                         │
│  ☑ DepMap    ☑ COSMIC        ☑ ChEMBL                          │
│  ☐ ClinicalTrials.gov                                            │
│                                                                  │
│  ─── Output ─────────────────                                   │
│  Top results:        [50        ]                               │
│  ☑ Generate PDF report                                          │
│                                                                  │
│  [Quick Run (5 min)]  [Full Analysis (60 min)]  [Save Config]   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Config File Support
1. Define `TargetConfig` struct with serde
2. Load from YAML/JSON at runtime
3. Pass to ingestion pipeline and scorer
4. Store configs in database for history

### Phase 2: Web GUI
1. Add `/targets/new` endpoint
2. Form → `TargetConfig` → job queue
3. WebSocket for progress updates
4. Results page with export options

### Phase 3: Template Library
1. Pre-defined configs for common targets:
   - "KRAS G12D Pancreatic Cancer"
   - "EGFR Lung Cancer"
   - "BRAF Melanoma"
   - "p53 Reactivation"
2. One-click clone and customize

### Phase 4: Custom Data Sources
1. CSV/JSON upload endpoint
2. Schema mapping UI
3. Validation and preview
4. Merge with public sources

## Architecture Benefits

```
User Config (YAML/Web)
        ↓
    TargetConfig
        ↓
┌───────────────────────────────────┐
│         IronClaw Agent            │
│  ┌─────────────────────────────┐  │
│  │ Dynamic Tool Selection      │  │
│  │ - Enabled sources only      │  │
│  │ - Custom data loaders       │  │
│  │ - Configurable weights      │  │
│  └─────────────────────────────┘  │
│                ↓                   │
│  ┌─────────────────────────────┐  │
│  │ Adaptive Scoring Engine     │  │
│  │ - Component weights from    │  │
│  │   config                    │  │
│  │ - Custom scorers (WASM)     │  │
│  └─────────────────────────────┘  │
└───────────────────────────────────┘
        ↓
    Ranked Results
```

## Database Reuse Strategy

Instead of re-ingesting for each target:

1. **Global Knowledge Graph** — Store ALL papers, entities, facts
2. **Target-Specific Views** — Filter by gene/cancer when scoring
3. **Incremental Updates** — Only fetch new papers since last run
4. **Cached Embeddings** — Reuse vectors across targets

```sql
-- Papers are stored once, queried many times
SELECT * FROM papers 
WHERE to_tsvector(content) @@ plainto_tsquery('KRAS G12D pancreatic');

-- Entity mentions link papers to targets
SELECT p.* FROM papers p
JOIN entity_mentions em ON p.id = em.paper_id
JOIN entities e ON em.entity_id = e.id
WHERE e.canonical_id = 'KRAS' AND e.entity_type = 'gene';
```

## Next Steps

1. Create `TargetConfig` struct in `ferrumyx-common`
2. Add YAML parsing to agent
3. Update ingestion pipeline to use config
4. Add web API endpoint for config submission
5. Build web GUI form

This makes Ferrumyx a **configurable research engine** rather than a single-target tool.
