//! ClinicalTrials.gov v2 API client.
//!
//! API docs: https://clinicaltrials.gov/data-api/api
//! Endpoint: https://clinicaltrials.gov/api/v2/studies
//!
//! Returns studies as PaperMetadata with:
//!   - title       = BriefTitle
//!   - abstract    = BriefSummary or DetailedDescription
//!   - pmid        = None (NCT IDs stored in doi field)
//!   - doi         = nct_id (e.g. NCT04956640)
//!   - source      = ClinicalTrials

use async_trait::async_trait;
use ferrumyx_common::sandbox::SandboxClient as Client;
use tracing::{debug, instrument};

use crate::models::{Author, IngestionSource, PaperMetadata};
use super::LiteratureSource;

const CT_API_URL: &str = "https://clinicaltrials.gov/api/v2/studies";

pub struct ClinicalTrialsClient {
    client: Client,
}

impl ClinicalTrialsClient {
    pub fn new() -> Self {
        Self { client: Client::new().unwrap() }
    }

    async fn search_studies(
        &self,
        query: &str,
        max_results: usize,
    ) -> anyhow::Result<Vec<serde_json::Value>> {
        let resp = self.client
            .get(CT_API_URL)?
            .query(&[
                ("query.term",    query),
                ("pageSize",      &max_results.to_string()),
                ("format",        "json"),
                ("fields",        "NCTId,BriefTitle,BriefSummary,DetailedDescription,\
                                   OverallStatus,Phase,Condition,InterventionName,\
                                   LeadSponsorName,StartDate,CompletionDate"),
            ])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        Ok(resp["studies"]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }
}

impl Default for ClinicalTrialsClient {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl LiteratureSource for ClinicalTrialsClient {
    #[instrument(skip(self))]
    async fn search(&self, query: &str, max_results: usize) -> anyhow::Result<Vec<PaperMetadata>> {
        // Strip PubMed [tiab] qualifiers — CT uses plain text search
        let clean_query: String = query
            .replace("[tiab]", "")
            .replace(" AND ", " ")
            .trim()
            .to_string();

        let studies = self.search_studies(&clean_query, max_results).await?;
        debug!(n = studies.len(), "ClinicalTrials.gov studies retrieved");

        let papers = studies.iter().map(|s| {
            let proto = &s["protocolSection"];
            let id_mod = &proto["identificationModule"];
            let desc_mod = &proto["descriptionModule"];
            let status_mod = &proto["statusModule"];
            let design_mod = &proto["designModule"];
            let cond_mod = &proto["conditionsModule"];
            let interv_mod = &proto["armsInterventionsModule"];
            let sponsor_mod = &proto["sponsorCollaboratorsModule"];

            let nct_id = id_mod["nctId"].as_str().unwrap_or("").to_string();
            let title  = id_mod["briefTitle"].as_str().unwrap_or("").to_string();

            // Abstract: prefer DetailedDescription, fall back to BriefSummary
            let abstract_text = desc_mod["detailedDescription"]["textBlock"]
                .as_str()
                .or_else(|| desc_mod["briefSummary"]["textBlock"].as_str())
                .map(String::from);

            // Build a pseudo-author from lead sponsor
            let sponsor = sponsor_mod["leadSponsor"]["name"]
                .as_str()
                .unwrap_or("Unknown Sponsor")
                .to_string();

            // Enrich title with phase and status
            let phase = design_mod["phases"]
                .as_array()
                .and_then(|phases| phases.first())
                .and_then(|p| p.as_str())
                .unwrap_or("N/A");
            let status = status_mod["overallStatus"].as_str().unwrap_or("Unknown");

            // Build conditions string
            let conditions: String = cond_mod["conditions"]
                .as_array()
                .map(|c| c.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join("; "))
                .unwrap_or_default();

            // Build interventions string
            let interventions: String = interv_mod["interventions"]
                .as_array()
                .map(|iv| iv.iter()
                    .filter_map(|v| v["name"].as_str())
                    .collect::<Vec<_>>()
                    .join("; "))
                .unwrap_or_default();

            // Enrich abstract with structured metadata
            let enriched_abstract = format!(
                "{}\n\nNCT ID: {}\nStatus: {}\nPhase: {}\nConditions: {}\nInterventions: {}",
                abstract_text.as_deref().unwrap_or("No description available."),
                nct_id, status, phase, conditions, interventions
            );

            PaperMetadata {
                doi:           Some(nct_id.clone()),  // store NCT ID here
                pmid:          None,
                pmcid:         None,
                title,
                abstract_text: Some(enriched_abstract),
                authors:       vec![Author {
                    name: sponsor,
                    affiliation: None,
                    orcid: None,
                }],
                journal:       Some(format!("ClinicalTrials.gov [{}]", status)),
                pub_date:      None,
                source:        IngestionSource::ClinicalTrials,
                open_access:   true,
                full_text_url: Some(format!("https://clinicaltrials.gov/study/{}", nct_id)),
            }
        }).collect();

        Ok(papers)
    }

    async fn fetch_full_text(&self, nct_id: &str) -> anyhow::Result<Option<String>> {
        // Return a CT.gov study detail URL — no HTML scraping
        Ok(Some(format!("https://clinicaltrials.gov/study/{}", nct_id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_query_strips_tiab() {
        let raw = "KRAS[tiab] AND G12D[tiab] AND pancreatic cancer[tiab]";
        let clean: String = raw
            .replace("[tiab]", "")
            .replace(" AND ", " ")
            .trim()
            .to_string();
        assert!(!clean.contains("[tiab]"));
        assert!(clean.contains("KRAS"));
    }

    #[test]
    fn test_client_default() {
        let _c = ClinicalTrialsClient::default();
    }
}
