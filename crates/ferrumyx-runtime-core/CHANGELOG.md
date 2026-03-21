# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- AWS Bedrock LLM provider via native Converse API with IAM and SSO auth support (feature-gated: `--features bedrock`)

## [0.16.1](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.16.0...v0.16.1) - 2026-03-06

### Fixed

- revert WASM artifact SHA256 checksums to null ([#627](https://github.com/nearai/ferrumyx-runtime-core/pull/627))

## [0.16.0](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.15.0...v0.16.0) - 2026-03-06

### Added

- *(e2e)* extensions tab tests, CI parallelization, and 3 production bug fixes ([#584](https://github.com/nearai/ferrumyx-runtime-core/pull/584))
- WASM extension versioning with WIT compat checks ([#592](https://github.com/nearai/ferrumyx-runtime-core/pull/592))
- Add HMAC-SHA256 webhook signature validation for Slack ([#588](https://github.com/nearai/ferrumyx-runtime-core/pull/588))
- restart ([#531](https://github.com/nearai/ferrumyx-runtime-core/pull/531))
- merge http/web_fetch tools, add tool output stash for large responses ([#578](https://github.com/nearai/ferrumyx-runtime-core/pull/578))
- integrate 13-dimension complexity scorer into smart routing ([#529](https://github.com/nearai/ferrumyx-runtime-core/pull/529))

### Fixed

- *(llm)* fix reasoning model response parsing bugs ([#564](https://github.com/nearai/ferrumyx-runtime-core/pull/564)) ([#580](https://github.com/nearai/ferrumyx-runtime-core/pull/580))
- *(ci)* fix three coverage workflow failures ([#597](https://github.com/nearai/ferrumyx-runtime-core/pull/597))
- Telegram channel accepts group messages from all users if owner_… ([#590](https://github.com/nearai/ferrumyx-runtime-core/pull/590))
- *(ci)* anchor coverage/ gitignore rule to repo root ([#591](https://github.com/nearai/ferrumyx-runtime-core/pull/591))
- *(security)* use OsRng for all security-critical key and token generation ([#519](https://github.com/nearai/ferrumyx-runtime-core/pull/519))
- prevent concurrent memory hygiene passes and Windows file lock errors ([#535](https://github.com/nearai/ferrumyx-runtime-core/pull/535))
- sort tool_definitions() for deterministic LLM tool ordering ([#582](https://github.com/nearai/ferrumyx-runtime-core/pull/582))
- *(ci)* persist all cargo-llvm-cov env vars for E2E coverage ([#559](https://github.com/nearai/ferrumyx-runtime-core/pull/559))

### Other

- *(llm)* complete response cache — set_model invalidation, stats logging, sync mutex ([#290](https://github.com/nearai/ferrumyx-runtime-core/pull/290))
- add 29 E2E trace tests for issues #571-575 ([#593](https://github.com/nearai/ferrumyx-runtime-core/pull/593))
- add 26 tests for multi-thread safety, db CRUD, concurrency, errors ([#442](https://github.com/nearai/ferrumyx-runtime-core/pull/442))
- update WASM artifact SHA256 checksums [skip ci] ([#560](https://github.com/nearai/ferrumyx-runtime-core/pull/560))
- add WIT compatibility tests for WASM extensions ([#586](https://github.com/nearai/ferrumyx-runtime-core/pull/586))
- Trajectory benchmarks and e2e trace test rig ([#553](https://github.com/nearai/ferrumyx-runtime-core/pull/553))

## [0.15.0](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.14.0...v0.15.0) - 2026-03-04

### Added

- *(oauth)* route callbacks through web gateway for hosted instances ([#555](https://github.com/nearai/ferrumyx-runtime-core/pull/555))
- *(web)* show error details for failed tool calls ([#490](https://github.com/nearai/ferrumyx-runtime-core/pull/490))
- *(extensions)* improve auth UX and add load-time validation ([#536](https://github.com/nearai/ferrumyx-runtime-core/pull/536))
- add local-test skill and Dockerfile.test for web gateway testing ([#524](https://github.com/nearai/ferrumyx-runtime-core/pull/524))

### Fixed

- *(security)* restrict query-token auth to SSE endpoints only ([#528](https://github.com/nearai/ferrumyx-runtime-core/pull/528))
- *(ci)* flush profraw coverage data in E2E teardown ([#550](https://github.com/nearai/ferrumyx-runtime-core/pull/550))
- *(wasm)* coerce string parameters to schema-declared types ([#498](https://github.com/nearai/ferrumyx-runtime-core/pull/498))
- *(agent)* strip leaked [Called tool ...] text from responses ([#497](https://github.com/nearai/ferrumyx-runtime-core/pull/497))
- *(web)* reset job list UI on restart failure ([#499](https://github.com/nearai/ferrumyx-runtime-core/pull/499))
- *(security)* replace .unwrap() panics in pairing store with proper error handling ([#515](https://github.com/nearai/ferrumyx-runtime-core/pull/515))

### Other

- Fix UTF-8 unsafe truncation in sandbox log capture ([#359](https://github.com/nearai/ferrumyx-runtime-core/pull/359))
- enhance coverage with feature matrix, postgres, and E2E ([#523](https://github.com/nearai/ferrumyx-runtime-core/pull/523))

## [0.14.0](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.13.1...v0.14.0) - 2026-03-04

### Added

- remove the okta tool ([#506](https://github.com/nearai/ferrumyx-runtime-core/pull/506))
- add OAuth support for WASM tools in web gateway ([#489](https://github.com/nearai/ferrumyx-runtime-core/pull/489))
- *(web)* fix jobs UI parity for non-sandbox mode ([#491](https://github.com/nearai/ferrumyx-runtime-core/pull/491))
- *(workspace)* add TOOLS.md, BOOTSTRAP.md, and disk-to-DB import ([#477](https://github.com/nearai/ferrumyx-runtime-core/pull/477))

### Fixed

- *(web)* mobile browser bar obscures chat input ([#508](https://github.com/nearai/ferrumyx-runtime-core/pull/508))
- *(web)* assign unique thread_id to manual routine triggers ([#500](https://github.com/nearai/ferrumyx-runtime-core/pull/500))
- *(web)* refresh routine UI after Run Now trigger ([#501](https://github.com/nearai/ferrumyx-runtime-core/pull/501))
- *(skills)* use slug for skill download URL from ClawHub ([#502](https://github.com/nearai/ferrumyx-runtime-core/pull/502))
- *(workspace)* thread document path through search results ([#503](https://github.com/nearai/ferrumyx-runtime-core/pull/503))
- *(workspace)* import custom templates before seeding defaults ([#505](https://github.com/nearai/ferrumyx-runtime-core/pull/505))
- use std::sync::RwLock in MessageTool to avoid runtime panic ([#411](https://github.com/nearai/ferrumyx-runtime-core/pull/411))
- wire secrets store into all WASM runtime activation paths ([#479](https://github.com/nearai/ferrumyx-runtime-core/pull/479))

### Other

- enforce regression tests for fix commits ([#517](https://github.com/nearai/ferrumyx-runtime-core/pull/517))
- add code coverage with cargo-llvm-cov and Codecov ([#511](https://github.com/nearai/ferrumyx-runtime-core/pull/511))
- Remove restart infrastructure, generalize WASM channel setup ([#493](https://github.com/nearai/ferrumyx-runtime-core/pull/493))

## [0.13.1](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.13.0...v0.13.1) - 2026-03-02

### Added

- add Brave Web Search WASM tool ([#474](https://github.com/nearai/ferrumyx-runtime-core/pull/474))

### Fixed

- *(web)* auto-scroll and Enter key completion for slash command autocomplete ([#475](https://github.com/nearai/ferrumyx-runtime-core/pull/475))
- correct download URLs for telegram-mtproto and slack-tool extensions ([#470](https://github.com/nearai/ferrumyx-runtime-core/pull/470))

## [0.13.0](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.12.0...v0.13.0) - 2026-03-02

### Added

- *(cli)* add tool setup command + GitHub setup schema ([#438](https://github.com/nearai/ferrumyx-runtime-core/pull/438))
- add web_fetch built-in tool ([#435](https://github.com/nearai/ferrumyx-runtime-core/pull/435))
- *(web)* DB-backed Jobs tab + scheduler-dispatched local jobs ([#436](https://github.com/nearai/ferrumyx-runtime-core/pull/436))
- *(extensions)* add OAuth setup UI for WASM tools + display name labels ([#437](https://github.com/nearai/ferrumyx-runtime-core/pull/437))
- *(bootstrap)* auto-detect libsql when ferrumyx-runtime-core.db exists ([#399](https://github.com/nearai/ferrumyx-runtime-core/pull/399))
- *(web)* slash command autocomplete + /status /list + fix chat input locking ([#404](https://github.com/nearai/ferrumyx-runtime-core/pull/404))
- *(routines)* deliver notifications to all installed channels ([#398](https://github.com/nearai/ferrumyx-runtime-core/pull/398))
- *(web)* persist tool calls, restore approvals on thread switch, and UI fixes ([#382](https://github.com/nearai/ferrumyx-runtime-core/pull/382))
- add FERRUMYX_RUNTIME_CORE_BASE_DIR env var with LazyLock caching ([#397](https://github.com/nearai/ferrumyx-runtime-core/pull/397))
- feat(signal) attachment upload  + message tool ([#375](https://github.com/nearai/ferrumyx-runtime-core/pull/375))

### Fixed

- *(channels)* add host-based credential injection to WASM channel wrapper ([#421](https://github.com/nearai/ferrumyx-runtime-core/pull/421))
- pre-validate Cloudflare tunnel token by spawning cloudflared ([#446](https://github.com/nearai/ferrumyx-runtime-core/pull/446))
- batch of quick fixes (#417, #338, #330, #358, #419, #344) ([#428](https://github.com/nearai/ferrumyx-runtime-core/pull/428))
- persist channel activation state across restarts ([#432](https://github.com/nearai/ferrumyx-runtime-core/pull/432))
- init WASM runtime eagerly regardless of tools directory existence ([#401](https://github.com/nearai/ferrumyx-runtime-core/pull/401))
- add TLS support for PostgreSQL connections ([#363](https://github.com/nearai/ferrumyx-runtime-core/pull/363)) ([#427](https://github.com/nearai/ferrumyx-runtime-core/pull/427))
- scan inbound messages for leaked secrets ([#433](https://github.com/nearai/ferrumyx-runtime-core/pull/433))
- use tailscale funnel --bg for proper tunnel setup ([#430](https://github.com/nearai/ferrumyx-runtime-core/pull/430))
- normalize secret names to lowercase for case-insensitive matching ([#413](https://github.com/nearai/ferrumyx-runtime-core/pull/413)) ([#431](https://github.com/nearai/ferrumyx-runtime-core/pull/431))
- persist model name to .env so dotted names survive restart ([#426](https://github.com/nearai/ferrumyx-runtime-core/pull/426))
- *(setup)* check cloudflared binary and validate tunnel token ([#424](https://github.com/nearai/ferrumyx-runtime-core/pull/424))
- *(setup)* validate PostgreSQL version and pgvector availability before migrations ([#423](https://github.com/nearai/ferrumyx-runtime-core/pull/423))
- guard zsh compdef call to prevent error before compinit ([#422](https://github.com/nearai/ferrumyx-runtime-core/pull/422))
- *(telegram)* remove restart button, validate token on setup ([#434](https://github.com/nearai/ferrumyx-runtime-core/pull/434))
- web UI routines tab shows all routines regardless of creating channel ([#391](https://github.com/nearai/ferrumyx-runtime-core/pull/391))
- Discord Ed25519 signature verification and capabilities header alias ([#148](https://github.com/nearai/ferrumyx-runtime-core/pull/148)) ([#372](https://github.com/nearai/ferrumyx-runtime-core/pull/372))
- prevent duplicate WASM channel activation on startup ([#390](https://github.com/nearai/ferrumyx-runtime-core/pull/390))

### Other

- rename WasmBuildable::repo_url to source_dir ([#445](https://github.com/nearai/ferrumyx-runtime-core/pull/445))
- Improve --help: add detailed about/examples/color, snapshot test (clo… ([#371](https://github.com/nearai/ferrumyx-runtime-core/pull/371))
- Add automated QA: schema validator, CI matrix, Docker build, and P1 test coverage ([#353](https://github.com/nearai/ferrumyx-runtime-core/pull/353))

## [0.12.0](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.11.1...v0.12.0) - 2026-02-26

### Added

- *(web)* improve WASM channel setup flow ([#380](https://github.com/nearai/ferrumyx-runtime-core/pull/380))
- *(web)* inline tool activity cards with auto-collapsing ([#376](https://github.com/nearai/ferrumyx-runtime-core/pull/376))
- *(web)* display logs newest-first in web gateway UI ([#369](https://github.com/nearai/ferrumyx-runtime-core/pull/369))
- *(signal)* tool approval workflow and status updates ([#350](https://github.com/nearai/ferrumyx-runtime-core/pull/350))
- add OpenRouter preset to setup wizard ([#270](https://github.com/nearai/ferrumyx-runtime-core/pull/270))
- *(channels)* add native Signal channel via signal-cli HTTP daemon ([#271](https://github.com/nearai/ferrumyx-runtime-core/pull/271))

### Fixed

- correct MCP registry URLs and remove non-existent Google endpoints ([#370](https://github.com/nearai/ferrumyx-runtime-core/pull/370))
- resolve_thread adopts existing session threads by UUID ([#377](https://github.com/nearai/ferrumyx-runtime-core/pull/377))
- resolve telegram/slack name collision between tool and channel registries ([#346](https://github.com/nearai/ferrumyx-runtime-core/pull/346))
- make onboarding installs prefer release artifacts with source fallback ([#323](https://github.com/nearai/ferrumyx-runtime-core/pull/323))
- copy missing files in Dockerfile to fix build ([#322](https://github.com/nearai/ferrumyx-runtime-core/pull/322))
- fall back to build-from-source when extension download fails ([#312](https://github.com/nearai/ferrumyx-runtime-core/pull/312))

### Other

- Add --version flag with clap built-in support and test ([#342](https://github.com/nearai/ferrumyx-runtime-core/pull/342))
- Update FEATURE_PARITY.md ([#337](https://github.com/nearai/ferrumyx-runtime-core/pull/337))
- add brew install ferrumyx-runtime-core instructions ([#310](https://github.com/nearai/ferrumyx-runtime-core/pull/310))
- Fix skills system: enable by default, fix registry and install ([#300](https://github.com/nearai/ferrumyx-runtime-core/pull/300))

## [0.11.1](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.11.0...v0.11.1) - 2026-02-23

### Other

- Ignore out-of-date generated CI so custom release.yml jobs are allowed

## [0.11.0](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.10.0...v0.11.0) - 2026-02-23

### Fixed

- auto-compact and retry on ContextLengthExceeded ([#315](https://github.com/nearai/ferrumyx-runtime-core/pull/315))

### Other

- *(README)* Adding badges to readme ([#316](https://github.com/nearai/ferrumyx-runtime-core/pull/316))
- Feat/completion ([#240](https://github.com/nearai/ferrumyx-runtime-core/pull/240))

## [0.10.0](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.9.0...v0.10.0) - 2026-02-22

### Added

- update dashboard favicon ([#309](https://github.com/nearai/ferrumyx-runtime-core/pull/309))
- add web UI test skill for Chrome extension ([#302](https://github.com/nearai/ferrumyx-runtime-core/pull/302))
- implement FullJob routine mode with scheduler dispatch ([#288](https://github.com/nearai/ferrumyx-runtime-core/pull/288))
- hot-activate WASM channels, channel-first prompts, unified artifact resolution ([#297](https://github.com/nearai/ferrumyx-runtime-core/pull/297))
- add pairing/permission system to all WASM channels and fix extension registry ([#286](https://github.com/nearai/ferrumyx-runtime-core/pull/286))
- group chat privacy, channel-aware prompts, and safety hardening ([#285](https://github.com/nearai/ferrumyx-runtime-core/pull/285))
- embedded registry catalog and WASM bundle install pipeline ([#283](https://github.com/nearai/ferrumyx-runtime-core/pull/283))
- show token usage and cost tracker in gateway status popover ([#284](https://github.com/nearai/ferrumyx-runtime-core/pull/284))
- support custom HTTP headers for OpenAI-compatible provider ([#269](https://github.com/nearai/ferrumyx-runtime-core/pull/269))
- add smart routing provider for cost-optimized model selection ([#281](https://github.com/nearai/ferrumyx-runtime-core/pull/281))

### Fixed

- persist user message at turn start before agentic loop ([#305](https://github.com/nearai/ferrumyx-runtime-core/pull/305))
- block send until thread is selected ([#306](https://github.com/nearai/ferrumyx-runtime-core/pull/306))
- reload chat history on SSE reconnect ([#307](https://github.com/nearai/ferrumyx-runtime-core/pull/307))
- map Esc to interrupt and Ctrl+C to graceful quit ([#267](https://github.com/nearai/ferrumyx-runtime-core/pull/267))

### Other

- Fix tool schema OpenAI compatibility ([#301](https://github.com/nearai/ferrumyx-runtime-core/pull/301))
- simplify config resolution and consolidate main.rs init ([#287](https://github.com/nearai/ferrumyx-runtime-core/pull/287))
- Update image source in README.md
- Add files via upload
- remove ExtensionSource::Bundled, use download-only install for WASM channels ([#293](https://github.com/nearai/ferrumyx-runtime-core/pull/293))
- allow OAuth callback to work on remote servers (fixes #186) ([#212](https://github.com/nearai/ferrumyx-runtime-core/pull/212))
- add rate limiting for built-in tools (closes #171) ([#276](https://github.com/nearai/ferrumyx-runtime-core/pull/276))
- add LLM providers guide (OpenRouter, Together AI, Fireworks, Ollama, vLLM) ([#193](https://github.com/nearai/ferrumyx-runtime-core/pull/193))
- Feat/html to markdown #106  ([#115](https://github.com/nearai/ferrumyx-runtime-core/pull/115))
- adopt agent-market design language for web UI ([#282](https://github.com/nearai/ferrumyx-runtime-core/pull/282))
- speed up startup from ~15s to ~2s ([#280](https://github.com/nearai/ferrumyx-runtime-core/pull/280))
- consolidate tool approval into single param-aware method ([#274](https://github.com/nearai/ferrumyx-runtime-core/pull/274))

## [0.9.0](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.8.0...v0.9.0) - 2026-02-21

### Added

- add TEE attestation shield to web gateway UI ([#275](https://github.com/nearai/ferrumyx-runtime-core/pull/275))
- configurable tool iterations, auto-approve, and policy fix ([#251](https://github.com/nearai/ferrumyx-runtime-core/pull/251))

### Fixed

- add X-Accel-Buffering header to SSE endpoints ([#277](https://github.com/nearai/ferrumyx-runtime-core/pull/277))

## [0.8.0](https://github.com/nearai/ferrumyx-runtime-core/compare/ferrumyx-runtime-core-v0.7.0...ferrumyx-runtime-core-v0.8.0) - 2026-02-20

### Added

- extension registry with metadata catalog and onboarding integration ([#238](https://github.com/nearai/ferrumyx-runtime-core/pull/238))
- *(models)* add GPT-5.3 Codex, full GPT-5.x family, Claude 4.x series, o4-mini ([#197](https://github.com/nearai/ferrumyx-runtime-core/pull/197))
- wire memory hygiene into the heartbeat loop ([#195](https://github.com/nearai/ferrumyx-runtime-core/pull/195))

### Fixed

- persist WASM channel workspace writes across callbacks ([#264](https://github.com/nearai/ferrumyx-runtime-core/pull/264))
- consolidate per-module ENV_MUTEX into crate-wide test lock ([#246](https://github.com/nearai/ferrumyx-runtime-core/pull/246))
- remove auto-proceed fake user message injection from agent loop ([#255](https://github.com/nearai/ferrumyx-runtime-core/pull/255))
- onboarding errors reset flow and remote server auth (#185, #186) ([#248](https://github.com/nearai/ferrumyx-runtime-core/pull/248))
- parallelize tool call execution via JoinSet ([#219](https://github.com/nearai/ferrumyx-runtime-core/pull/219)) ([#252](https://github.com/nearai/ferrumyx-runtime-core/pull/252))
- prevent pipe deadlock in shell command execution ([#140](https://github.com/nearai/ferrumyx-runtime-core/pull/140))
- persist turns after approval and add agent-level tests ([#250](https://github.com/nearai/ferrumyx-runtime-core/pull/250))

### Other

- add automated PR labeling system ([#253](https://github.com/nearai/ferrumyx-runtime-core/pull/253))
- update CLAUDE.md for recently merged features ([#183](https://github.com/nearai/ferrumyx-runtime-core/pull/183))

## [0.7.0](https://github.com/nearai/ferrumyx-runtime-core/compare/ferrumyx-runtime-core-v0.6.0...ferrumyx-runtime-core-v0.7.0) - 2026-02-19

### Added

- extend lifecycle hooks with declarative bundles ([#176](https://github.com/nearai/ferrumyx-runtime-core/pull/176))
- support per-request model override in /v1/chat/completions ([#103](https://github.com/nearai/ferrumyx-runtime-core/pull/103))

### Fixed

- harden openai-compatible provider, approval replay, and embeddings defaults ([#237](https://github.com/nearai/ferrumyx-runtime-core/pull/237))
- Network Security Findings ([#201](https://github.com/nearai/ferrumyx-runtime-core/pull/201))

### Added

- Refactored OpenAI-compatible chat completion routing to use the rig adapter and `RetryProvider` composition for custom base URL usage.
- Added Ollama embeddings provider support (`EMBEDDING_PROVIDER=ollama`, `OLLAMA_BASE_URL`) in workspace embeddings.
- Added migration `V9__flexible_embedding_dimension.sql` for flexible embedding vector dimensions.

### Changed

- Changed default sandbox image to `ferrumyx-runtime-core-worker:latest` in config/settings/sandbox defaults.
- Improved tool-message sanitization and provider compatibility handling across NEAR AI, rig adapter, and shared LLM provider code.

### Fixed

- Fixed approval-input aliases (`a`, `/approve`, `/always`, `/deny`, etc.) in submission parsing.
- Fixed multi-tool approval resume flow by preserving and replaying deferred tool calls so all prior `tool_use` IDs receive matching `tool_result` messages.
- Fixed REPL quit/exit handling to route shutdown through the agent loop for graceful termination.

## [0.6.0](https://github.com/nearai/ferrumyx-runtime-core/compare/ferrumyx-runtime-core-v0.5.0...ferrumyx-runtime-core-v0.6.0) - 2026-02-19

### Added

- add issue triage skill ([#200](https://github.com/nearai/ferrumyx-runtime-core/pull/200))
- add PR triage dashboard skill ([#196](https://github.com/nearai/ferrumyx-runtime-core/pull/196))
- add OpenRouter usage examples ([#189](https://github.com/nearai/ferrumyx-runtime-core/pull/189))
- add Tinfoil private inference provider ([#62](https://github.com/nearai/ferrumyx-runtime-core/pull/62))
- shell env scrubbing and command injection detection ([#164](https://github.com/nearai/ferrumyx-runtime-core/pull/164))
- Add PR review tools, job monitor, and channel injection for E2E sandbox workflows ([#57](https://github.com/nearai/ferrumyx-runtime-core/pull/57))
- Secure prompt-based skills system (Phases 1-4) ([#51](https://github.com/nearai/ferrumyx-runtime-core/pull/51))
- Add benchmarking harness with spot suite ([#10](https://github.com/nearai/ferrumyx-runtime-core/pull/10))
- 10 infrastructure improvements from zeroclaw ([#126](https://github.com/nearai/ferrumyx-runtime-core/pull/126))

### Fixed

- *(rig)* prevent OpenAI Responses API panic on tool call IDs ([#182](https://github.com/nearai/ferrumyx-runtime-core/pull/182))
- *(docs)* correct settings storage path in README ([#194](https://github.com/nearai/ferrumyx-runtime-core/pull/194))
- OpenAI tool calling — schema normalization, missing types, and Responses API panic ([#132](https://github.com/nearai/ferrumyx-runtime-core/pull/132))
- *(security)* prevent path traversal bypass in WASM HTTP allowlist ([#137](https://github.com/nearai/ferrumyx-runtime-core/pull/137))
- persist OpenAI-compatible provider and respect embeddings disable ([#177](https://github.com/nearai/ferrumyx-runtime-core/pull/177))
- remove .expect() calls in FailoverProvider::try_providers ([#156](https://github.com/nearai/ferrumyx-runtime-core/pull/156))
- sentinel value collision in FailoverProvider cooldown ([#125](https://github.com/nearai/ferrumyx-runtime-core/pull/125)) ([#154](https://github.com/nearai/ferrumyx-runtime-core/pull/154))
- skills module audit cleanup ([#173](https://github.com/nearai/ferrumyx-runtime-core/pull/173))

### Other

- Fix division by zero panic in ValueEstimator::is_profitable ([#139](https://github.com/nearai/ferrumyx-runtime-core/pull/139))
- audit feature parity matrix against codebase and recent commits ([#202](https://github.com/nearai/ferrumyx-runtime-core/pull/202))
- architecture improvements for contributor velocity ([#198](https://github.com/nearai/ferrumyx-runtime-core/pull/198))
- fix rustfmt formatting from PR #137
- add .env.example examples for Ollama and OpenAI-compatible ([#110](https://github.com/nearai/ferrumyx-runtime-core/pull/110))

## [0.5.0](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.4.0...v0.5.0) - 2026-02-17

### Added

- add cooldown management to FailoverProvider ([#114](https://github.com/nearai/ferrumyx-runtime-core/pull/114))

## [0.4.0](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.3.0...v0.4.0) - 2026-02-17

### Added

- move per-invocation approval check into Tool trait ([#119](https://github.com/nearai/ferrumyx-runtime-core/pull/119))
- add polished boot screen on CLI startup ([#118](https://github.com/nearai/ferrumyx-runtime-core/pull/118))
- Add lifecycle hooks system with 6 interception points ([#18](https://github.com/nearai/ferrumyx-runtime-core/pull/18))

### Other

- remove accidentally committed .sidecar and .todos directories ([#123](https://github.com/nearai/ferrumyx-runtime-core/pull/123))

## [0.3.0](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.2.0...v0.3.0) - 2026-02-17

### Added

- direct api key and cheap model ([#116](https://github.com/nearai/ferrumyx-runtime-core/pull/116))

## [0.2.0](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.1.3...v0.2.0) - 2026-02-16

### Added

- mark Ollama + OpenAI-compatible as implemented ([#102](https://github.com/nearai/ferrumyx-runtime-core/pull/102))
- multi-provider inference + libSQL onboarding selection ([#92](https://github.com/nearai/ferrumyx-runtime-core/pull/92))
- add multi-provider LLM failover with retry backoff ([#28](https://github.com/nearai/ferrumyx-runtime-core/pull/28))
- add libSQL/Turso embedded database backend ([#47](https://github.com/nearai/ferrumyx-runtime-core/pull/47))
- Move debug log truncation from agent loop to REPL channel ([#65](https://github.com/nearai/ferrumyx-runtime-core/pull/65))

### Fixed

- shell destructive-command check bypassed by Value::Object arguments ([#72](https://github.com/nearai/ferrumyx-runtime-core/pull/72))
- propagate real tool_call_id instead of hardcoded placeholder ([#73](https://github.com/nearai/ferrumyx-runtime-core/pull/73))
- Fix wasm tool schemas and runtime ([#42](https://github.com/nearai/ferrumyx-runtime-core/pull/42))
- flatten tool messages for NEAR AI cloud-api compatibility ([#41](https://github.com/nearai/ferrumyx-runtime-core/pull/41))
- security hardening across all layers ([#35](https://github.com/nearai/ferrumyx-runtime-core/pull/35))

### Other

- Explicitly enable cargo-dist caching for binary artifacts building
- Skip building binary artifacts on every PR
- add module specification rules to CLAUDE.md
- add setup/onboarding specification (src/setup/README.md)
- deduplicate tool code and remove dead stubs ([#98](https://github.com/nearai/ferrumyx-runtime-core/pull/98))
- Reformat architecture diagram in README ([#64](https://github.com/nearai/ferrumyx-runtime-core/pull/64))
- Add review discipline guidelines to CLAUDE.md ([#68](https://github.com/nearai/ferrumyx-runtime-core/pull/68))
- Bump MSRV to 1.92, add GCP deployment files ([#40](https://github.com/nearai/ferrumyx-runtime-core/pull/40))
- Add OpenAI-compatible HTTP API (/v1/chat/completions, /v1/models)   ([#31](https://github.com/nearai/ferrumyx-runtime-core/pull/31))


## [0.1.3](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.1.2...v0.1.3) - 2026-02-12

### Other

- Enabled builds caching during CI/CD
- Disabled npm publishing as the name is already taken

## [0.1.2](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.1.1...v0.1.2) - 2026-02-12

### Other

- Added Installation instructions for the pre-built binaries
- Disabled Windows ARM64 builds as auto-updater [provided by cargo-dist] does not support this platform yet and it is not a common platform for us to support

## [0.1.1](https://github.com/nearai/ferrumyx-runtime-core/compare/v0.1.0...v0.1.1) - 2026-02-12

### Other

- Renamed the secrets in release-plz.yml to match the configuration
- Make sure that the binaries release CD it kicking in after release-plz

## [0.1.0](https://github.com/nearai/ferrumyx-runtime-core/releases/tag/v0.1.0) - 2026-02-12

### Added

- Add multi-provider LLM support via rig-core adapter ([#36](https://github.com/nearai/ferrumyx-runtime-core/pull/36))
- Sandbox jobs ([#4](https://github.com/nearai/ferrumyx-runtime-core/pull/4))
- Add Google Suite & Telegram WASM tools ([#9](https://github.com/nearai/ferrumyx-runtime-core/pull/9))
- Improve CLI ([#5](https://github.com/nearai/ferrumyx-runtime-core/pull/5))

### Fixed

- resolve runtime panic in Linux keychain integration ([#32](https://github.com/nearai/ferrumyx-runtime-core/pull/32))

### Other

- Skip release-plz on forks
- Upgraded release-plz CD pipeline
- Added CI/CD and release pipelines ([#45](https://github.com/nearai/ferrumyx-runtime-core/pull/45))
- DM pairing + Telegram channel improvements ([#17](https://github.com/nearai/ferrumyx-runtime-core/pull/17))
- Fixes build, adds missing sse event and correct command ([#11](https://github.com/nearai/ferrumyx-runtime-core/pull/11))
- Codex/feature parity pr hook ([#6](https://github.com/nearai/ferrumyx-runtime-core/pull/6))
- Add WebSocket gateway and control plane ([#8](https://github.com/nearai/ferrumyx-runtime-core/pull/8))
- select bundled Telegram channel and auto-install ([#3](https://github.com/nearai/ferrumyx-runtime-core/pull/3))
- Adding skills for reusable work
- Fix MCP tool calls, approval loop, shutdown, and improve web UI
- Add auth mode, fix MCP token handling, and parallelize startup loading
- Merge remote-tracking branch 'origin/main' into ui
- Adding web UI
- Rename `setup` CLI command to `onboard` for compatibility
- Add in-chat extension discovery, auth, and activation system
- Add Telegram typing indicator via WIT on-status callback
- Add proactivity features: memory CLI, session pruning, self-repair notifications, slash commands, status diagnostics, context warnings
- Add hosted MCP server support with OAuth 2.1 and token refresh
- Add interactive setup wizard and persistent settings
- Rebrand to Ferrumyx Runtime Core with security-first mission
- Fix build_software tool stuck in planning mode loop
- Enable sandbox by default
- Fix Telegram Markdown formatting and clarify tool/memory distinctions
- Simplify Telegram channel config with host-injected tunnel/webhook settings
- Apply Telegram channel learnings to WhatsApp implementation
- Merge remote-tracking branch 'origin/main'
- Docker file for sandbox
- Replace hardcoded intent patterns with job tools
- Fix router test to match intentional job creation patterns
- Add Docker execution sandbox for secure shell command isolation
- Move setup wizard credentials to database storage
- Add interactive setup wizard for first-run configuration
- Add Telegram Bot API channel as WASM module
- Add OpenClaw feature parity tracking matrix
- Add Chat Completions API support and expand REPL debugging
- Implementing channels to be handled in wasm
- Support non interactive mode and model selection
- Implement tool approval, fix tool definition refresh, and wire embeddings
- Tool use
- Wiring more
- Add heartbeat integration, planning phase, and auto-repair
- Login flow
- Extend support for session management
- Adding builder capability
- Load tools at launch
- Fix multiline message rendering in TUI
- Parse NEAR AI alternative response format with output field
- Handle NEAR AI plain text responses
- Disable mouse capture to allow text selection in TUI
- Add verbose logging to debug empty NEAR AI responses
- Improve NEAR AI response parsing for varying response formats
- Show status/thinking messages in chat window, debug empty responses
- Add timeout and logging to NEAR AI provider
- Add status updates to show agent thinking/processing state
- Add CLI subcommands for WASM tool management
- Fix TUI shutdown: send /shutdown message and handle in agent loop
- Remove SimpleCliChannel, add Ctrl+D twice quit, redirect logs to TUI
- Fix TuiChannel integration and enable in main.rs
- Integrate Codex patterns: task scheduler, TUI, sessions, compaction
- Adding LICENSE
- Add README with Ferrumyx Runtime Core branding
- Add WASM sandbox secure API extension
- Wire database Store into agent loop
- Implementing WASM runtime
- Add workspace integration tests
- Compact memory_tree output format
- Replace memory_list with memory_tree tool
- Simplify workspace to path-based storage, remove legacy code
- Add NEAR AI chat-api as default LLM provider
- Add CLAUDE.md project documentation
- Add workspace and memory system (OpenClaw-inspired)
- Initial implementation of the agent framework


