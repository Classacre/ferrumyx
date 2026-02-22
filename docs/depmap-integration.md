# DepMap Integration Design

**Goal:** Wire DepMap CRISPR dependency data into the target scoring pipeline.

**Status:** Design phase — 2026-02-22

---

## Overview

DepMap (Cancer Dependency Map) provides CRISPR-Cas9 gene dependency scores (CERES) that quantify how essential each gene is for cancer cell survival. This is **Component 2** of the target scoring model with weight **0.18** (second highest after mutation frequency).

### CERES Score Interpretation

| CERES Score | Interpretation |
|-------------|----------------|
| < -1.0 | Strongly essential (knockout kills cell) |
| -1.0 to -0.5 | Moderately essential |
| -0.5 to 0 | Weak dependency |
| > 0 | Not essential / proliferation advantage |

**Normalization:** More negative = more essential = higher component score.
```
n2 = 1.0 - ((ceres_score + 2.0) / 2.0)
```
So CERES -2.0 → n2 = 1.0 (max), CERES 0.0 → n2 = 0.0 (min).

---

## Data Access Strategy

DepMap provides two access methods:

### Option A: Bulk Download (Preferred for MVP)

- **URL:** `https://depmap.org/portal/download/all/`
- **File:** `CRISPR_gene_effect.csv` (~500MB, updated quarterly)
- **Format:** CSV with genes as columns, cell lines as rows
- **License:** CC BY 4.0
- **Pros:** No API key needed, fast local queries, works offline
- **Cons:** Need to refresh quarterly, ~500MB storage

### Option B: DepMap API

- **Endpoint:** `https://depmap.org/portal/api/`
- **Requires:** API key (free academic registration)
- **Pros:** Always up-to-date
- **Cons:** Rate limits, requires network, slower for bulk queries

**Decision:** Start with bulk download for MVP. Add API client for real-time updates later.

---

## Implementation Plan

### Phase 1: Bulk Data Loader (This Session)

1. **Download and cache** `CRISPR_gene_effect.csv`
2. **Parse CSV** into memory-efficient structure
3. **Map cell lines** to OncoTree cancer types (using DepMap's `Model.csv`)
4. **Expose query interface:**
   - `get_gene_dependency(gene, cancer_type) -> Vec<CERES>`
   - `get_cancer_dependencies(cancer_type, top_n) -> Vec<(Gene, CERES)>`

### Phase 2: Scoring Integration

1. **Wire into `ComponentScoresRaw`** in `ferrumyx-ranker`
2. **Compute normalized score** using `normalise_ceres()`
3. **Handle missing data:**
   - If gene not in DepMap → `crispr_dependency = None` → excluded from weighted sum
   - Renormalize remaining components

### Phase 3: PostgreSQL Cache (Optional)

1. **Create `depmap_gene_effect` table** for persistent cache
2. **Load bulk data on first run** → store in DB
3. **Query from DB** instead of CSV for faster access

---

## File Structure

```
crates/ferrumyx-ingestion/src/sources/
├── depmap.rs              # Existing stub → expand with real implementation
└── depmap_cache.rs        # NEW: Local cache manager

crates/ferrumyx-ranker/src/
├── depmap_provider.rs     # NEW: Trait for dependency data access
└── scorer.rs              # UPDATE: Wire in crispr_dependency component

data/
└── depmap/
    ├── CRISPR_gene_effect.csv      # Downloaded bulk file
    └── Model.csv                   # Cell line → cancer type mapping
```

---

## API Design

### DepMapCache (Bulk Data Manager)

```rust
pub struct DepMapCache {
    gene_effects: HashMap<String, HashMap<String, f64>>,  // gene -> cell_line -> CERES
    cell_line_cancers: HashMap<String, String>,           // cell_line -> oncotree_code
    loaded_at: DateTime<Utc>,
}

impl DepMapCache {
    /// Load from CSV files, downloading if needed
    pub async fn load_or_download(data_dir: &Path) -> Result<Self>;
    
    /// Get CERES scores for a gene across all cell lines of a cancer type
    pub fn get_gene_scores(&self, gene: &str, cancer_type: &str) -> Vec<f64>;
    
    /// Get mean CERES for a gene in a cancer type (aggregated across cell lines)
    pub fn get_mean_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64>;
    
    /// Get top N dependencies for a cancer type
    pub fn get_top_dependencies(&self, cancer_type: &str, n: usize) -> Vec<(String, f64)>;
}
```

### DepMapProvider Trait (for Ranker)

```rust
/// Trait for dependency data access (allows mocking in tests)
pub trait DepMapProvider: Send + Sync {
    fn get_mean_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64>;
    fn get_top_dependencies(&self, cancer_type: &str, n: usize) -> Vec<(String, f64)>;
}
```

---

## Integration Points

### 1. Ingestion Pipeline (Optional)

Not needed — DepMap is not a literature source. The existing `LiteratureSource` impl returns empty.

### 2. Ranker Pipeline

```rust
// In scorer.rs
pub fn compute_component_scores(
    gene: &str,
    cancer_type: &str,
    depmap: &dyn DepMapProvider,
    // ... other data sources
) -> ComponentScoresRaw {
    let crispr_dependency = depmap.get_mean_ceres(gene, cancer_type);
    
    ComponentScoresRaw {
        crispr_dependency,
        // ... other components
    }
}
```

### 3. Normalization

Already implemented in `normalise.rs`:

```rust
pub fn normalise_ceres(ceres_score: f64) -> f64 {
    let clamped = ceres_score.clamp(-2.0, 0.0);
    let norm = minmax_normalise(clamped, -2.0, 0.0);
    1.0 - norm  // invert: more essential → higher score
}
```

---

## Missing Data Handling

DepMap covers ~20,000 genes across ~1,000 cell lines. Not all gene-cancer pairs have data.

**Strategy:**

1. **If CERES available:** Compute normalized score, include in weighted sum
2. **If CERES missing:** Set `crispr_dependency = None`, renormalize weights:
   ```rust
   let available_weights: Vec<f64> = weights.iter()
       .zip(component_scores.iter())
       .filter(|(_, c)| c.is_some())
       .map(|(w, _)| *w)
       .collect();
   let weight_sum: f64 = available_weights.iter().sum();
   // Normalize by weight_sum instead of 1.0
   ```

3. **Confidence penalty:** If CRISPR data missing, reduce overall confidence:
   ```
   confidence *= 0.85  // Missing key component
   ```

---

## Test Plan

1. **Unit tests:**
   - CSV parsing
   - Cell line → cancer type mapping
   - Mean CERES calculation
   - Top N dependencies

2. **Integration tests:**
   - Load sample CSV (subset)
   - Query known gene-cancer pairs
   - Verify normalization

3. **End-to-end:**
   - Run scorer with DepMap data
   - Compare with manual calculation
   - Verify ranking changes

---

## Next Steps

1. ✅ Design document (this file)
2. ⬜ Implement `DepMapCache` with CSV parsing
3. ⬜ Add download helper for bulk files
4. ⬜ Wire into `scorer.rs`
5. ⬜ Add tests
6. ⬜ Update ARCHITECTURE.md with implementation notes

---

## References

- DepMap Portal: https://depmap.org/portal/
- API Docs: https://depmap.org/portal/api/
- CERES Paper: https://doi.org/10.1038/s41588-019-0374-4 (Meyers et al., 2019)
- ARCHITECTURE.md §4.3: Data Sources Per Component
