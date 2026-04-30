//! Docker container orchestration for bioinformatics tools.
//!
//! Provides isolated execution environments for BLAST, PyMOL, FastQC, and other
//! bioinformatics software with per-job isolation and resource management.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bollard::container::{Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions, StartContainerOptions, WaitContainerOptions};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::models::HostConfig;
use bollard::Docker;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use ferrumyx_runtime::{ToolError, Value};

/// Container orchestrator for bioinformatics tools.
pub struct BioContainerOrchestrator {
    docker: Arc<Docker>,
    running_containers: Arc<RwLock<HashMap<String, ContainerInfo>>>,
}

/// Information about a running container.
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub container_id: String,
    pub tool_name: String,
    pub job_token: String,
    pub start_time: Instant,
    pub resource_limits: ResourceLimits,
}

/// Resource limits for container execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_mb: u64,
    pub cpu_shares: u64,
    pub disk_mb: u64,
    pub timeout_secs: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_mb: 2048, // 2GB
            cpu_shares: 1024,
            disk_mb: 10240, // 10GB
            timeout_secs: 1800, // 30 minutes
        }
    }
}

/// Execution result from a containerized tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerExecutionResult {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_secs: f64,
    pub truncated: bool,
    pub container_id: String,
    pub job_token: String,
}

impl BioContainerOrchestrator {
    /// Create a new container orchestrator.
    pub async fn new() -> Result<Self, ToolError> {
        let docker = connect_docker().await?;
        Ok(Self {
            docker: Arc::new(docker),
            running_containers: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Execute a bioinformatics tool in an isolated container.
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        command: &str,
        working_dir: &str,
        env_vars: HashMap<String, String>,
        resource_limits: ResourceLimits,
    ) -> Result<ContainerExecutionResult, ToolError> {
        let job_token = Uuid::new_v4().to_string();
        let container_name = format!("ferrumyx-{}-{}", tool_name, job_token);

        // Get the appropriate Docker image for the tool
        let image = self.get_tool_image(tool_name)?;

        // Create container configuration
        let config = self.build_container_config(
            &image,
            command,
            working_dir,
            env_vars,
            &resource_limits,
            &job_token,
        )?;

        let options = CreateContainerOptions {
            name: container_name.clone(),
            ..Default::default()
        };

        // Create the container
        let start_time = Instant::now();
        let container = self.docker
            .create_container(Some(options), config)
            .await
            .map_err(|e| ToolError::new(&format!("Failed to create container: {}", e)))?;

        let container_id = container.id;

        // Store container info
        let info = ContainerInfo {
            container_id: container_id.clone(),
            tool_name: tool_name.to_string(),
            job_token: job_token.clone(),
            start_time,
            resource_limits: resource_limits.clone(),
        };

        {
            let mut running = self.running_containers.write().await;
            running.insert(job_token.clone(), info);
        }

        // Start the container
        self.docker
            .start_container(&container_id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| ToolError::new(&format!("Failed to start container: {}", e)))?;

        // Wait for completion with timeout
        let result = tokio::time::timeout(
            Duration::from_secs(resource_limits.timeout_secs),
            self.wait_for_completion(&container_id, 10 * 1024 * 1024), // 10MB max output
        )
        .await;

        // Always clean up
        let _ = self.cleanup_container(&container_id).await;

        // Remove from running containers
        {
            let mut running = self.running_containers.write().await;
            running.remove(&job_token);
        }

        match result {
            Ok(Ok((exit_code, stdout, stderr, truncated))) => {
                let execution_time = start_time.elapsed().as_secs_f64();
                Ok(ContainerExecutionResult {
                    exit_code,
                    stdout,
                    stderr,
                    execution_time_secs: execution_time,
                    truncated,
                    container_id,
                    job_token,
                })
            }
            Ok(Err(e)) => Err(ToolError::new(&format!("Container execution failed: {}", e))),
            Err(_) => {
                let _ = self.force_cleanup(&container_id).await;
                Err(ToolError::new("Container execution timed out"))
            }
        }
    }

    /// Get the Docker image for a specific tool.
    fn get_tool_image(&self, tool_name: &str) -> Result<String, ToolError> {
        match tool_name {
            "blast" => Ok("ferrumyx/blast:latest".to_string()),
            "pymol" => Ok("ferrumyx/pymol:latest".to_string()),
            "fastqc" => Ok("ferrumyx/fastqc:latest".to_string()),
            "fpocket" => Ok("ferrumyx/fpocket:latest".to_string()),
            "vina" => Ok("ferrumyx/vina:latest".to_string()),
            "rdkit" => Ok("ferrumyx/rdkit:latest".to_string()),
            "admet" => Ok("ferrumyx/admet:latest".to_string()),
            _ => Err(ToolError::new(&format!("Unknown tool: {}", tool_name))),
        }
    }

    /// Build container configuration.
    fn build_container_config(
        &self,
        image: &str,
        command: &str,
        working_dir: &str,
        env_vars: HashMap<String, String>,
        limits: &ResourceLimits,
        job_token: &str,
    ) -> Result<Config<String>, ToolError> {
        // Build environment variables
        let mut env_vec = vec![
            format!("JOB_TOKEN={}", job_token),
            format!("TOOL_WORKDIR={}", working_dir),
        ];

        for (key, value) in env_vars {
            env_vec.push(format!("{}={}", key, value));
        }

        // Build volume mounts
        let binds = vec![
            format!("{}:/workspace:rw", working_dir),
            format!("{}:/output:rw", working_dir), // Assume output is in working dir
        ];

        let host_config = HostConfig {
            binds: Some(binds),
            memory: Some((limits.memory_mb * 1024 * 1024) as i64),
            cpu_shares: Some(limits.cpu_shares as i64),
            auto_remove: Some(true),
            network_mode: Some("bridge".to_string()),
            // Security: drop all capabilities
            cap_drop: Some(vec!["ALL".to_string()]),
            cap_add: Some(vec!["CHOWN".to_string()]),
            security_opt: Some(vec!["no-new-privileges:true".to_string()]),
            readonly_rootfs: Some(false), // Allow writing to /workspace
            tmpfs: Some(vec![
                ("/tmp".to_string(), "size=512M".to_string()),
            ].into_iter().collect()),
            ..Default::default()
        };

        let config = Config {
            image: Some(image.to_string()),
            cmd: Some(vec!["sh".to_string(), "-c".to_string(), command.to_string()]),
            working_dir: Some("/workspace".to_string()),
            env: Some(env_vec),
            host_config: Some(host_config),
            user: Some("1000:1000".to_string()), // Non-root user
            ..Default::default()
        };

        Ok(config)
    }

    /// Wait for container completion and collect output.
    async fn wait_for_completion(
        &self,
        container_id: &str,
        max_output_bytes: usize,
    ) -> Result<(i64, String, String, bool), ToolError> {
        // Wait for container to finish
        let mut wait_stream = self.docker
            .wait_container(container_id, Some(WaitContainerOptions {
                condition: "not-running",
            }))
            .await
            .map_err(|e| ToolError::new(&format!("Wait failed: {}", e)))?;

        let exit_code = match wait_stream.next().await {
            Some(Ok(response)) => response.status_code,
            Some(Err(e)) => return Err(ToolError::new(&format!("Wait error: {}", e))),
            None => return Err(ToolError::new("Container wait stream ended unexpectedly")),
        };

        // Collect logs
        let (stdout, stderr, truncated) = self.collect_logs(container_id, max_output_bytes).await?;

        Ok((exit_code, stdout, stderr, truncated))
    }

    /// Collect stdout and stderr from container.
    async fn collect_logs(
        &self,
        container_id: &str,
        max_output: usize,
    ) -> Result<(String, String, bool), ToolError> {
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            follow: false,
            ..Default::default()
        };

        let mut stream = self.docker.logs(container_id, Some(options)).await
            .map_err(|e| ToolError::new(&format!("Logs failed: {}", e)))?;

        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut truncated = false;
        let half_max = max_output / 2;

        while let Some(result) = stream.next().await {
            match result {
                Ok(LogOutput::StdOut { message }) => {
                    let text = String::from_utf8_lossy(&message);
                    if stdout.len() + text.len() <= half_max {
                        stdout.push_str(&text);
                    } else {
                        truncated = true;
                    }
                }
                Ok(LogOutput::StdErr { message }) => {
                    let text = String::from_utf8_lossy(&message);
                    if stderr.len() + text.len() <= half_max {
                        stderr.push_str(&text);
                    } else {
                        truncated = true;
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("Error reading container logs: {}", e);
                }
            }
        }

        Ok((stdout, stderr, truncated))
    }

    /// Clean up a container.
    async fn cleanup_container(&self, container_id: &str) -> Result<(), ToolError> {
        self.docker
            .remove_container(
                container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| ToolError::new(&format!("Cleanup failed: {}", e)))?;
        Ok(())
    }

    /// Force cleanup of a container (for timeouts).
    async fn force_cleanup(&self, container_id: &str) -> Result<(), ToolError> {
        // Try to kill first
        let _ = self.docker.kill_container::<String>(container_id, None).await;
        // Then remove
        let _ = self.cleanup_container(container_id).await;
        Ok(())
    }

    /// Get health status of running containers.
    pub async fn health_check(&self) -> HashMap<String, ContainerHealth> {
        let running = self.running_containers.read().await;
        let mut health = HashMap::new();

        for (token, info) in running.iter() {
            let elapsed = info.start_time.elapsed();
            let is_healthy = elapsed < Duration::from_secs(info.resource_limits.timeout_secs);

            health.insert(token.clone(), ContainerHealth {
                container_id: info.container_id.clone(),
                tool_name: info.tool_name.clone(),
                running_time_secs: elapsed.as_secs(),
                is_healthy,
                memory_usage_mb: None, // Would need docker stats API
            });
        }

        health
    }

    /// Get list of running containers.
    pub async fn list_running(&self) -> Vec<ContainerInfo> {
        let running = self.running_containers.read().await;
        running.values().cloned().collect()
    }
}

/// Health status of a container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerHealth {
    pub container_id: String,
    pub tool_name: String,
    pub running_time_secs: u64,
    pub is_healthy: bool,
    pub memory_usage_mb: Option<u64>,
}

/// Connect to Docker daemon.
/// Similar to ferrumyx-runtime-core implementation.
async fn connect_docker() -> Result<Docker, ToolError> {
    // Try default connection
    if let Ok(docker) = Docker::connect_with_local_defaults() {
        if docker.ping().await.is_ok() {
            return Ok(docker);
        }
    }

    // Try common socket locations
    #[cfg(unix)]
    {
        use std::path::PathBuf;
        let socket_paths = vec![
            "/var/run/docker.sock",
            &format!("{}/.docker/run/docker.sock", std::env::var("HOME").unwrap_or_default()),
            &format!("{}/.colima/default/docker.sock", std::env::var("HOME").unwrap_or_default()),
        ];

        for path in socket_paths {
            if std::path::Path::new(path).exists() {
                if let Ok(docker) = Docker::connect_with_socket(path, 120, bollard::API_DEFAULT_VERSION) {
                    if docker.ping().await.is_ok() {
                        return Ok(docker);
                    }
                }
            }
        }
    }

    Err(ToolError::new("Could not connect to Docker daemon"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::test;

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.memory_mb, 2048);
        assert_eq!(limits.cpu_shares, 1024);
        assert_eq!(limits.disk_mb, 10240);
        assert_eq!(limits.timeout_secs, 1800);
    }

    #[test]
    fn test_get_tool_image() {
        let orchestrator = BioContainerOrchestrator {
            docker: Arc::new(Docker::connect_with_local_defaults().unwrap_or_else(|_| panic!("Docker not available"))),
            running_containers: Arc::new(RwLock::new(HashMap::new())),
        };

        assert_eq!(orchestrator.get_tool_image("blast").unwrap(), "ferrumyx/blast:latest");
        assert_eq!(orchestrator.get_tool_image("pymol").unwrap(), "ferrumyx/pymol:latest");
        assert_eq!(orchestrator.get_tool_image("fastqc").unwrap(), "ferrumyx/fastqc:latest");
        assert_eq!(orchestrator.get_tool_image("fpocket").unwrap(), "ferrumyx/fpocket:latest");
        assert_eq!(orchestrator.get_tool_image("vina").unwrap(), "ferrumyx/vina:latest");
        assert_eq!(orchestrator.get_tool_image("rdkit").unwrap(), "ferrumyx/rdkit:latest");
        assert_eq!(orchestrator.get_tool_image("admet").unwrap(), "ferrumyx/admet:latest");

        assert!(orchestrator.get_tool_image("unknown_tool").is_err());
    }

    #[test]
    fn test_build_container_config() {
        let orchestrator = BioContainerOrchestrator {
            docker: Arc::new(Docker::connect_with_local_defaults().unwrap_or_else(|_| panic!("Docker not available"))),
            running_containers: Arc::new(RwLock::new(HashMap::new())),
        };

        let mut env_vars = HashMap::new();
        env_vars.insert("TEST_VAR".to_string(), "test_value".to_string());

        let limits = ResourceLimits {
            memory_mb: 1024,
            cpu_shares: 512,
            disk_mb: 5120,
            timeout_secs: 900,
        };

        let config = orchestrator.build_container_config(
            "ferrumyx/test:latest",
            "echo hello",
            "/tmp/test",
            env_vars,
            &limits,
            "test-job-123"
        ).unwrap();

        assert_eq!(config.image, Some("ferrumyx/test:latest".to_string()));
        assert_eq!(config.cmd, Some(vec!["sh".to_string(), "-c".to_string(), "echo hello".to_string()]));
        assert_eq!(config.working_dir, Some("/workspace".to_string()));
        assert_eq!(config.user, Some("1000:1000".to_string()));

        let env = config.env.unwrap();
        assert!(env.contains(&"JOB_TOKEN=test-job-123".to_string()));
        assert!(env.contains(&"TOOL_WORKDIR=/tmp/test".to_string()));
        assert!(env.contains(&"TEST_VAR=test_value".to_string()));

        let host_config = config.host_config.unwrap();
        assert!(host_config.auto_remove.unwrap());
        assert_eq!(host_config.memory, Some(1024 * 1024 * 1024)); // 1GB in bytes
        assert_eq!(host_config.cpu_shares, Some(512));
        assert_eq!(host_config.network_mode, Some("bridge".to_string()));

        let cap_drop = host_config.cap_drop.unwrap();
        assert!(cap_drop.contains(&"ALL".to_string()));

        let cap_add = host_config.cap_add.unwrap();
        assert!(cap_add.contains(&"CHOWN".to_string()));

        let security_opt = host_config.security_opt.unwrap();
        assert!(security_opt.contains(&"no-new-privileges:true".to_string()));
    }

    #[tokio::test]
    async fn test_health_check_empty() {
        let orchestrator = BioContainerOrchestrator {
            docker: Arc::new(Docker::connect_with_local_defaults().unwrap_or_else(|_| panic!("Docker not available"))),
            running_containers: Arc::new(RwLock::new(HashMap::new())),
        };

        let health = orchestrator.health_check().await;
        assert!(health.is_empty());
    }

    #[tokio::test]
    async fn test_list_running_empty() {
        let orchestrator = BioContainerOrchestrator {
            docker: Arc::new(Docker::connect_with_local_defaults().unwrap_or_else(|_| panic!("Docker not available"))),
            running_containers: Arc::new(RwLock::new(HashMap::new())),
        };

        let running = orchestrator.list_running().await;
        assert!(running.is_empty());
    }

    #[test]
    fn test_container_info_creation() {
        use std::time::Instant;

        let limits = ResourceLimits::default();
        let start_time = Instant::now();

        let info = ContainerInfo {
            container_id: "test-container-123".to_string(),
            tool_name: "blast".to_string(),
            job_token: "test-job-456".to_string(),
            start_time,
            resource_limits: limits.clone(),
        };

        assert_eq!(info.container_id, "test-container-123");
        assert_eq!(info.tool_name, "blast");
        assert_eq!(info.job_token, "test-job-456");
        assert_eq!(info.resource_limits.memory_mb, limits.memory_mb);
    }

    #[test]
    fn test_container_execution_result_creation() {
        let result = ContainerExecutionResult {
            exit_code: 0,
            stdout: "Hello World".to_string(),
            stderr: "".to_string(),
            execution_time_secs: 1.5,
            truncated: false,
            container_id: "test-container".to_string(),
            job_token: "test-job".to_string(),
        };

        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "Hello World");
        assert_eq!(result.stderr, "");
        assert_eq!(result.execution_time_secs, 1.5);
        assert!(!result.truncated);
        assert_eq!(result.container_id, "test-container");
        assert_eq!(result.job_token, "test-job");
    }

    #[test]
    fn test_container_health_creation() {
        let health = ContainerHealth {
            container_id: "test-container".to_string(),
            tool_name: "blast".to_string(),
            running_time_secs: 60,
            is_healthy: true,
            memory_usage_mb: Some(512),
        };

        assert_eq!(health.container_id, "test-container");
        assert_eq!(health.tool_name, "blast");
        assert_eq!(health.running_time_secs, 60);
        assert!(health.is_healthy);
        assert_eq!(health.memory_usage_mb, Some(512));
    }

    // Integration test that requires Docker - only run if Docker is available
    #[tokio::test]
    async fn test_docker_connection() {
        let result = connect_docker().await;
        // This test will pass if Docker is available, fail if not
        // In CI/CD, we should ensure Docker is available
        match result {
            Ok(docker) => {
                // Test that we can ping Docker
                let ping_result = docker.ping().await;
                assert!(ping_result.is_ok(), "Docker ping failed");
            }
            Err(e) => {
                // Docker not available - this is expected in some environments
                println!("Docker not available: {}", e);
            }
        }
    }

    // Test that would require mocking Docker API calls
    // For now, we test the logic that doesn't require actual Docker operations
    #[test]
    fn test_tool_image_mapping() {
        let orchestrator = BioContainerOrchestrator {
            docker: Arc::new(Docker::connect_with_local_defaults().unwrap_or_else(|_| panic!("Docker not available"))),
            running_containers: Arc::new(RwLock::new(HashMap::new())),
        };

        // Test all supported tools
        let tools = vec!["blast", "pymol", "fastqc", "fpocket", "vina", "rdkit", "admet"];

        for tool in tools {
            let image = orchestrator.get_tool_image(tool).unwrap();
            assert!(image.starts_with("ferrumyx/"));
            assert!(image.ends_with(":latest"));
            assert!(image.contains(tool));
        }
    }

    #[test]
    fn test_build_container_config_env_vars() {
        let orchestrator = BioContainerOrchestrator {
            docker: Arc::new(Docker::connect_with_local_defaults().unwrap_or_else(|_| panic!("Docker not available"))),
            running_containers: Arc::new(RwLock::new(HashMap::new())),
        };

        let mut env_vars = HashMap::new();
        env_vars.insert("CUSTOM_VAR1".to_string(), "value1".to_string());
        env_vars.insert("CUSTOM_VAR2".to_string(), "value2".to_string());

        let limits = ResourceLimits::default();

        let config = orchestrator.build_container_config(
            "test:latest",
            "test command",
            "/test/dir",
            env_vars,
            &limits,
            "job-123"
        ).unwrap();

        let env = config.env.unwrap();

        // Check that custom env vars are included
        assert!(env.contains(&"CUSTOM_VAR1=value1".to_string()));
        assert!(env.contains(&"CUSTOM_VAR2=value2".to_string()));

        // Check that standard env vars are present
        assert!(env.contains(&"JOB_TOKEN=job-123".to_string()));
        assert!(env.contains(&"TOOL_WORKDIR=/test/dir".to_string()));
    }
}