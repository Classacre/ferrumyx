use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProfile {
    pub ram_gb: f64,
    pub logical_cpus: usize,
    pub has_nvidia_gpu: bool,
    pub has_cuda_toolkit: bool,
    pub cuda_install_attempted: bool,
}

impl RuntimeProfile {
    pub fn detect_and_prepare() -> Self {
        if let Some(cached) = load_cached_profile() {
            return cached;
        }

        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();
        let total_mem_kib = sys.total_memory() as f64;
        let ram_gb = (total_mem_kib / 1024.0 / 1024.0).max(0.0);
        let logical_cpus = sys.cpus().len().max(1);

        let has_nvidia_gpu = detect_nvidia_gpu();
        let has_cuda_toolkit = command_success("nvcc", &["--version"]);
        let mut cuda_install_attempted = false;

        if has_nvidia_gpu && !has_cuda_toolkit {
            cuda_install_attempted = try_install_cuda_toolkit_once();
        }

        let profile = Self {
            ram_gb,
            logical_cpus,
            has_nvidia_gpu,
            has_cuda_toolkit: has_cuda_toolkit || (has_nvidia_gpu && cuda_install_attempted),
            cuda_install_attempted,
        };
        save_cached_profile(&profile);
        profile
    }

    pub fn source_timeout_secs(&self) -> u64 {
        if self.ram_gb >= 48.0 && self.logical_cpus >= 12 {
            12
        } else if self.ram_gb >= 24.0 && self.logical_cpus >= 8 {
            18
        } else {
            25
        }
    }

    pub fn tuned_max_results(&self, requested: usize) -> usize {
        let cap = if self.ram_gb >= 48.0 && self.logical_cpus >= 12 {
            500
        } else if self.ram_gb >= 24.0 && self.logical_cpus >= 8 {
            300
        } else if self.ram_gb >= 12.0 {
            150
        } else {
            80
        };
        requested.clamp(1, cap)
    }

    pub fn use_full_text_default(&self) -> bool {
        true
    }

    pub fn tuned_embedding_batch_size(&self, current: usize) -> usize {
        let target = if self.has_nvidia_gpu && self.has_cuda_toolkit {
            64
        } else if self.ram_gb >= 24.0 {
            32
        } else {
            16
        };
        let floor = (target / 2).max(8);
        current.max(1).max(floor).min(target)
    }
}

fn profile_cache_path() -> std::path::PathBuf {
    std::env::var("FERRUMYX_RUNTIME_PROFILE_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("data/cache/runtime_profile.json"))
}

fn load_cached_profile() -> Option<RuntimeProfile> {
    let path = profile_cache_path();
    let payload = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<RuntimeProfile>(&payload).ok()
}

fn save_cached_profile(profile: &RuntimeProfile) {
    let path = profile_cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(payload) = serde_json::to_string_pretty(profile) {
        let _ = std::fs::write(path, payload);
    }
}

fn command_success(bin: &str, args: &[&str]) -> bool {
    Command::new(bin)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn command_output_contains(bin: &str, args: &[&str], needle: &str) -> bool {
    Command::new(bin)
        .args(args)
        .output()
        .ok()
        .and_then(|o| {
            if !o.status.success() {
                return None;
            }
            Some(String::from_utf8_lossy(&o.stdout).to_string())
        })
        .is_some_and(|s| {
            s.to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
        })
}

fn detect_nvidia_gpu() -> bool {
    if command_success("nvidia-smi", &["--query-gpu=name", "--format=csv,noheader"]) {
        return true;
    }
    #[cfg(target_os = "windows")]
    {
        if command_output_contains(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name) -join \"`n\"",
            ],
            "nvidia",
        ) {
            return true;
        }
        if command_output_contains(
            "wmic",
            &["path", "win32_VideoController", "get", "name"],
            "nvidia",
        ) {
            return true;
        }
    }
    false
}

fn try_install_cuda_toolkit_once() -> bool {
    static ATTEMPTED: OnceLock<bool> = OnceLock::new();
    *ATTEMPTED.get_or_init(try_install_cuda_toolkit)
}

fn try_install_cuda_toolkit() -> bool {
    #[cfg(not(target_os = "windows"))]
    {
        return false;
    }

    #[cfg(target_os = "windows")]
    {
        if !command_success("winget", &["--version"]) {
            return false;
        }

        let candidate_ids = ["Nvidia.CUDA", "NVIDIA.CUDA"];
        for id in candidate_ids {
            let status = Command::new("winget")
                .args([
                    "install",
                    "-e",
                    "--id",
                    id,
                    "--silent",
                    "--accept-source-agreements",
                    "--accept-package-agreements",
                ])
                .spawn()
                .and_then(|mut child| {
                    let start = std::time::Instant::now();
                    loop {
                        if let Some(status) = child.try_wait()? {
                            return Ok(status.success());
                        }
                        if start.elapsed() > Duration::from_secs(180) {
                            let _ = child.kill();
                            return Ok(false);
                        }
                        std::thread::sleep(Duration::from_millis(500));
                    }
                })
                .unwrap_or(false);

            if status && command_success("nvcc", &["--version"]) {
                return true;
            }
        }

        false
    }
}
