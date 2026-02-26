#!/usr/bin/env bash
# Ferrumyx Windows Easy Start Script
# This script installs Rust and Ollama, selects the best model based on RAM, and starts the agent.

set -e

echo "========================================="
echo " Ferrumyx Easy Start (Linux/macOS) "
echo "========================================="

# 1. Check/Install Rust
if ! command -v cargo &> /dev/null; then
    echo "Rust is not installed. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "✅ Rust is already installed."
fi

# 2. Check/Install Ollama
if ! command -v ollama &> /dev/null; then
    echo "Ollama is not installed. Installing Ollama..."
    curl -fsSL https://ollama.com/install.sh | sh
else
    echo "✅ Ollama is already installed."
fi

# Ensure Ollama daemon is running in background if not already
if ! curl -s http://localhost:11434/api/tags >/dev/null; then
    echo "Starting Ollama daemon..."
    ollama serve > /dev/null 2>&1 &
    sleep 3
fi

# 3. Detect System RAM
echo "Detecting System RAM..."
OS=$(uname -s)
RAM_GB=0

if [ "$OS" = "Darwin" ]; then
    RAM_BYTES=$(sysctl -n hw.memsize)
    RAM_GB=$((RAM_BYTES / 1024 / 1024 / 1024))
elif [ "$OS" = "Linux" ]; then
    RAM_KB=$(grep MemTotal /proc/meminfo | awk '{print $2}')
    RAM_GB=$((RAM_KB / 1024 / 1024))
else
    echo "Unsupported OS for RAM detection. Defaulting to 1B model."
    RAM_GB=4
fi

echo "Total System RAM: ${RAM_GB}GB"

# 4. Select Optimal Model
MODEL="llama3.2:1b"
if [ "$RAM_GB" -ge 16 ]; then
    MODEL="llama3.1:8b"
elif [ "$RAM_GB" -ge 8 ]; then
    MODEL="llama3.2:3b"
fi

echo "Based on your ${RAM_GB}GB of RAM, we selected the optimal model: ${MODEL}"

# 5. Pull Model
echo "Pulling model ${MODEL}... (This may take a while)"
ollama pull "$MODEL"

# Update configuration file to use the selected model if needed
CONFIG_FILE="ferrumyx.toml"
if [ -f "$CONFIG_FILE" ]; then
    sed -i.bak "s/model = \".*\"/model = \"${MODEL}\"/" "$CONFIG_FILE"
    echo "Updated ferrumyx.toml to use ${MODEL}"
fi

# 6. Build and Run
echo "Building and starting Ferrumyx Agent..."
export RUST_LOG=info
cargo run --release --bin ferrumyx
