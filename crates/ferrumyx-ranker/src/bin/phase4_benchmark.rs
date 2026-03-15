use ferrumyx_common::query::QueryRequest;
use ferrumyx_db::Database;
use ferrumyx_ranker::{ProviderRefreshRequest, TargetQueryEngine};
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

fn percentile_ms(samples: &[u64], p: f64) -> Option<u64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let rank = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted.get(rank).copied()
}

fn parse_genes_csv(raw: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    raw.split(',')
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty())
        .filter(|v| seen.insert(v.clone()))
        .collect()
}

fn profile_name() -> String {
    std::env::var("FERRUMYX_BENCH_PHASE4_PROFILE")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            std::env::var("FERRUMYX_PHASE4_BENCH_PROFILE")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
        .unwrap_or_else(|| "balanced".to_string())
        .to_lowercase()
}

fn profile_default_usize(profile: &str, key: &str) -> usize {
    match (profile, key) {
        ("aggressive", "query_runs") => 16,
        ("aggressive", "refresh_runs") => 10,
        ("aggressive", "max_results") => 32,
        ("aggressive", "refresh_batch") => 12,
        ("balanced", "query_runs") => 10,
        ("balanced", "refresh_runs") => 6,
        ("balanced", "max_results") => 25,
        ("balanced", "refresh_batch") => 6,
        ("safe", "query_runs") => 8,
        ("safe", "refresh_runs") => 4,
        ("safe", "max_results") => 15,
        ("safe", "refresh_batch") => 4,
        _ => profile_default_usize("balanced", key),
    }
}

fn profile_default_u8(profile: &str, key: &str) -> u8 {
    match (profile, key) {
        ("aggressive", "refresh_retries") => 2,
        ("balanced", "refresh_retries") => 1,
        ("safe", "refresh_retries") => 1,
        _ => 1,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let profile = profile_name();
    let db_url = std::env::var("FERRUMYX_BENCH_DB_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| std::env::var("FERRUMYX_DB_URL").ok())
        .unwrap_or_else(|| "./data/lancedb".to_string());

    let query_runs = std::env::var("FERRUMYX_PHASE4_BENCH_QUERY_RUNS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| profile_default_usize(&profile, "query_runs"))
        .clamp(1, 200);
    let refresh_runs = std::env::var("FERRUMYX_PHASE4_BENCH_REFRESH_RUNS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| profile_default_usize(&profile, "refresh_runs"))
        .clamp(1, 100);
    let max_results = std::env::var("FERRUMYX_PHASE4_BENCH_MAX_RESULTS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| profile_default_usize(&profile, "max_results"))
        .clamp(1, 200);
    let refresh_batch_size = std::env::var("FERRUMYX_PHASE4_BENCH_REFRESH_BATCH")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| profile_default_usize(&profile, "refresh_batch"))
        .clamp(1, 32);
    let refresh_retries = std::env::var("FERRUMYX_PHASE4_BENCH_REFRESH_RETRIES")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or_else(|| profile_default_u8(&profile, "refresh_retries"))
        .min(3);
    let offline_strict = std::env::var("FERRUMYX_PHASE4_BENCH_OFFLINE_STRICT")
        .ok()
        .or_else(|| std::env::var("FERRUMYX_BENCH_OFFLINE_STRICT").ok())
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let show_provider_timing = std::env::var("FERRUMYX_PHASE4_BENCH_PROVIDER_TIMING")
        .ok()
        .or_else(|| std::env::var("FERRUMYX_BENCH_PROVIDER_TIMING").ok())
        .is_none_or(|v| v == "1" || v.eq_ignore_ascii_case("true"));

    let cancer_code = std::env::var("FERRUMYX_PHASE4_BENCH_CANCER")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "PAAD".to_string())
        .to_uppercase();
    let query_text = std::env::var("FERRUMYX_PHASE4_BENCH_QUERY_TEXT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| format!("phase4 benchmark {}", cancer_code));
    let seed_genes = parse_genes_csv(
        &std::env::var("FERRUMYX_PHASE4_BENCH_GENES")
            .unwrap_or_else(|_| "KRAS,EGFR,BRAF,TP53,PIK3CA".to_string()),
    );

    println!("=== Phase 4 Benchmark ===");
    println!(
        "db={} profile={} query_runs={} refresh_runs={} max_results={} cancer={} offline_strict={}",
        db_url, profile, query_runs, refresh_runs, max_results, cancer_code, offline_strict
    );

    let db = Database::open(&db_url).await?;
    db.initialize().await?;
    let db = Arc::new(db);
    let engine = TargetQueryEngine::new(db);

    let mut query_samples_ms = Vec::new();
    let mut discovered_genes = Vec::new();
    for run_idx in 0..query_runs {
        let started = Instant::now();
        let results = engine
            .execute_query(QueryRequest {
                query_text: query_text.clone(),
                cancer_code: Some(cancer_code.clone()),
                gene_symbol: None,
                mutation: None,
                max_results,
            })
            .await?;
        let elapsed_ms = started.elapsed().as_millis() as u64;
        query_samples_ms.push(elapsed_ms);
        println!(
            "[query {:>2}] {} ms (results={})",
            run_idx + 1,
            elapsed_ms,
            results.len()
        );
        if discovered_genes.is_empty() {
            discovered_genes = results
                .iter()
                .take(24)
                .map(|r| r.gene_symbol.trim().to_uppercase())
                .filter(|g| !g.is_empty())
                .collect();
        }
    }

    if discovered_genes.is_empty() {
        discovered_genes = seed_genes;
    }
    if discovered_genes.is_empty() {
        anyhow::bail!("no genes available for provider refresh benchmark");
    }
    discovered_genes.truncate(24);

    let mut refresh_samples_ms = Vec::new();
    let mut refresh_attempted_total = 0usize;
    let mut refresh_failed_total = 0usize;
    let mut provider_timing_totals_ms: BTreeMap<String, u64> = BTreeMap::new();
    for run_idx in 0..refresh_runs {
        let started = Instant::now();
        let report = engine
            .refresh_provider_signals(ProviderRefreshRequest {
                genes: discovered_genes.clone(),
                cancer_code: Some(cancer_code.clone()),
                max_genes: discovered_genes.len().clamp(1, 200),
                batch_size: refresh_batch_size,
                retries: refresh_retries,
                offline_strict,
            })
            .await?;
        let elapsed_ms = started.elapsed().as_millis() as u64;
        refresh_samples_ms.push(elapsed_ms);

        let attempted = report.cbio_attempted
            + report.cosmic_attempted
            + report.gtex_attempted
            + report.tcga_attempted
            + report.chembl_attempted
            + report.reactome_attempted;
        let failed = report.cbio_failed
            + report.cosmic_failed
            + report.gtex_failed
            + report.tcga_failed
            + report.chembl_failed
            + report.reactome_failed;
        refresh_attempted_total += attempted;
        refresh_failed_total += failed;
        for (provider, ms) in &report.provider_timing_ms {
            *provider_timing_totals_ms
                .entry(provider.clone())
                .or_insert(0) += *ms;
        }
        println!(
            "[refresh {:>2}] {} ms (genes={}, attempted={}, failed={})",
            run_idx + 1,
            elapsed_ms,
            report.genes_processed,
            attempted,
            failed
        );
        if show_provider_timing {
            for (provider, ms) in &report.provider_timing_ms {
                println!("  [provider] {}={}ms", provider, ms);
            }
        }
    }

    let query_p50 = percentile_ms(&query_samples_ms, 0.50).unwrap_or(0);
    let query_p95 = percentile_ms(&query_samples_ms, 0.95).unwrap_or(0);
    let query_avg = if query_samples_ms.is_empty() {
        0.0
    } else {
        query_samples_ms.iter().sum::<u64>() as f64 / query_samples_ms.len() as f64
    };
    let refresh_p50 = percentile_ms(&refresh_samples_ms, 0.50).unwrap_or(0);
    let refresh_p95 = percentile_ms(&refresh_samples_ms, 0.95).unwrap_or(0);
    let refresh_avg = if refresh_samples_ms.is_empty() {
        0.0
    } else {
        refresh_samples_ms.iter().sum::<u64>() as f64 / refresh_samples_ms.len() as f64
    };
    let refresh_error_rate = if refresh_attempted_total == 0 {
        0.0
    } else {
        refresh_failed_total as f64 / refresh_attempted_total as f64
    };

    println!();
    println!("=== Phase 4 Benchmark Summary ===");
    println!(
        "query:   runs={} p50={}ms p95={}ms avg={:.1}ms",
        query_samples_ms.len(),
        query_p50,
        query_p95,
        query_avg
    );
    println!(
        "refresh: runs={} p50={}ms p95={}ms avg={:.1}ms attempted={} failed={} err_rate={:.3}",
        refresh_samples_ms.len(),
        refresh_p50,
        refresh_p95,
        refresh_avg,
        refresh_attempted_total,
        refresh_failed_total,
        refresh_error_rate
    );
    if show_provider_timing {
        println!("provider_timing_ms_total:");
        for (provider, total_ms) in provider_timing_totals_ms {
            let avg_ms = if refresh_runs == 0 {
                0.0
            } else {
                total_ms as f64 / refresh_runs as f64
            };
            println!(
                "  {} total={}ms avg_per_run={:.1}ms",
                provider, total_ms, avg_ms
            );
        }
    }
    println!("genes_benchmarked={}", discovered_genes.join(","));

    Ok(())
}
