//! Test script to verify WASM bioinformatics tools can be loaded and executed

use ferrumyx_runtime::tools::wasm::{WasmToolLoader, load_dev_tools};
use ferrumyx_runtime::tools::ToolRegistry;
use ferrumyx_runtime::config::WasmConfig;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing WASM bioinformatics tools loading...");

    // Create tool registry
    let registry = Arc::new(ToolRegistry::new());

    // Create WASM loader
    let loader = WasmToolLoader::new(
        None, // No runtime needed for discovery
        registry.clone(),
        None, // No secrets store
    );

    // Get WASM config
    let wasm_config = WasmConfig::resolve().unwrap_or_default();
    println!("WASM enabled: {}", wasm_config.enabled);
    println!("Tools directory: {}", wasm_config.tools_dir.display());

    // Try to load dev tools
    match load_dev_tools(&loader, &wasm_config.tools_dir).await {
        Ok(count) => {
            println!("✅ Successfully loaded {} WASM tools", count);

            // List loaded tools
            println!("Loaded tools:");
            for tool_name in registry.list_tool_names().await {
                println!("  - {}", tool_name);
            }

            // Test BLAST tool execution
            if let Some(blast_tool) = registry.get_tool("blast-tool").await {
                println!("Testing BLAST tool execution...");
                let params = r#"{
                    "action": "blast_search",
                    "sequence": "ATCGATCGATCG",
                    "program": "blastn",
                    "database": "nt",
                    "max_results": 5
                }"#;

                match blast_tool.execute(params).await {
                    Ok(result) => {
                        println!("✅ BLAST tool executed successfully");
                        println!("Result: {}", result);
                    }
                    Err(e) => {
                        println!("❌ BLAST tool execution failed: {}", e);
                    }
                }
            } else {
                println!("❌ BLAST tool not found in registry");
            }
        }
        Err(e) => {
            println!("❌ Failed to load WASM tools: {}", e);
            println!("This is expected if WASM tools haven't been built yet.");
            println!("Run the following commands to build the tools:");
            println!("  cd crates/ferrumyx-runtime-core/tools-src/blast && cargo build --target wasm32-wasip2 --release");
            println!("  cd crates/ferrumyx-runtime-core/tools-src/fastqc && cargo build --target wasm32-wasip2 --release");
            println!("  cd crates/ferrumyx-runtime-core/tools-src/pymol && cargo build --target wasm32-wasip2 --release");
        }
    }

    Ok(())
}