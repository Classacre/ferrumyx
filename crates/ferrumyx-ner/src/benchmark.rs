//! Performance benchmarking utilities for NER models.
//!
//! This module provides tools to measure and profile NER model performance,
//! including throughput metrics and latency statistics.
//!
//! # Example
//!
//! ```rust,no_run
//! use ferrumyx_ner::{NerModel, NerConfig, benchmark::Benchmark};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let model = NerModel::new(NerConfig::diseases()).await?;
//!     
//!     let texts = vec![
//!         "Patient has diabetes mellitus.",
//!         "The patient was diagnosed with lung cancer.",
//!     ];
//!     
//!     let results = Benchmark::new(&model)
//!         .warmup(5)
//!         .measure(&texts, 100)?;
//!     
//!     println!("{}", results.report());
//!     Ok(())
//! }
//! ```

use std::time::{Duration, Instant};

use crate::{NerModel, NerEntity, Result};

/// Benchmark results for NER model performance.
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    /// Number of texts processed
    pub num_texts: usize,
    /// Number of iterations
    pub iterations: usize,
    /// Total processing time
    pub total_time: Duration,
    /// Average time per text
    pub avg_time_per_text: Duration,
    /// Throughput in texts per second
    pub throughput_texts_per_sec: f64,
    /// Total entities extracted
    pub total_entities: usize,
    /// Average entities per text
    pub avg_entities_per_text: f64,
    /// Whether batch processing was used
    pub used_batch: bool,
    /// Batch size (if applicable)
    pub batch_size: usize,
}

impl BenchmarkResults {
    /// Generate a human-readable report.
    pub fn report(&self) -> String {
        let mut report = String::new();
        report.push_str("\n=== NER Performance Benchmark ===\n\n");
        report.push_str(&format!("Mode: {}\n", if self.used_batch { "Batch" } else { "Sequential" }));
        if self.used_batch {
            report.push_str(&format!("Batch size: {}\n", self.batch_size));
        }
        report.push_str(&format!("Texts processed: {}\n", self.num_texts));
        report.push_str(&format!("Iterations: {}\n", self.iterations));
        report.push_str(&format!("Total time: {:.2?}\n", self.total_time));
        report.push_str(&format!("Avg time per text: {:.2?}\n", self.avg_time_per_text));
        report.push_str(&format!("Throughput: {:.1} texts/sec\n", self.throughput_texts_per_sec));
        report.push_str(&format!("Total entities: {}\n", self.total_entities));
        report.push_str(&format!("Avg entities/text: {:.2}\n", self.avg_entities_per_text));
        report.push('\n');
        report
    }
}

/// Benchmark runner for NER models.
pub struct Benchmark<'a> {
    model: &'a NerModel,
    warmup_iterations: usize,
}

impl<'a> Benchmark<'a> {
    /// Create a new benchmark runner for the given model.
    pub fn new(model: &'a NerModel) -> Self {
        Self {
            model,
            warmup_iterations: 3,
        }
    }
    
    /// Set the number of warmup iterations before measuring.
    pub fn warmup(mut self, iterations: usize) -> Self {
        self.warmup_iterations = iterations;
        self
    }
    
    /// Measure sequential processing performance.
    ///
    /// Processes each text individually in a loop.
    pub fn measure_sequential(&self, texts: &[&str], iterations: usize) -> Result<BenchmarkResults> {
        // Warmup
        for _ in 0..self.warmup_iterations {
            for text in texts {
                let _ = self.model.extract(text)?;
            }
        }
        
        // Measure
        let start = Instant::now();
        let mut total_entities = 0;
        
        for _ in 0..iterations {
            for text in texts {
                let entities = self.model.extract(text)?;
                total_entities += entities.len();
            }
        }
        
        let total_time = start.elapsed();
        let total_texts = texts.len() * iterations;
        
        Ok(BenchmarkResults {
            num_texts: total_texts,
            iterations,
            total_time,
            avg_time_per_text: total_time / total_texts as u32,
            throughput_texts_per_sec: total_texts as f64 / total_time.as_secs_f64(),
            total_entities,
            avg_entities_per_text: total_entities as f64 / total_texts as f64,
            used_batch: false,
            batch_size: 1,
        })
    }
    
    /// Measure batch processing performance.
    ///
    /// Processes all texts in a single batch per iteration.
    pub fn measure_batch(&self, texts: &[&str], iterations: usize) -> Result<BenchmarkResults> {
        // Warmup
        for _ in 0..self.warmup_iterations {
            let _ = self.model.extract_batch(texts)?;
        }
        
        // Measure
        let start = Instant::now();
        let mut total_entities = 0;
        
        for _ in 0..iterations {
            let batch_results = self.model.extract_batch(texts)?;
            for entities in batch_results {
                total_entities += entities.len();
            }
        }
        
        let total_time = start.elapsed();
        let total_texts = texts.len() * iterations;
        
        Ok(BenchmarkResults {
            num_texts: total_texts,
            iterations,
            total_time,
            avg_time_per_text: total_time / total_texts as u32,
            throughput_texts_per_sec: total_texts as f64 / total_time.as_secs_f64(),
            total_entities,
            avg_entities_per_text: total_entities as f64 / total_texts as f64,
            used_batch: true,
            batch_size: texts.len(),
        })
    }
    
    /// Measure both sequential and batch processing and compare.
    pub fn compare(&self, texts: &[&str], iterations: usize) -> Result<(BenchmarkResults, BenchmarkResults)> {
        println!("Running sequential benchmark...");
        let sequential = self.measure_sequential(texts, iterations)?;
        
        println!("Running batch benchmark...");
        let batch = self.measure_batch(texts, iterations)?;
        
        Ok((sequential, batch))
    }
}

/// Quick benchmark function for simple use cases.
pub fn quick_benchmark(model: &NerModel, texts: &[&str]) -> Result<BenchmarkResults> {
    Benchmark::new(model).measure_batch(texts, 10)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_benchmark_results_report() {
        let results = BenchmarkResults {
            num_texts: 100,
            iterations: 10,
            total_time: Duration::from_secs(1),
            avg_time_per_text: Duration::from_millis(10),
            throughput_texts_per_sec: 100.0,
            total_entities: 50,
            avg_entities_per_text: 0.5,
            used_batch: true,
            batch_size: 10,
        };
        
        let report = results.report();
        assert!(report.contains("Batch"));
        assert!(report.contains("100.0 texts/sec"));
    }
}
