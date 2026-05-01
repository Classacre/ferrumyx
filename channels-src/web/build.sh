#!/usr/bin/env bash
# Build the Web channel WASM component
#
# Prerequisites:
#   - Rust with wasm32-wasip2 target: rustup target add wasm32-wasip2
#   - wasm-tools for component creation: cargo install wasm-tools
#
# Output:
#   - web.wasm - WASM component ready for deployment
#   - web-channel.capabilities.json - Capabilities file (copy alongside .wasm)

set -euo pipefail

cd "$(dirname "$0")"

if ! command -v wasm-tools &> /dev/null; then
    echo "Error: wasm-tools not found. Install with: cargo install wasm-tools"
    exit 1
fi

echo "Building Web channel WASM component..."

# Build the WASM module
cargo build --release --target wasm32-wasip2

# Convert to component model (if not already a component)
# wasm-tools component new is idempotent on components
WASM_PATH="target/wasm32-wasip2/release/web_channel.wasm"

if [ -f "$WASM_PATH" ]; then
    # Create component if needed
    wasm-tools component new "$WASM_PATH" -o web.wasm 2>/dev/null || cp "$WASM_PATH" web.wasm

    # Optimize the component
    wasm-tools strip web.wasm -o web.wasm

    echo "Built: web.wasm ($(du -h web.wasm | cut -f1))"
    echo ""
    echo "To install:"
    echo "  mkdir -p ~/.ferrumyx-runtime-core/channels"
    echo "  cp web.wasm web-channel.capabilities.json ~/.ferrumyx-runtime-core/channels/"
    echo ""
else
    echo "Error: WASM output not found at $WASM_PATH"
    exit 1
fi