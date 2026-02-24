# CUDA Setup Guide for Ferrumyx

This guide explains how to enable GPU acceleration for NER models in Ferrumyx.

## Prerequisites

- NVIDIA GPU with CUDA support (Compute Capability 5.0+)
- Windows 10/11 or Linux
- Rust toolchain installed

## Windows Setup

### Step 1: Download CUDA Toolkit

1. Go to https://developer.nvidia.com/cuda-downloads
2. Select:
   - **Operating System**: Windows
   - **Architecture**: x86_64
   - **Version**: 11.8 or 12.x (recommended)
   - **Installer Type**: exe (local)
3. Download and run the installer

### Step 2: Install CUDA Toolkit

Run the installer with default settings. This will install:
- CUDA Toolkit (nvcc, libraries)
- NVIDIA drivers (if not already installed)
- CUDA samples (optional)

### Step 3: Verify Installation

Open a new PowerShell or Command Prompt window and run:

```powershell
nvcc --version
```

You should see output like:
```
nvcc: NVIDIA (R) Cuda compiler driver
Copyright (c) 2005-2023 NVIDIA Corporation
Built on Mon_Apr__3_17:36:15_Pacific_Daylight_Time_2023
Cuda compilation tools, release 12.1, V12.1.105
```

### Step 4: Set Environment Variables

Add CUDA to your system PATH:

```powershell
# For CUDA 12.x
[Environment]::SetEnvironmentVariable("CUDA_PATH", "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.4", "User")

# Add to PATH
$oldPath = [Environment]::GetEnvironmentVariable("PATH", "User")
$newPath = $oldPath + ";C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.4\bin"
[Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
```

**Restart your terminal/IDE** after setting environment variables.

## Linux Setup

### Ubuntu/Debian

```bash
# Install CUDA toolkit
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.0-1_all.deb
sudo dpkg -i cuda-keyring_1.0-1_all.deb
sudo apt-get update
sudo apt-get -y install cuda

# Add to PATH
echo 'export PATH=/usr/local/cuda/bin:$PATH' >> ~/.bashrc
source ~/.bashrc

# Verify
nvcc --version
```

## Building Ferrumyx with CUDA

Once CUDA is installed:

```bash
# Build with CUDA support
cargo build --release --features cuda

# Run benchmark with GPU
cargo run --example benchmark_ner --release --features cuda
```

## Verifying GPU Usage

The benchmark will show which device is being used:

```
=== NER Performance Benchmark ===
Device: Cuda(0)  # GPU
# or
Device: Cpu      # CPU (fallback)
```

## Troubleshooting

### "nvcc not found"
- Ensure CUDA bin directory is in PATH
- Restart your terminal after setting environment variables
- Try: `where nvcc` (Windows) or `which nvcc` (Linux)

### Build errors with cudarc
- Ensure CUDA version matches your driver
- Check `nvidia-smi` shows CUDA version
- Try different CUDA toolkit version (11.8 is most stable)

### Runtime errors
- Check GPU memory: `nvidia-smi`
- Models need ~2-4GB VRAM each
- Close other GPU applications

## Expected Performance

With RTX 3060 (6GB VRAM):
- CPU: ~3000-5000ms per text
- GPU: ~50-200ms per text (15-100x faster)
- Batch GPU: ~20-50ms per text (60-250x faster)

## Alternative: CPU-Only Mode

If you cannot install CUDA, the system works on CPU but will be slower:

```bash
# Build without CUDA (default)
cargo build --release

# Use smaller models for better CPU performance
cargo run --example benchmark_ner --release
```

Consider using the hybrid approach (rule-based filtering + ML NER) for CPU-only deployments.
