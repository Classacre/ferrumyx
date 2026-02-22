# DepMap Integration Implementation

**Date:** 2026-02-22
**Status:** ✅ Complete

---

## Summary

Successfully implemented DepMap CRISPR dependency data integration for the Ferrumyx target scoring pipeline.

## Changes Made

### 1. New Files

#### `docs/depmap-integration.md`
- Design document with data access strategy
- API design for `DepMapCache` and `DepMapProvider`
- Integration points and test plan

#### `crates/ferrumyx-ingestion/src/sources/depmap_cache.rs`
- `DepMapCache` struct for bulk CSV data loading
- CSV parsers for:
  - `CRISPR_gene_effect.csv` (gene × cell line CERES scores)
  - `Model.csv` (cell line → cancer type mapping)
- Query methods:
  - `get_gene_scores()` - all CERES scores for a gene-cancer pair
  - `get_mean_ceres()` - aggregated mean score
  - `get_median_ceres()` - robust median score
  - `get_top_dependencies()` - ranked gene list for cancer type

#### `crates/ferrumyx-ranker/src/depmap_provider.rs`
- `DepMapProvider` trait for dependency data access
- `MockDepMapProvider` for unit testing
- `DepMapCacheAdapter` for wrapping cache instances

### 2. Modified Files

#### `crates/ferrumyx-ingestion/src/sources/mod.rs`
- Added `depmap_cache` module
- Re-exported `DepMapCache`

#### `crates/ferrumyx-ranker/src/lib.rs`
- Added `depmap_provider` module

#### `crates/ferrumyx-ranker/src/scorer.rs`
- Added imports for `DepMapProvider` and `normalise_ceres`
- Added `compute_crispr_component()` function
- Added `compute_component_scores_with_depmap()` convenience function
- Added unit tests for CRISPR component scoring

## How It Works

### Data Flow

```
DepMap Bulk Download (CSV)
        ↓
   DepMapCache
   (in-memory HashMap)
        ↓
   DepMapProvider trait
        ↓
   compute_crispr_component()
        ↓
   normalise_ceres()
        ↓
   ComponentScoresRaw.crispr_dependency
```

### Score Normalization

CERES scores range from ~-2.0 (maximally essential) to 0.0 (not essential).

Normalization formula (from `normalise.rs`):
```rust
fn normalise_ceres(ceres_score: f64) -> f64 {
    let clamped = ceres_score.clamp(-2.0, 0.0);
    let norm = minmax_normalise(clamped, -2.0, 0.0);
    1.0 - norm  // invert: more essential → higher score
}
```

Examples:
- CERES -2.0 → normalized 1.0 (perfect dependency)
- CERES -1.0 → normalized 0.5 (moderate dependency)
- CERES 0.0 → normalized 0.0 (no dependency)

### Missing Data Handling

If a gene-cancer pair has no DepMap data:
- `ComponentScoresRaw.crispr_dependency = None`
- The weighted sum renormalizes over available components
- A confidence penalty may apply (configurable)

## Usage

### Loading DepMap Data

```rust
use ferrumyx_ingestion::sources::DepMapCache;
use std::path::Path;

let cache = DepMapCache::load_from_dir(Path::new("data/depmap"))?;

// Query gene dependency
let mean_ceres = cache.get_mean_ceres("KRAS", "PAAD");
println!("KRAS mean CERES in PAAD: {:?}", mean_ceres);

// Get top dependencies for a cancer type
let top_deps = cache.get_top_dependencies("PAAD", 20);
for (gene, ceres) in top_deps {
    println!("{}: {:.3}", gene, ceres);
}
```

### In Target Scoring

```rust
use ferrumyx_ranker::{DepMapProvider, compute_crispr_component};

// Using mock for testing
let provider = MockDepMapProvider::new()
    .with("KRAS", "PAAD", -1.2);

let score = compute_crispr_component("KRAS", "PAAD", &provider);
// Returns Some(0.6)

// Or with real cache
let cache = Arc::new(DepMapCache::load_default()?);
let adapter = DepMapCacheAdapter::new(cache);
let score = compute_crispr_component("KRAS", "PAAD", &adapter);
```

## Test Results

All unit tests pass:
- ✅ `test_parse_gene_symbol` - CSV gene column parsing
- ✅ `test_mean_calculation` - Mean aggregation
- ✅ `test_median_calculation_odd/even` - Median calculation
- ✅ `test_mock_provider` - Mock provider functionality
- ✅ `test_adapter` - Provider adapter
- ✅ `test_crispr_component_normalized` - Score normalization
- ✅ `test_crispr_component_missing_gene/cancer` - Missing data handling

## Next Steps

1. **Download DepMap data:**
   - Get `CRISPR_gene_effect.csv` from https://depmap.org/portal/download/all/
   - Get `Model.csv` for cell line metadata
   - Place in `data/depmap/`

2. **Wire into full scoring pipeline:**
   - Update `TargetScorer` to accept `DepMapProvider`
   - Integrate with other component data sources (COSMIC, TCGA, etc.)

3. **Add PostgreSQL cache (optional):**
   - Create `depmap_gene_effect` table
   - Load bulk data on first run
   - Query from DB for faster access

4. **Add API client (optional):**
   - Implement real-time DepMap API queries
   - Use for quarterly updates

## Files Changed

```
docs/depmap-integration.md                 # NEW: Design doc
crates/ferrumyx-ingestion/src/sources/
├── depmap_cache.rs                        # NEW: Bulk data cache
└── mod.rs                                 # MODIFIED: exports
crates/ferrumyx-ranker/src/
├── depmap_provider.rs                     # NEW: Trait + mock
├── lib.rs                                 # MODIFIED: exports
└── scorer.rs                              # MODIFIED: CRISPR scoring
```