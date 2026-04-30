# Ferrumyx Docker Container Orchestration

This directory contains Docker configurations for running bioinformatics tools in isolated containers within the Ferrumyx system.

## Architecture

The container orchestration provides:

- **Isolated Execution**: Each bioinformatics tool runs in its own container with resource limits
- **Per-Job Tokens**: Unique tokens for job isolation and tracking
- **Resource Management**: CPU, memory, and disk limits per container
- **Health Monitoring**: Automatic health checks and cleanup
- **IronClaw Integration**: Seamless integration with Ferrumyx's agent orchestrator

## Images

The following Docker images are provided:

- `ferrumyx/blast:latest` - NCBI BLAST for sequence similarity searches
- `ferrumyx/pymol:latest` - PyMOL for molecular structure visualization
- `ferrumyx/fastqc:latest` - FastQC for sequencing quality control
- `ferrumyx/fpocket:latest` - fpocket for protein pocket detection
- `ferrumyx/vina:latest` - AutoDock Vina for molecular docking
- `ferrumyx/rdkit:latest` - RDKit for cheminformatics
- `ferrumyx/admet:latest` - ADMET prediction tools

## Building Images

Run the build script:

```bash
./docker/build.sh
```

Or build individually:

```bash
docker build -f docker/Dockerfile.blast -t ferrumyx/blast:latest .
```

## Running Services

Start all services with docker-compose:

```bash
docker-compose -f docker/docker-compose.yml up -d
```

Check running containers:

```bash
docker ps
```

## Integration with Ferrumyx

The container orchestrator is automatically initialized when Ferrumyx starts, provided Docker is available. The bioinformatics tools in `ferrumyx-agent` use the orchestrator to execute commands in isolated containers.

### Resource Limits

Default limits per container:
- Memory: 2GB (configurable)
- CPU: 1024 shares (1 core)
- Disk: 10GB
- Timeout: 30 minutes

### Security

Containers run with:
- Non-root user (UID 1000)
- Dropped capabilities (ALL)
- Read-only root filesystem (except /workspace)
- No new privileges

### Job Isolation

Each job gets:
- Unique container name
- Unique job token for tracking
- Isolated workspace directory
- Environment variables for job context

## Health Monitoring

The orchestrator performs health checks every 5 minutes and automatically cleans up completed containers.

## Troubleshooting

### Docker Not Available

If Docker is not running, bioinformatics tools will fall back to placeholder mode with helpful error messages.

### Container Timeouts

Jobs exceeding the timeout limit are automatically killed and cleaned up.

### Resource Limits

Monitor container resource usage with:

```bash
docker stats
```

## Development

To add a new bioinformatics tool:

1. Create `Dockerfile.newtool` in the docker directory
2. Add the image mapping in `BioContainerOrchestrator::get_tool_image()`
3. Update `docker-compose.yml`
4. Update this README