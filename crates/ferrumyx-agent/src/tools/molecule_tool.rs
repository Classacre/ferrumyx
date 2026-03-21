use async_trait::async_trait;
use ferrumyx_runtime::context::JobContext;
use ferrumyx_runtime::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;

/// Tool to run the molecular pipeline for a target protein identifier.
pub struct RunMoleculePipelineTool;

impl RunMoleculePipelineTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for RunMoleculePipelineTool {
    fn name(&self) -> &str {
        "run_molecule_pipeline"
    }

    fn description(&self) -> &str {
        "Runs structure fetch, ligand generation, docking, and ADMET scoring for a UniProt target."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "uniprot_id": {
                    "type": "string",
                    "description": "UniProt accession (for example: P01116)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum molecules to return (default: 10, max: 50)"
                }
            },
            "required": ["uniprot_id"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let uniprot_id = require_str(&params, "uniprot_id")?.to_string();
        let max_results = params
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(10)
            .clamp(1, 50);

        let started = std::time::Instant::now();
        let pipeline = ferrumyx_molecules::pipeline::MoleculesPipeline::new("data/molecules");
        let ranked = pipeline
            .run(&uniprot_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("molecule pipeline failed: {e}")))?;

        let top: Vec<_> = ranked
            .into_iter()
            .take(max_results)
            .map(|m| {
                json!({
                    "smiles": m.molecule.smiles,
                    "mw": m.molecule.mw,
                    "docking_score": m.docking_score,
                    "composite_score": m.composite_score,
                    "admet": m.admet_properties
                })
            })
            .collect();

        Ok(ToolOutput::success(
            json!({
                "status": "ok",
                "uniprot_id": uniprot_id,
                "results": top
            }),
            started.elapsed(),
        ))
    }
}

fn require_str<'a>(params: &'a serde_json::Value, name: &str) -> Result<&'a str, ToolError> {
    params
        .get(name)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            ToolError::InvalidParameters(format!("missing required string parameter: {name}"))
        })
}
