<#
.SYNOPSIS
Ferrumyx Windows Easy Start Script

.DESCRIPTION
This script installs Rust and Ollama, selects the best model based on RAM, and starts the agent.
#>

$ErrorActionPreference = "Stop"

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host " Ferrumyx Easy Start (Windows) " -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

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

# 4. Select Optimal Model
$model = "llama3.2:1b"
if ($ramGB -ge 16) {
    $model = "llama3.1:8b"
} elseif ($ramGB -ge 8) {
    $model = "llama3.2" # 3B
}

Write-Host "Based on your ${ramGB}GB of RAM, we selected the optimal model: ${model}" -ForegroundColor Cyan

# 5. Pull Model
Write-Host "Pulling model ${model}... (This may take a while)" -ForegroundColor Yellow
ollama pull $model

# Update configuration file to use the selected model if needed
$configFile = "ferrumyx.toml"
if (Test-Path $configFile) {
    (Get-Content $configFile) -replace 'model = ".*"', "model = `"$model`"" | Set-Content $configFile
    Write-Host "Updated ferrumyx.toml to use ${model}" -ForegroundColor Green
}

# 6. Build and Run
Write-Host "Building and starting Ferrumyx Agent..." -ForegroundColor Cyan
$env:RUST_LOG = "info"
try {
    cargo run --release --bin ferrumyx
} catch {
    Write-Host "Execution failed: $_" -ForegroundColor Red
}
