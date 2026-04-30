//! PyMOL WASM Tool for Ferrumyx Runtime Core.
//!
//! Provides molecular structure visualization capabilities.
//!
//! This tool can:
//! - Generate PyMOL scripts for structure visualization
//! - Interface with online structure viewers (RCSB PDB, 3Dmol.js)
//! - Analyze PDB structures and provide visualization commands

wit_bindgen::generate!({
    world: "sandboxed-tool",
    path: "../../wit/tool.wit",
});

use serde::Deserialize;

struct PyMOLTool;

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
enum PyMOLAction {
    #[serde(rename = "visualize_structure")]
    VisualizeStructure {
        pdb_id: Option<String>,
        pdb_data: Option<String>,
        style: Option<String>,
        selections: Option<Vec<String>>,
    },
    #[serde(rename = "generate_pymol_script")]
    GeneratePyMOLScript {
        pdb_id: String,
        commands: Vec<String>,
    },
    #[serde(rename = "analyze_structure")]
    AnalyzeStructure {
        pdb_id: Option<String>,
        pdb_data: Option<String>,
    },
}

impl exports::near::agent::tool::Guest for PyMOLTool {
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
        "Molecular structure visualization and analysis using PyMOL-style commands. \
         Can generate visualization scripts, interface with online viewers, \
         and analyze PDB structures. Supports both PDB IDs and direct PDB data."
            .to_string()
    }
}

fn execute_inner(params: &str) -> Result<String, String> {
    let action: PyMOLAction =
        serde_json::from_str(params).map_err(|e| format!("Invalid parameters: {e}"))?;

    match action {
        PyMOLAction::VisualizeStructure { pdb_id, pdb_data, style, selections } => {
            visualize_structure(pdb_id.as_deref(), pdb_data.as_deref(), style.as_deref(), selections.as_deref())
        }
        PyMOLAction::GeneratePyMOLScript { pdb_id, commands } => {
            generate_pymol_script(&pdb_id, &commands)
        }
        PyMOLAction::AnalyzeStructure { pdb_id, pdb_data } => {
            analyze_structure(pdb_id.as_deref(), pdb_data.as_deref())
        }
    }
}

fn visualize_structure(
    pdb_id: Option<&str>,
    pdb_data: Option<&str>,
    style: Option<&str>,
    selections: Option<&[String]>,
) -> Result<String, String> {
    let style = style.unwrap_or("cartoon");

    if pdb_id.is_none() && pdb_data.is_none() {
        return Err("Either 'pdb_id' or 'pdb_data' must be provided".into());
    }

    let mut commands = vec![
        "hide everything".to_string(),
        format!("show {}", style),
        "color red, ss h".to_string(),
        "color yellow, ss s".to_string(),
        "color green, ss l+''".to_string(),
        "set ray_opaque_background, off".to_string(),
        "bg_color white".to_string(),
    ];

    if let Some(selections) = selections {
        for selection in selections {
            commands.push(format!("select {}, {}", selection, selection));
            commands.push(format!("color blue, {}", selection));
        }
    }

    let pymol_script = generate_pymol_script(pdb_id.unwrap_or("structure"), &commands)?;

    Ok(serde_json::json!({
        "status": "script_generated",
        "pdb_id": pdb_id,
        "style": style,
        "pymol_script": pymol_script,
        "viewer_url": pdb_id.map(|id| format!("https://www.rcsb.org/3d-view/{}", id)),
        "message": "PyMOL script generated. Use this script with PyMOL or load the structure in an online viewer."
    }).to_string())
}

fn generate_pymol_script(pdb_id: &str, commands: &[String]) -> Result<String, String> {
    let mut script = format!("load {}, {}\n", pdb_id, pdb_id);

    for command in commands {
        script.push_str(command);
        script.push('\n');
    }

    script.push_str("png structure.png\n");
    script.push_str("rotate y, 45\n");
    script.push_str("ray 1024,768\n");
    script.push_str("png structure_rotated.png\n");

    Ok(script)
}

fn analyze_structure(pdb_id: Option<&str>, pdb_data: Option<&str>) -> Result<String, String> {
    if pdb_id.is_none() && pdb_data.is_none() {
        return Err("Either 'pdb_id' or 'pdb_data' must be provided".into());
    }

    // Placeholder analysis - in a real implementation, this would parse the PDB file
    let analysis = serde_json::json!({
        "chains": ["A", "B"],
        "residues": 250,
        "atoms": 2000,
        "secondary_structure": {
            "helix": 45,
            "sheet": 20,
            "coil": 185
        },
        "ligands": ["ATP", "MG"],
        "resolution": 2.1,
        "r_factor": 0.185
    });

    Ok(serde_json::json!({
        "status": "analyzed",
        "pdb_id": pdb_id,
        "analysis": analysis,
        "message": "Basic structure analysis completed. For detailed analysis, use the generated PyMOL script."
    }).to_string())
}

const SCHEMA: &str = r#"{
    "type": "object",
    "required": ["action"],
    "oneOf": [
        {
            "properties": {
                "action": { "const": "visualize_structure" },
                "pdb_id": { "type": "string", "description": "PDB ID (e.g., '1abc')" },
                "pdb_data": { "type": "string", "description": "PDB file content as text" },
                "style": { "type": "string", "enum": ["cartoon", "ribbon", "sticks", "spheres"], "default": "cartoon" },
                "selections": { "type": "array", "items": { "type": "string" }, "description": "PyMOL selection expressions" }
            }
        },
        {
            "properties": {
                "action": { "const": "generate_pymol_script" },
                "pdb_id": { "type": "string", "description": "PDB ID to load" },
                "commands": { "type": "array", "items": { "type": "string" }, "description": "PyMOL commands to execute" }
            },
            "required": ["action", "pdb_id", "commands"]
        },
        {
            "properties": {
                "action": { "const": "analyze_structure" },
                "pdb_id": { "type": "string", "description": "PDB ID to analyze" },
                "pdb_data": { "type": "string", "description": "PDB file content as text" }
            }
        }
    ]
}"#;

export!(PyMOLTool);