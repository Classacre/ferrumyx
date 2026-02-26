use reqwest::{Client, ClientBuilder};
use std::collections::HashSet;
use std::time::Duration;
use url::Url;
use crate::error::FerrumyxError;

/// A Sandbox-capped HTTP Client that only allows requests to approved domains.
/// This implements the IronClaw Sandbox Layer concept for network capability capping.
#[derive(Debug, Clone)]
pub struct SandboxClient {
    client: Client,
    allowlist: HashSet<String>,
}

impl SandboxClient {
    /// Creates a new SandboxClient with a default allowlist of required scientific and AI domains.
    pub fn new() -> Result<Self, FerrumyxError> {
        let mut allowlist = HashSet::new();
        // Default Ferrumyx allowlist
        let domains = vec![
            "eutils.ncbi.nlm.nih.gov", // PubMed
            "www.ebi.ac.uk",           // EuropePMC, ChEMBL, AlphaFold
            "api.biorxiv.org",         // bioRxiv
            "export.arxiv.org",        // arXiv
            "clinicaltrials.gov",      // ClinicalTrials
            "api.crossref.org",        // CrossRef
            "api.semanticscholar.org", // Semantic Scholar
            "api.unpaywall.org",       // Unpaywall
            "rest.genenames.org",      // HGNC
            "oncotree.mskcc.org",      // OncoTree
            "depmap.org",              // DepMap
            "cancer.sanger.ac.uk",     // COSMIC
            "data.rcsb.org",           // PDB
            "models.rcsb.org",         // PDB Models
            "localhost",               // Ollama local
            "127.0.0.1",               // Localhost alt
            "api.openai.com",          // OpenAI LLMs
            "api.anthropic.com",       // Anthropic LLMs
            "huggingface.co",          // HuggingFace Models
            "cdn-lfs.huggingface.co",  // HuggingFace LFS
        ];

        for d in domains {
            allowlist.insert(d.to_string());
        }

        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| FerrumyxError::IngestionError(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self { client, allowlist })
    }

    /// Appends an exact hostname to the allowlist.
    pub fn allow_domain(&mut self, domain: &str) {
        self.allowlist.insert(domain.to_string());
    }

    /// Validates if a URL is permitted under the current sandbox policy.
    pub fn is_allowed(&self, url: &str) -> bool {
        if let Ok(parsed) = Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                // Check exact match or if it's a subdomain of an allowed domain
                for allowed in &self.allowlist {
                    if host == allowed || host.ends_with(&format!(".{}", allowed)) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Exposes the inner `reqwest::Client` builder pattern safely for GET requests.
    pub fn get(&self, url: &str) -> Result<reqwest::RequestBuilder, FerrumyxError> {
        if !self.is_allowed(url) {
            return Err(FerrumyxError::SecurityError(format!(
                "Network capabilities capped: domain not in allowlist for URL {}",
                url
            )));
        }

        Ok(self.client.get(url))
    }

    /// Exposes the inner `reqwest::Client` builder pattern safely for POST requests.
    pub fn post(&self, url: &str) -> Result<reqwest::RequestBuilder, FerrumyxError> {
        if !self.is_allowed(url) {
            return Err(FerrumyxError::SecurityError(format!(
                "Network capabilities capped: domain not in allowlist for URL {}",
                url
            )));
        }

        Ok(self.client.post(url))
    }

    /// Exposes the inner `reqwest::Client` builder pattern safely.
    pub fn request(&self, method: reqwest::Method, url: &str) -> Result<reqwest::RequestBuilder, FerrumyxError> {
        if !self.is_allowed(url) {
            return Err(FerrumyxError::SecurityError(format!(
                "Network capabilities capped: domain not in allowlist for URL {}",
                url
            )));
        }

        Ok(self.client.request(method, url))
    }
}
