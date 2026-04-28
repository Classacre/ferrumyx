//! BioClaw-inspired PubMed Literature Search Tool
//! Searches PubMed for oncology-related research papers.

use ferrumyx_runtime::{Tool, ToolError, ToolOutput, Value};
use std::sync::Arc;

pub struct PubMedSearchTool {
    db: Arc<ferrumyx_db::Database>,
    api_key: Option<String>,
}

impl PubMedSearchTool {
    pub fn new(db: Arc<ferrumyx_db::Database>) -> Self {
        let api_key = std::env::var("PUBMED_API_KEY").ok();
        Self { db, api_key }
    }
}

    #[tracing::instrument(skip(self, params))]
    async fn search_pubmed(&self, query: &str, max_results: usize) -> Result<Vec<PubMedResult>, ToolError> {
        let base_url = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi";
        let api_key_param = self.api_key.as_ref().map(|k| format!("&api_key={}", k)).unwrap_or_default();
        
        let url = format!(
            "{}?db=pubmed&term={}&retmax={}{}&sort=relevance",
            base_url, urlencoding::encode(query), max_results, api_key_param
        );

        let client = reqwest::Client::new();
        let response = client.get(&url).send().await.map_err(|e| {
            ToolError::new(format!("PubMed request failed: {}", e))
        })?;

        let text = response.text().await.map_err(|e| {
            ToolError::new(format!("Failed to read response: {}", e))
        })?;

        // Parse XML response (simplified)
        let mut results = Vec::new();
        for line in text.lines() {
            if line.contains("<Id>") {
                if let Some(id) = line.trim().strip_prefix("<Id>").and_then(|s| s.strip_suffix("</Id>")) {
                    results.push(PubMedResult {
                        pmid: id.to_string(),
                        title: "".to_string(),
                        abstract_text: "".to_string(),
                    });
                }
            }
        }

        // Fetch summaries for each ID
        if !results.is_empty() {
            let ids: Vec<_> = results.iter().map(|r| r.pmid.clone()).collect();
            let id_list = ids.join(",");
            let summary_url = format!(
                "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&id={}{}",
                id_list, api_key_param
            );
            
            if let Ok(resp) = client.get(&summary_url).send().await {
                if let Ok(text) = resp.text().await {
                    for (i, line) in text.lines().enumerate() {
                        if line.contains("<Title>") {
                            if let Some(title) = line.trim().strip_prefix("<Title>").and_then(|s| s.strip_suffix("</Title>")) {
                                if i < results.len() {
                                    results[i].title = title.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}

#[async_trait::async_trait]
impl Tool for PubMedSearchTool {
    fn name(&self) -> &str {
        "pubmed_search"
    }

    fn description(&self) -> &str {
        "Search PubMed for oncology-related research papers. Use for literature review and target validation."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string", 
                    "description": "Search query (e.g., 'EGFR lung cancer oncology')"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results (default 10)",
                    "default": 10
                },
                "focus": {
                    "type": "string", 
                    "enum": ["genes", "drugs", "pathways", "all"],
                    "description": "Research focus",
                    "default": "all"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value, _ctx: &dyn std::any::Any) -> Result<ToolOutput, ToolError> {
        let query = params.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::new("Missing query parameter"))?;
            
        let max_results = params.get("max_results")
            .and_then(|v| v.as_i64())
            .map(|v| v as usize)
            .unwrap_or(10)
            .clamp(1, 50);

        let focus = params.get("focus")
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        // Enhance query with oncology focus
        let enhanced_query = match focus {
            "genes" => format!("{} gene oncology", query),
            "drugs" => format!("{} drug oncology", query),
            "pathways" => format!("{} pathway oncology", query),
            _ => format!("{} oncology", query),
        };

        let results = self.search_pubmed(&enhanced_query, max_results).await?;

        let summary = if results.is_empty() {
            format!("No results found for: {}", query)
        } else {
            let mut output = format!("Found {} results for '{}':\n\n", results.len(), query);
            for (i, r) in results.iter().enumerate() {
                output += &format!("{}. PMID: {}\n   {}\n\n", i + 1, r.pmid, r.title);
            }
            output
        };

        Ok(ToolOutput::new(summary, true))
    }
}

#[derive(Debug, Clone)]
struct PubMedResult {
    pmid: String,
    title: String,
    abstract_text: String,
}