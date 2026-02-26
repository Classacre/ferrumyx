use serde_json::Value;

use async_trait::async_trait;
use std::path::PathBuf;

use ironclaw::tools::{Tool, ToolOutput, ToolError};
use ironclaw::context::JobContext;
use std::time::Instant;
use ferrumyx_molecules::pdb::StructureFetcher;
use ferrumyx_molecules::pocket::FPocketRunner;
use ferrumyx_molecules::docking::{VinaRunner, DockingConfig};

/// Tool for fetching protein structures from PDB or AlphaFold.
pub struct FetchStructureTool {
    cache_dir: PathBuf,
}

impl FetchStructureTool {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }
}

#[async_trait]
impl Tool for FetchStructureTool {
    fn name(&self) -> &str {
        "fetch_structure"
    }

    fn description(&self) -> &str {
        "Fetches a protein structure from PDB (by PDB ID) or AlphaFold (by UniProt ID)."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "enum": ["pdb", "alphafold"],
                    "description": "The source to fetch from ('pdb' or 'alphafold')."
                },
                "id": {
                    "type": "string",
                    "description": "The PDB ID (e.g., '1CRN') or UniProt ID (e.g., 'P01112')."
                }
            },
            "required": ["source", "id"]
        })
    }

    async fn execute(&self, params: Value, _ctx: &JobContext) -> std::result::Result<ToolOutput, ToolError> {
        let start = Instant::now();
        let source = params["source"].as_str().unwrap_or("");
        let id = params["id"].as_str().unwrap_or("");

        if id.is_empty() {
            return Err(ToolError::InvalidParameters("Missing required parameter: id".to_string()));
        }

        let fetcher = StructureFetcher::new(&self.cache_dir);

        let path = match source {
            "pdb" => fetcher.fetch_pdb(id).await.map_err(|e| ToolError::ExecutionFailed(e.to_string()))?,
            "alphafold" => fetcher.fetch_alphafold(id).await.map_err(|e| ToolError::ExecutionFailed(e.to_string()))?,
            _ => return Err(ToolError::InvalidParameters("Invalid source. Must be 'pdb' or 'alphafold'.".to_string())),
        };

        let res = serde_json::json!({
            "status": "success",
            "path": path.to_string_lossy().into_owned()
        });
        Ok(ToolOutput::success(res, start.elapsed()))
    }
}

/// Tool for detecting binding pockets using fpocket.
pub struct DetectPocketsTool {
    executable_path: PathBuf,
}

impl DetectPocketsTool {
    pub fn new(executable_path: PathBuf) -> Self {
        Self { executable_path }
    }
}

#[async_trait]
impl Tool for DetectPocketsTool {
    fn name(&self) -> &str {
        "detect_pockets"
    }

    fn description(&self) -> &str {
        "Runs fpocket on a given PDB file to detect potential binding pockets."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pdb_path": {
                    "type": "string",
                    "description": "The path to the PDB file to analyze."
                }
            },
            "required": ["pdb_path"]
        })
    }

    async fn execute(&self, params: Value, _ctx: &JobContext) -> std::result::Result<ToolOutput, ToolError> {
        let start = Instant::now();
        let pdb_path_str = params["pdb_path"].as_str().unwrap_or("");
        if pdb_path_str.is_empty() {
            return Err(ToolError::InvalidParameters("Missing required parameter: pdb_path".to_string()));
        }

        let pdb_path = PathBuf::from(pdb_path_str);
        if !pdb_path.exists() {
            return Err(ToolError::ExecutionFailed(format!("PDB file not found at path: {}", pdb_path_str)));
        }

        let runner = FPocketRunner::new(&self.executable_path);
        let out_dir = runner.run(&pdb_path).await.map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let res = serde_json::json!({
            "status": "success",
            "output_dir": out_dir.to_string_lossy().into_owned()
        });
        Ok(ToolOutput::success(res, start.elapsed()))
    }
}

/// Tool for running AutoDock Vina.
pub struct DockMoleculeTool {
    executable_path: PathBuf,
}

impl DockMoleculeTool {
    pub fn new(executable_path: PathBuf) -> Self {
        Self { executable_path }
    }
}

#[async_trait]
impl Tool for DockMoleculeTool {
    fn name(&self) -> &str {
        "dock_molecule"
    }

    fn description(&self) -> &str {
        "Runs AutoDock Vina to dock a ligand into a receptor."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "receptor_path": { "type": "string" },
                "ligand_path": { "type": "string" },
                "center_x": { "type": "number" },
                "center_y": { "type": "number" },
                "center_z": { "type": "number" },
                "size_x": { "type": "number" },
                "size_y": { "type": "number" },
                "size_z": { "type": "number" },
                "exhaustiveness": { "type": "integer", "default": 8 },
                "out_path": { "type": "string" }
            },
            "required": ["receptor_path", "ligand_path", "center_x", "center_y", "center_z", "size_x", "size_y", "size_z", "out_path"]
        })
    }

    async fn execute(&self, params: Value, _ctx: &JobContext) -> std::result::Result<ToolOutput, ToolError> {
        let start = Instant::now();
        let config = DockingConfig {
            receptor: PathBuf::from(params["receptor_path"].as_str().unwrap_or("")),
            ligand: PathBuf::from(params["ligand_path"].as_str().unwrap_or("")),
            center_x: params["center_x"].as_f64().unwrap_or(0.0),
            center_y: params["center_y"].as_f64().unwrap_or(0.0),
            center_z: params["center_z"].as_f64().unwrap_or(0.0),
            size_x: params["size_x"].as_f64().unwrap_or(20.0),
            size_y: params["size_y"].as_f64().unwrap_or(20.0),
            size_z: params["size_z"].as_f64().unwrap_or(20.0),
            exhaustiveness: params["exhaustiveness"].as_u64().unwrap_or(8) as u32,
            out: PathBuf::from(params["out_path"].as_str().unwrap_or("")),
        };

        let runner = VinaRunner::new(&self.executable_path);
        let out_path = runner.run(&config).await.map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let res = serde_json::json!({
            "status": "success",
            "output_path": out_path.to_string_lossy().into_owned()
        });
        Ok(ToolOutput::success(res, start.elapsed()))
    }
}
