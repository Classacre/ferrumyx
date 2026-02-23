//! List all tensor names in cached OpenMed models

use std::path::Path;

fn main() -> anyhow::Result<()> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| anyhow::anyhow!("No cache dir"))?
        .join("huggingface")
        .join("hub");
    
    println!("HuggingFace cache dir: {:?}\n", cache_dir);
    
    // Find model directories
    if let Ok(entries) = std::fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            
            if name_str.starts_with("models--OpenMed") {
                let model_name = name_str
                    .trim_start_matches("models--")
                    .replace("--", "/");
                
                println!("=== {} ===", model_name);
                
                // Look for safetensors
                let snapshots_dir = entry.path().join("snapshots");
                if let Ok(snapshots) = std::fs::read_dir(&snapshots_dir) {
                    for snapshot in snapshots.flatten() {
                        let safetensors_path = snapshot.path().join("model.safetensors");
                        if safetensors_path.exists() {
                            match list_tensors(&safetensors_path) {
                                Ok(names) => {
                                    // Group by prefix
                                    let mut prefixes: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
                                    for name in names {
                                        let prefix = name.split('.').next().unwrap_or("").to_string();
                                        prefixes.entry(prefix).or_default().push(name);
                                    }
                                    
                                    println!("  Tensor prefixes found:");
                                    for (prefix, tensors) in &prefixes {
                                        println!("    {}: {} tensors", prefix, tensors.len());
                                        // Show first few tensor names
                                        for t in tensors.iter().take(3) {
                                            println!("      - {}", t);
                                        }
                                        if tensors.len() > 3 {
                                            println!("      ... and {} more", tensors.len() - 3);
                                        }
                                    }
                                }
                                Err(e) => println!("  Error: {:?}", e),
                            }
                        }
                    }
                }
                println!();
            }
        }
    }
    
    Ok(())
}

fn list_tensors(path: &Path) -> anyhow::Result<Vec<String>> {
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
    
    let mut names = Vec::new();
    
    if let Some(obj) = header.as_object() {
        for name in obj.keys() {
            if name != "__metadata__" {
                names.push(name.clone());
            }
        }
    }
    
    names.sort();
    Ok(names)
}
