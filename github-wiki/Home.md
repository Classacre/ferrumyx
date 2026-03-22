# Ferrumyx Wiki Home

Ferrumyx is an autonomous oncology discovery platform implemented as a Rust workspace. This wiki is GitHub Wiki-ready and documents the live implementation: runtime behavior, APIs, CLI, editable parameters, and extension points.

## What this wiki covers

- End-to-end workflow: ingestion -> KG -> ranking -> molecules
- Agent/runtime model and tool surface
- API contracts and request parameters
- CLI arguments and command usage
- Editable/tunable parameters (config + environment)
- Developer extension playbooks

## Quick links

- [Getting Started](Getting-Started)
- [Workflows and Architecture](Workflows-and-Architecture)
- [API Reference](API-Reference)
- [CLI Reference](CLI-Reference)
- [Configuration and Tunable Parameters](Configuration-and-Tunable-Parameters)
- [Developer Guide](Developer-Guide)

## Source of truth in code

- Agent entrypoint: `crates/ferrumyx-agent/src/main.rs`
- Tool modules: `crates/ferrumyx-agent/src/tools/`
- Ingestion pipeline: `crates/ferrumyx-ingestion/src/pipeline.rs`
- Embeddings/hybrid retrieval: `crates/ferrumyx-ingestion/src/embedding.rs`
- DB layer: `crates/ferrumyx-db/src/`
- Web router/API surface: `crates/ferrumyx-web/src/router.rs`
- Federation schemas: `crates/ferrumyx-common/src/federation.rs`

## Publishing this wiki to GitHub Wiki

GitHub Wikis are stored in a separate repo (`<repo>.wiki.git`).

1. Clone wiki repo:
   - `git clone https://github.com/Classacre/ferrumyx.wiki.git`
2. Copy Markdown files from this folder (`github-wiki/`) into the cloned wiki repo.
3. Commit and push from the wiki repo.

Recommended page order:

1. `Home.md`
2. `_Sidebar.md`
3. Remaining reference pages
