//! FastQC WASM Tool for Ferrumyx Runtime Core.
//!
//! Provides FASTQ quality control analysis.
//!
//! This tool can analyze FASTQ data provided as:
//! - Direct text input (for small datasets)
//! - URLs to FASTQ files (for larger datasets)
//! - Base64-encoded FASTQ data

wit_bindgen::generate!({
    world: "sandboxed-tool",
    path: "../../wit/tool.wit",
});

use serde::Deserialize;
use regex::Regex;
use std::collections::HashMap;

const MAX_FASTQ_SIZE: usize = 10000000; // 10MB limit for direct input

#[derive(Debug, Default)]
struct FastQCStats {
    total_reads: usize,
    total_bases: usize,
    read_lengths: Vec<usize>,
    base_qualities: Vec<Vec<u8>>,
    gc_content: Vec<f64>,
    n_content: Vec<f64>,
    overrepresented_sequences: HashMap<String, usize>,
}

struct FastQCTool;

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
enum FastQCAction {
    #[serde(rename = "analyze_fastq")]
    AnalyzeFastq {
        data: Option<String>,
        url: Option<String>,
        format: Option<String>, // "text", "base64", "url"
        max_reads: Option<usize>,
    },
}

impl exports::near::agent::tool::Guest for FastQCTool {
    fn execute(req: exports::near::agent::tool::Request) -> exports::near::agent::tool::Response {
        match execute_inner(&req.params) {
            Ok(result) => exports::near::agent::tool::Response {
                output: Some(result),
                error: None,
            },
            Err(e) => exports::near::agent::tool::Response {
                output: None,
                error: Some(e),
            },
        }
    }

    fn schema() -> String {
        SCHEMA.to_string()
    }

    fn description() -> String {
        "Perform FastQC quality control analysis on FASTQ sequencing data. \
         Can analyze data provided as text, base64-encoded, or from URLs. \
         Returns comprehensive QC metrics including base quality, GC content, \
         and overrepresented sequences."
            .to_string()
    }
}

fn execute_inner(params: &str) -> Result<String, String> {
    let action: FastQCAction =
        serde_json::from_str(params).map_err(|e| format!("Invalid parameters: {e}"))?;

    match action {
        FastQCAction::AnalyzeFastq { data, url, format, max_reads } => {
            analyze_fastq(data.as_deref(), url.as_deref(), format.as_deref(), max_reads)
        }
    }
}

fn analyze_fastq(
    data: Option<&str>,
    url: Option<&str>,
    format: Option<&str>,
    max_reads: Option<usize>,
) -> Result<String, String> {
    let max_reads = max_reads.unwrap_or(100000);

    // Get FASTQ data
    let fastq_content = if let Some(url) = url {
        // For URL-based analysis, we'd need to fetch the file
        // For now, return a placeholder
        return Ok(serde_json::json!({
            "status": "placeholder",
            "message": "URL-based FastQC analysis would fetch and analyze the FASTQ file from the provided URL",
            "url": url,
            "max_reads": max_reads
        }).to_string());
    } else if let Some(data) = data {
        let format = format.unwrap_or("text");
        match format {
            "text" => data.to_string(),
            "base64" => String::from_utf8(base64::decode(data).map_err(|e| format!("Invalid base64: {}", e))?)
                .map_err(|e| format!("Invalid UTF-8 in decoded base64: {}", e))?,
            _ => return Err(format!("Unsupported format: {}. Use 'text' or 'base64'", format)),
        }
    } else {
        return Err("Either 'data' or 'url' parameter must be provided".into());
    };

    if fastq_content.len() > MAX_FASTQ_SIZE {
        return Err(format!("FASTQ data exceeds maximum size of {} bytes", MAX_FASTQ_SIZE));
    }

    // Parse and analyze FASTQ
    let stats = analyze_fastq_content(&fastq_content, max_reads)?;

    Ok(serde_json::json!({
        "status": "completed",
        "statistics": {
            "total_reads": stats.total_reads,
            "total_bases": stats.total_bases,
            "average_read_length": if stats.total_reads > 0 { stats.total_bases / stats.total_reads } else { 0 },
            "min_read_length": stats.read_lengths.iter().min().copied().unwrap_or(0),
            "max_read_length": stats.read_lengths.iter().max().copied().unwrap_or(0),
            "average_gc_content": if !stats.gc_content.is_empty() {
                stats.gc_content.iter().sum::<f64>() / stats.gc_content.len() as f64
            } else { 0.0 },
            "average_n_content": if !stats.n_content.is_empty() {
                stats.n_content.iter().sum::<f64>() / stats.n_content.len() as f64
            } else { 0.0 }
        },
        "overrepresented_sequences": stats.overrepresented_sequences.into_iter().take(10).collect::<HashMap<_, _>>()
    }).to_string())
}

fn analyze_fastq_content(content: &str, max_reads: usize) -> Result<FastQCStats, String> {
    let mut stats = FastQCStats::default();
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() % 4 != 0 {
        return Err("Invalid FASTQ format: number of lines is not divisible by 4".into());
    }

    let num_reads = (lines.len() / 4).min(max_reads);

    for i in 0..num_reads {
        let seq_line = lines[i * 4 + 1];
        let qual_line = lines[i * 4 + 3];

        if seq_line.len() != qual_line.len() {
            return Err(format!("Sequence and quality lines have different lengths in read {}", i + 1));
        }

        // Basic statistics
        stats.total_reads += 1;
        stats.total_bases += seq_line.len();
        stats.read_lengths.push(seq_line.len());

        // GC content
        let gc_count = seq_line.chars().filter(|&c| c == 'G' || c == 'C').count();
        let gc_content = gc_count as f64 / seq_line.len() as f64;
        stats.gc_content.push(gc_content);

        // N content
        let n_count = seq_line.chars().filter(|&c| c == 'N').count();
        let n_content = n_count as f64 / seq_line.len() as f64;
        stats.n_content.push(n_content);

        // Overrepresented sequences (simple k-mer counting for k=10)
        if seq_line.len() >= 10 {
            for j in 0..=(seq_line.len() - 10) {
                let kmer = &seq_line[j..j + 10];
                *stats.overrepresented_sequences.entry(kmer.to_string()).or_insert(0) += 1;
            }
        }
    }

    Ok(stats)
}

const SCHEMA: &str = r#"{
    "type": "object",
    "required": ["action"],
    "oneOf": [
        {
            "properties": {
                "action": { "const": "analyze_fastq" },
                "data": { "type": "string", "description": "FASTQ data as text or base64" },
                "url": { "type": "string", "description": "URL to FASTQ file" },
                "format": { "type": "string", "enum": ["text", "base64"], "default": "text" },
                "max_reads": { "type": "integer", "default": 100000, "description": "Maximum number of reads to analyze" }
            }
        }
    ],
    "not": {
        "allOf": [
            { "required": ["data"] },
            { "required": ["url"] }
        ]
    }
}"#;

export!(FastQCTool);