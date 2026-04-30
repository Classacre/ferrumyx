# Getting Started

## Prerequisites

- Rust toolchain (stable)
- Protobuf compiler (`protoc`) available on PATH (or set `PROTOC` explicitly)
- Optional: Docker and scientific binaries depending on molecule workflows

## Start the system

### Agent runtime

- Command: `cargo run --release --bin ferrumyx`
- Entrypoint: `crates/ferrumyx-agent/src/main.rs`

### Web server

- Command: `cargo run -p ferrumyx-web`
- Default bind: `127.0.0.1:3001` (override with `FERRUMYX_WEB_ADDR`)
- Entrypoint: `crates/ferrumyx-web/src/main.rs`

### Combined helper script

- Windows helper: `start.ps1`
- Cross-platform helper: `start.sh`

## Initial verification checklist

1. Open web UI (`/`) and ensure navigation loads.
2. Open `/settings` and verify provider/runtime settings are visible.
3. Submit a small ingestion run from `/ingestion`.
4. Validate evidence appears in `/kg` and `/targets`.
5. Run `/query` with a bounded `max_results`.

## Core runtime binaries in workspace

- `ferrumyx` (agent binary from `ferrumyx-agent`)
- `ferrumyx-web` (web/API server)
- `ferrumyx-runtime-core` (runtime-core CLI and service utility)

## Troubleshooting first boot

- Build failures around protobuf: set `PROTOC` env var.
- Web starts but chat is offline: chat handler auto-attempts launching `ferrumyx` and falls back gracefully when gateway unavailable.
- Slow startup during first embedding use: model downloads/cache warm-up occur during initial runs.

## Next Steps

- Learn about [User Guides](User-Guides) for research workflows
- Integrate via [API Reference](API-Reference) or [CLI Reference](CLI-Reference)
- For development, see [Developer Setup](Developer-Setup)
- If issues persist, check [Troubleshooting](Troubleshooting)
