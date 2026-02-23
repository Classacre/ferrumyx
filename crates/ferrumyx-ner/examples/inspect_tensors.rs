//! Inspect safetensors file structure to understand tensor naming

use std::collections::HashMap;
use candle_core::safetensors::Load;

fn main() -> anyhow::Result<()> {
    // Check if models are cached locally
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| anyhow::anyhow!("No cache dir"))?
        .join("huggingface")
        .join("hub");
    
    println!("HuggingFace cache dir: {:?}", cache_dir);
    
    // Find model directories
    if let Ok(entries) = std::fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            
            if name_str.starts_with("models--OpenMed") {
                println!("\n=== Found model: {} ===", name_str);
                
                // Look for safetensors
                let snapshots_dir = entry.path().join("snapshots");
                if let Ok(snapshots) = std::fs::read_dir(&snapshots_dir) {
                    for snapshot in snapshots.flatten() {
                        let safetensors_path = snapshot.path().join("model.safetensors");
                        if safetensors_path.exists() {
                            println!("  Safetensors: {:?}", safetensors_path);
                            
                            // Load and inspect
                            match inspect_safetensors(&safetensors_path) {
                                Ok(info) => {
                                    println!("  Tensors (first 20):");
                                    for (name, shape) in info.iter().take(20) {
                                        println!("    {} -> {:?}", name, shape);
                                    }
                                    if info.len() > 20 {
                                        println!("  ... and {} more tensors", info.len() - 20);
                                    }
                                }
                                Err(e) => println!("  Error inspecting: {:?}", e),
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

fn inspect_safetensors(path: &std::path::Path) -> anyhow::Result<Vec<(String, Vec<usize>)>> {
    use std::fs::File;
    use memmap2::Mmap;
    
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    
    // Parse the header
    let header_len = u64::from_le_bytes([
        mmap[0], mmap[1], mmap[2], mmap[3],
        mmap[4], mmap[5], mmap[6], mmap[7],
    ]) as usize;
    
    let header_bytes = &mmap[8..8+header_len];
    let header: serde_json::Value = serde_json::from_slice(header_bytes)?;
    
    let mut tensors = Vec::new();
    
    if let Some(obj) = header.as_object() {
        for (name, value) in obj {
            if name == "__metadata__" {
                continue;
            }
            if let Some(info) = value.as_object() {
                if let Some(dims) = info.get("data_type") {
                    let _dtype = dims.as_str().unwrap_or("unknown");
                }
                if let Some(shape) = info.get("shape").and_then(|s| s.as_array()) {
                    let dims: Vec<usize> = shape
                        .iter()
                        .filter_map(|v| v.as_u64().map(|n| n as usize))
                        .collect();
                    tensors.push((name.clone(), dims));
                }
            }
        }
    }
    
    // Sort by name
    tensors.sort_by(|a, b| a.0.cmp(&b.0));
    
    Ok(tensors)
}
