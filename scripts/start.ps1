<#
.SYNOPSIS
Ferrumyx v2.0.0 Windows Easy Start Script

.DESCRIPTION
This script installs Rust and Ollama, sets up PostgreSQL, selects the best model based on RAM, and starts the IronClaw-powered Ferrumyx agent with BioClaw skills.
#>

$ErrorActionPreference = "Stop"

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host " Ferrumyx v2.0.0 Easy Start (Windows) " -ForegroundColor Cyan
Write-Host " IronClaw + BioClaw Integration " -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

# Prerequisites: Ensure PostgreSQL is installed and running with pgvector extension
Write-Host "⚠️  Prerequisites: PostgreSQL with pgvector extension must be installed and running." -ForegroundColor Yellow
Write-Host "   Install from: https://www.postgresql.org/download/windows/" -ForegroundColor Yellow
Write-Host "   Enable pgvector: CREATE EXTENSION vector;" -ForegroundColor Yellow
Write-Host ""

# 1. Check/Install Rust
if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Rust is not installed. Downloading rustup-init.exe..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri "https://win.rustup.rs" -OutFile "$env:TEMP\rustup-init.exe"
    Write-Host "Installing Rust (Default settings)..." -ForegroundColor Yellow
    & "$env:TEMP\rustup-init.exe" -y
    $env:Path += ";$env:USERPROFILE\.cargo\bin"
} else {
    Write-Host "✅ Rust is already installed." -ForegroundColor Green
}

# 2. Check/Install Ollama
if (!(Get-Command ollama -ErrorAction SilentlyContinue)) {
    Write-Host "Ollama is not installed. Downloading Ollama installer..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri "https://ollama.com/download/OllamaSetup.exe" -OutFile "$env:TEMP\OllamaSetup.exe"
    Write-Host "Installing Ollama..." -ForegroundColor Yellow
    Start-Process -FilePath "$env:TEMP\OllamaSetup.exe" -Wait -NoNewWindow
    # Refresh Path
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
} else {
    Write-Host "✅ Ollama is already installed." -ForegroundColor Green
}

# Ensure Ollama daemon
try {
    Invoke-RestMethod -Uri "http://localhost:11434/api/tags" -ErrorAction Stop | Out-Null
} catch {
    Write-Host "Starting Ollama daemon in background..." -ForegroundColor Yellow
    Start-Process ollama -ArgumentList "serve" -WindowStyle Hidden
    Start-Sleep -Seconds 5
}

# 3. Detect System RAM
Write-Host "Detecting System RAM..." -ForegroundColor Cyan
$computerSystem = Get-CimInstance Win32_ComputerSystem
$ramGB = [math]::Round($computerSystem.TotalPhysicalMemory / 1GB)
Write-Host "Total System RAM: ${ramGB}GB" -ForegroundColor Cyan

# 4. Detect System GPU & CUDA
Write-Host "Detecting System GPU..." -ForegroundColor Cyan
$gpus = Get-CimInstance Win32_VideoController
$hasNvidia = $false
foreach ($gpu in $gpus) {
    if ($gpu.Name -match "NVIDIA") {
        $hasNvidia = $true
        Write-Host "Found NVIDIA GPU: $($gpu.Name)" -ForegroundColor Green
    }
}

$hasNvcc = $false
if (Get-Command nvcc -ErrorAction SilentlyContinue) {
    $hasNvcc = $true
    Write-Host "CUDA Toolkit (nvcc) found." -ForegroundColor Green
} else {
    if ($hasNvidia) {
        Write-Host "NVIDIA GPU found but CUDA Toolkit (nvcc) is missing. Hardware acceleration will be disabled for embeddings. Install CUDA to enable." -ForegroundColor Yellow
    }
}
$cudaEnabled = $hasNvidia -and $hasNvcc

# 5. Select Optimal Model (Ollama uses GPU automatically if available)
$model = "llama3.2:1b"
if ($ramGB -ge 16 -or $hasNvidia) {
    $model = "llama3.1:8b"
} elseif ($ramGB -ge 8) {
    $model = "llama3.2" # 3B
}

Write-Host "Based on your hardware, we selected the optimal model: ${model}" -ForegroundColor Cyan

# 6. Pull Model
Write-Host "Pulling model ${model}... (This may take a while)" -ForegroundColor Yellow
ollama pull $model

# Update configuration file to use the selected model if needed
$configFile = "ferrumyx.toml"
if (Test-Path $configFile) {
    (Get-Content $configFile) -replace 'model = ".*"', "model = `"$model`"" | Set-Content $configFile
    Write-Host "Updated ferrumyx.toml to use ${model}" -ForegroundColor Green
}

# 7. Build and Run
Write-Host "Building and starting Ferrumyx Agent..." -ForegroundColor Cyan
$env:RUST_LOG = "info"

$cargoArgs = @("run", "--release", "-p", "ferrumyx-web")

# If CUDA capable, pass the feature flag to root compilation so the workspace member picks it up
if ($cudaEnabled) {
    Write-Host "Compiling with CUDA hardware acceleration enabled..." -ForegroundColor Green
    $cargoArgs += "--features"
    $cargoArgs += "ferrumyx-ingestion/cuda"
}

Write-Host "Waiting for IronClaw-powered Ferrumyx server to start..." -ForegroundColor Yellow
Write-Host "Features: WASM sandboxing, BioClaw skills, multi-channel support, encrypted secrets" -ForegroundColor Cyan
Write-Host "Will attempt to open http://localhost:3000 in your browser..." -ForegroundColor Yellow
Start-Job -ScriptBlock { Start-Sleep -Seconds 5; Start-Process "http://localhost:3000" } | Out-Null

try {
    & cargo $cargoArgs
} catch {
    Write-Host "Execution failed: $_" -ForegroundColor Red
}
