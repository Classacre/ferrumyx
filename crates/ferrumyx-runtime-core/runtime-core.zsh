#compdef ferrumyx-runtime-core

autoload -U is-at-least

_ferrumyx-runtime-core() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
'-V[Print version]' \
'--version[Print version]' \
":: :_ferrumyx-runtime-core_commands" \
"*::: :->ferrumyx-runtime-core" \
&& ret=0
    case $state in
    (ferrumyx-runtime-core)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-command-$line[1]:"
        case $line[1] in
            (run)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
&& ret=0
;;
(onboard)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--skip-auth[Skip authentication (use existing session)]' \
'(--provider-only)--channels-only[Reconfigure channels only]' \
'(--channels-only)--provider-only[Reconfigure LLM provider and model only]' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
&& ret=0
;;
(config)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
":: :_ferrumyx-runtime-core__config_commands" \
"*::: :->config" \
&& ret=0

    case $state in
    (config)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-config-command-$line[1]:"
        case $line[1] in
            (init)
_arguments "${_arguments_options[@]}" : \
'-o+[Output path (default\: ~/.ferrumyx_runtime_core/config.toml)]:OUTPUT:_files' \
'--output=[Output path (default\: ~/.ferrumyx_runtime_core/config.toml)]:OUTPUT:_files' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--force[Overwrite existing file]' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
'-f+[Show only settings matching this prefix (e.g., "agent", "heartbeat")]:FILTER:_default' \
'--filter=[Show only settings matching this prefix (e.g., "agent", "heartbeat")]:FILTER:_default' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(get)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':path -- Setting path (e.g., "agent.max_parallel_jobs"):_default' \
&& ret=0
;;
(set)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':path -- Setting path (e.g., "agent.max_parallel_jobs"):_default' \
':value -- Value to set:_default' \
&& ret=0
;;
(reset)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':path -- Setting path (e.g., "agent.max_parallel_jobs"):_default' \
&& ret=0
;;
(path)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__config__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-config-help-command-$line[1]:"
        case $line[1] in
            (init)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(get)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(set)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(reset)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(path)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(tool)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
":: :_ferrumyx-runtime-core__tool_commands" \
"*::: :->tool" \
&& ret=0

    case $state in
    (tool)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-tool-command-$line[1]:"
        case $line[1] in
            (install)
_arguments "${_arguments_options[@]}" : \
'-n+[Tool name (defaults to directory/file name)]:NAME:_default' \
'--name=[Tool name (defaults to directory/file name)]:NAME:_default' \
'--capabilities=[Path to capabilities JSON file (auto-detected if not specified)]:CAPABILITIES:_files' \
'-t+[Target directory for installation (default\: ~/.ferrumyx_runtime_core/tools/)]:TARGET:_files' \
'--target=[Target directory for installation (default\: ~/.ferrumyx_runtime_core/tools/)]:TARGET:_files' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--release[Build in release mode (default\: true)]' \
'--skip-build[Skip compilation (use existing .wasm file)]' \
'-f[Force overwrite if tool already exists]' \
'--force[Force overwrite if tool already exists]' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':path -- Path to tool source directory (with Cargo.toml) or .wasm file:_files' \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
'-d+[Directory to list tools from (default\: ~/.ferrumyx_runtime_core/tools/)]:DIR:_files' \
'--dir=[Directory to list tools from (default\: ~/.ferrumyx_runtime_core/tools/)]:DIR:_files' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'-v[Show detailed information]' \
'--verbose[Show detailed information]' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
'-d+[Directory to remove tool from (default\: ~/.ferrumyx_runtime_core/tools/)]:DIR:_files' \
'--dir=[Directory to remove tool from (default\: ~/.ferrumyx_runtime_core/tools/)]:DIR:_files' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':name -- Name of the tool to remove:_default' \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
'-d+[Directory to look for tool (default\: ~/.ferrumyx_runtime_core/tools/)]:DIR:_files' \
'--dir=[Directory to look for tool (default\: ~/.ferrumyx_runtime_core/tools/)]:DIR:_files' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':name_or_path -- Name of the tool or path to .wasm file:_default' \
&& ret=0
;;
(auth)
_arguments "${_arguments_options[@]}" : \
'-d+[Directory to look for tool (default\: ~/.ferrumyx_runtime_core/tools/)]:DIR:_files' \
'--dir=[Directory to look for tool (default\: ~/.ferrumyx_runtime_core/tools/)]:DIR:_files' \
'-u+[User ID for storing the secret (default\: "default")]:USER:_default' \
'--user=[User ID for storing the secret (default\: "default")]:USER:_default' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':name -- Name of the tool:_default' \
&& ret=0
;;
(setup)
_arguments "${_arguments_options[@]}" : \
'-d+[Directory to look for tool (default\: ~/.ferrumyx_runtime_core/tools/)]:DIR:_files' \
'--dir=[Directory to look for tool (default\: ~/.ferrumyx_runtime_core/tools/)]:DIR:_files' \
'-u+[User ID for storing the secret (default\: "default")]:USER:_default' \
'--user=[User ID for storing the secret (default\: "default")]:USER:_default' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':name -- Name of the tool:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__tool__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-tool-help-command-$line[1]:"
        case $line[1] in
            (install)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(auth)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(setup)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(registry)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
":: :_ferrumyx-runtime-core__registry_commands" \
"*::: :->registry" \
&& ret=0

    case $state in
    (registry)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-registry-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
'-k+[Filter by kind\: "tool" or "channel"]:KIND:_default' \
'--kind=[Filter by kind\: "tool" or "channel"]:KIND:_default' \
'-t+[Filter by tag (e.g. "default", "google", "messaging")]:TAG:_default' \
'--tag=[Filter by tag (e.g. "default", "google", "messaging")]:TAG:_default' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'-v[Show detailed information]' \
'--verbose[Show detailed information]' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':name -- Extension or bundle name (e.g. "slack", "google", "tools/gmail"):_default' \
&& ret=0
;;
(install)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'-f[Force overwrite if already installed]' \
'--force[Force overwrite if already installed]' \
'--build[Build from source instead of downloading pre-built artifact]' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':name -- Extension or bundle name (e.g. "slack", "google", "default"):_default' \
&& ret=0
;;
(install-defaults)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'-f[Force overwrite if already installed]' \
'--force[Force overwrite if already installed]' \
'--build[Build from source instead of downloading pre-built artifact]' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__registry__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-registry-help-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(install)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(install-defaults)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(mcp)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
":: :_ferrumyx-runtime-core__mcp_commands" \
"*::: :->mcp" \
&& ret=0

    case $state in
    (mcp)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-mcp-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
'--transport=[Transport type\: http (default), stdio, unix]:TRANSPORT:_default' \
'--command=[Command to run (stdio transport)]:COMMAND:_default' \
'*--arg=[Command arguments (stdio transport, can be repeated)]:CMD_ARGS:_default' \
'*--env=[Environment variables (stdio transport, KEY=VALUE format, can be repeated)]:ENV:_default' \
'--socket=[Unix socket path (unix transport)]:SOCKET:_default' \
'*--header=[Custom HTTP headers (KEY\:VALUE format, can be repeated)]:HEADERS:_default' \
'--client-id=[OAuth client ID (if authentication is required)]:CLIENT_ID:_default' \
'--auth-url=[OAuth authorization URL (optional, can be discovered)]:AUTH_URL:_default' \
'--token-url=[OAuth token URL (optional, can be discovered)]:TOKEN_URL:_default' \
'--scopes=[Scopes to request (comma-separated)]:SCOPES:_default' \
'--description=[Server description]:DESCRIPTION:_default' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':name -- Server name (e.g., "notion", "github"):_default' \
'::url -- Server URL (e.g., "https\://mcp.notion.com") -- required for http transport:_default' \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':name -- Server name to remove:_default' \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'-v[Show detailed information]' \
'--verbose[Show detailed information]' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(auth)
_arguments "${_arguments_options[@]}" : \
'-u+[User ID for storing the token (default\: "default")]:USER:_default' \
'--user=[User ID for storing the token (default\: "default")]:USER:_default' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':name -- Server name to authenticate:_default' \
&& ret=0
;;
(test)
_arguments "${_arguments_options[@]}" : \
'-u+[User ID for authentication (default\: "default")]:USER:_default' \
'--user=[User ID for authentication (default\: "default")]:USER:_default' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':name -- Server name to test:_default' \
&& ret=0
;;
(toggle)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'(--disable)--enable[Enable the server]' \
'(--enable)--disable[Disable the server]' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':name -- Server name:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__mcp__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-mcp-help-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(auth)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(test)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(toggle)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(memory)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
":: :_ferrumyx-runtime-core__memory_commands" \
"*::: :->memory" \
&& ret=0

    case $state in
    (memory)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-memory-command-$line[1]:"
        case $line[1] in
            (search)
_arguments "${_arguments_options[@]}" : \
'-l+[Maximum number of results]:LIMIT:_default' \
'--limit=[Maximum number of results]:LIMIT:_default' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':query -- Search query:_default' \
&& ret=0
;;
(read)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':path -- File path (e.g., "MEMORY.md", "daily/2024-01-15.md"):_default' \
&& ret=0
;;
(write)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'-a[Append instead of overwrite]' \
'--append[Append instead of overwrite]' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':path -- File path (e.g., "notes/idea.md"):_default' \
'::content -- Content to write (omit to read from stdin):_default' \
&& ret=0
;;
(tree)
_arguments "${_arguments_options[@]}" : \
'-d+[Maximum depth to traverse]:DEPTH:_default' \
'--depth=[Maximum depth to traverse]:DEPTH:_default' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
'::path -- Root path to start from:_default' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__memory__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-memory-help-command-$line[1]:"
        case $line[1] in
            (search)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(read)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(write)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(tree)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(pairing)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
":: :_ferrumyx-runtime-core__pairing_commands" \
"*::: :->pairing" \
&& ret=0

    case $state in
    (pairing)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-pairing-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--json[Output as JSON]' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':channel -- Channel name (e.g., telegram, slack):_default' \
&& ret=0
;;
(approve)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
':channel -- Channel name (e.g., telegram, slack):_default' \
':code -- Pairing code (e.g., ABC12345):_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__pairing__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-pairing-help-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(approve)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(service)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
":: :_ferrumyx-runtime-core__service_commands" \
"*::: :->service" \
&& ret=0

    case $state in
    (service)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-service-command-$line[1]:"
        case $line[1] in
            (install)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(start)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(stop)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(uninstall)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__service__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-service-help-command-$line[1]:"
        case $line[1] in
            (install)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(start)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(stop)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(uninstall)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(doctor)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
&& ret=0
;;
(completion)
_arguments "${_arguments_options[@]}" : \
'--shell=[The shell to generate completions for]:SHELL:(bash elvish fish powershell zsh)' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
&& ret=0
;;
(worker)
_arguments "${_arguments_options[@]}" : \
'--job-id=[Job ID to execute]:JOB_ID:_default' \
'--orchestrator-url=[URL of the orchestrator'\''s internal API]:ORCHESTRATOR_URL:_default' \
'--max-iterations=[Maximum iterations before stopping]:MAX_ITERATIONS:_default' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(claude-bridge)
_arguments "${_arguments_options[@]}" : \
'--job-id=[Job ID to execute]:JOB_ID:_default' \
'--orchestrator-url=[URL of the orchestrator'\''s internal API]:ORCHESTRATOR_URL:_default' \
'--max-turns=[Maximum agentic turns for Claude Code]:MAX_TURNS:_default' \
'--model=[Claude model to use (e.g. "sonnet", "opus")]:MODEL:_default' \
'-m+[Single message mode - send one message and exit]:MESSAGE:_default' \
'--message=[Single message mode - send one message and exit]:MESSAGE:_default' \
'-c+[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--config=[Configuration file path (optional, uses env vars by default)]:CONFIG:_files' \
'--cli-only[Run in interactive CLI mode only (disable other channels)]' \
'--no-db[Skip database connection (for testing)]' \
'--no-onboard[Skip first-run onboarding check]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-help-command-$line[1]:"
        case $line[1] in
            (run)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(onboard)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(config)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__help__config_commands" \
"*::: :->config" \
&& ret=0

    case $state in
    (config)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-help-config-command-$line[1]:"
        case $line[1] in
            (init)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(get)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(set)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(reset)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(path)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(tool)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__help__tool_commands" \
"*::: :->tool" \
&& ret=0

    case $state in
    (tool)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-help-tool-command-$line[1]:"
        case $line[1] in
            (install)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(auth)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(setup)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(registry)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__help__registry_commands" \
"*::: :->registry" \
&& ret=0

    case $state in
    (registry)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-help-registry-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(install)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(install-defaults)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(mcp)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__help__mcp_commands" \
"*::: :->mcp" \
&& ret=0

    case $state in
    (mcp)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-help-mcp-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(auth)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(test)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(toggle)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(memory)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__help__memory_commands" \
"*::: :->memory" \
&& ret=0

    case $state in
    (memory)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-help-memory-command-$line[1]:"
        case $line[1] in
            (search)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(read)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(write)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(tree)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(pairing)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__help__pairing_commands" \
"*::: :->pairing" \
&& ret=0

    case $state in
    (pairing)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-help-pairing-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(approve)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(service)
_arguments "${_arguments_options[@]}" : \
":: :_ferrumyx-runtime-core__help__service_commands" \
"*::: :->service" \
&& ret=0

    case $state in
    (service)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ferrumyx-runtime-core-help-service-command-$line[1]:"
        case $line[1] in
            (install)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(start)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(stop)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(uninstall)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(doctor)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(status)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(completion)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(worker)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(claude-bridge)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
}

(( $+functions[_ferrumyx-runtime-core_commands] )) ||
_ferrumyx-runtime-core_commands() {
    local commands; commands=(
'run:Run the AI agent' \
'onboard:Run interactive setup wizard' \
'config:Manage app configs' \
'tool:Manage WASM tools' \
'registry:Browse/install extensions' \
'mcp:Manage MCP servers' \
'memory:Manage workspace memory' \
'pairing:Manage DM pairing' \
'service:Manage OS service' \
'doctor:Run diagnostics' \
'status:Show system status' \
'completion:Generate completions' \
'worker:Run as a sandboxed worker inside a Docker container (internal use). This is invoked automatically by the orchestrator, not by users directly' \
'claude-bridge:Run as a Claude Code bridge inside a Docker container (internal use). Spawns the \`claude\` CLI and streams output back to the orchestrator' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__claude-bridge_commands] )) ||
_ferrumyx-runtime-core__claude-bridge_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core claude-bridge commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__completion_commands] )) ||
_ferrumyx-runtime-core__completion_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core completion commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config_commands] )) ||
_ferrumyx-runtime-core__config_commands() {
    local commands; commands=(
'init:Generate a default config.toml file' \
'list:List all settings and their current values' \
'get:Get a specific setting value' \
'set:Set a setting value' \
'reset:Reset a setting to its default value' \
'path:Show the settings storage info' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core config commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config__get_commands] )) ||
_ferrumyx-runtime-core__config__get_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core config get commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config__help_commands] )) ||
_ferrumyx-runtime-core__config__help_commands() {
    local commands; commands=(
'init:Generate a default config.toml file' \
'list:List all settings and their current values' \
'get:Get a specific setting value' \
'set:Set a setting value' \
'reset:Reset a setting to its default value' \
'path:Show the settings storage info' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core config help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config__help__get_commands] )) ||
_ferrumyx-runtime-core__config__help__get_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core config help get commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config__help__help_commands] )) ||
_ferrumyx-runtime-core__config__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core config help help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config__help__init_commands] )) ||
_ferrumyx-runtime-core__config__help__init_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core config help init commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config__help__list_commands] )) ||
_ferrumyx-runtime-core__config__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core config help list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config__help__path_commands] )) ||
_ferrumyx-runtime-core__config__help__path_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core config help path commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config__help__reset_commands] )) ||
_ferrumyx-runtime-core__config__help__reset_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core config help reset commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config__help__set_commands] )) ||
_ferrumyx-runtime-core__config__help__set_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core config help set commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config__init_commands] )) ||
_ferrumyx-runtime-core__config__init_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core config init commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config__list_commands] )) ||
_ferrumyx-runtime-core__config__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core config list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config__path_commands] )) ||
_ferrumyx-runtime-core__config__path_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core config path commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config__reset_commands] )) ||
_ferrumyx-runtime-core__config__reset_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core config reset commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__config__set_commands] )) ||
_ferrumyx-runtime-core__config__set_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core config set commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__doctor_commands] )) ||
_ferrumyx-runtime-core__doctor_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core doctor commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help_commands] )) ||
_ferrumyx-runtime-core__help_commands() {
    local commands; commands=(
'run:Run the AI agent' \
'onboard:Run interactive setup wizard' \
'config:Manage app configs' \
'tool:Manage WASM tools' \
'registry:Browse/install extensions' \
'mcp:Manage MCP servers' \
'memory:Manage workspace memory' \
'pairing:Manage DM pairing' \
'service:Manage OS service' \
'doctor:Run diagnostics' \
'status:Show system status' \
'completion:Generate completions' \
'worker:Run as a sandboxed worker inside a Docker container (internal use). This is invoked automatically by the orchestrator, not by users directly' \
'claude-bridge:Run as a Claude Code bridge inside a Docker container (internal use). Spawns the \`claude\` CLI and streams output back to the orchestrator' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__claude-bridge_commands] )) ||
_ferrumyx-runtime-core__help__claude-bridge_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help claude-bridge commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__completion_commands] )) ||
_ferrumyx-runtime-core__help__completion_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help completion commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__config_commands] )) ||
_ferrumyx-runtime-core__help__config_commands() {
    local commands; commands=(
'init:Generate a default config.toml file' \
'list:List all settings and their current values' \
'get:Get a specific setting value' \
'set:Set a setting value' \
'reset:Reset a setting to its default value' \
'path:Show the settings storage info' \
    )
    _describe -t commands 'ferrumyx-runtime-core help config commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__config__get_commands] )) ||
_ferrumyx-runtime-core__help__config__get_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help config get commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__config__init_commands] )) ||
_ferrumyx-runtime-core__help__config__init_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help config init commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__config__list_commands] )) ||
_ferrumyx-runtime-core__help__config__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help config list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__config__path_commands] )) ||
_ferrumyx-runtime-core__help__config__path_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help config path commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__config__reset_commands] )) ||
_ferrumyx-runtime-core__help__config__reset_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help config reset commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__config__set_commands] )) ||
_ferrumyx-runtime-core__help__config__set_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help config set commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__doctor_commands] )) ||
_ferrumyx-runtime-core__help__doctor_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help doctor commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__help_commands] )) ||
_ferrumyx-runtime-core__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__mcp_commands] )) ||
_ferrumyx-runtime-core__help__mcp_commands() {
    local commands; commands=(
'add:Add an MCP server' \
'remove:Remove an MCP server' \
'list:List configured MCP servers' \
'auth:Authenticate with an MCP server (OAuth flow)' \
'test:Test connection to an MCP server' \
'toggle:Enable or disable an MCP server' \
    )
    _describe -t commands 'ferrumyx-runtime-core help mcp commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__mcp__add_commands] )) ||
_ferrumyx-runtime-core__help__mcp__add_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help mcp add commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__mcp__auth_commands] )) ||
_ferrumyx-runtime-core__help__mcp__auth_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help mcp auth commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__mcp__list_commands] )) ||
_ferrumyx-runtime-core__help__mcp__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help mcp list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__mcp__remove_commands] )) ||
_ferrumyx-runtime-core__help__mcp__remove_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help mcp remove commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__mcp__test_commands] )) ||
_ferrumyx-runtime-core__help__mcp__test_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help mcp test commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__mcp__toggle_commands] )) ||
_ferrumyx-runtime-core__help__mcp__toggle_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help mcp toggle commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__memory_commands] )) ||
_ferrumyx-runtime-core__help__memory_commands() {
    local commands; commands=(
'search:Search workspace memory (hybrid full-text + semantic)' \
'read:Read a file from the workspace' \
'write:Write content to a workspace file' \
'tree:Show workspace directory tree' \
'status:Show workspace status (document count, index health)' \
    )
    _describe -t commands 'ferrumyx-runtime-core help memory commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__memory__read_commands] )) ||
_ferrumyx-runtime-core__help__memory__read_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help memory read commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__memory__search_commands] )) ||
_ferrumyx-runtime-core__help__memory__search_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help memory search commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__memory__status_commands] )) ||
_ferrumyx-runtime-core__help__memory__status_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help memory status commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__memory__tree_commands] )) ||
_ferrumyx-runtime-core__help__memory__tree_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help memory tree commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__memory__write_commands] )) ||
_ferrumyx-runtime-core__help__memory__write_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help memory write commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__onboard_commands] )) ||
_ferrumyx-runtime-core__help__onboard_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help onboard commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__pairing_commands] )) ||
_ferrumyx-runtime-core__help__pairing_commands() {
    local commands; commands=(
'list:List pending pairing requests' \
'approve:Approve a pairing request by code' \
    )
    _describe -t commands 'ferrumyx-runtime-core help pairing commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__pairing__approve_commands] )) ||
_ferrumyx-runtime-core__help__pairing__approve_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help pairing approve commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__pairing__list_commands] )) ||
_ferrumyx-runtime-core__help__pairing__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help pairing list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__registry_commands] )) ||
_ferrumyx-runtime-core__help__registry_commands() {
    local commands; commands=(
'list:List available extensions in the registry' \
'info:Show detailed information about an extension or bundle' \
'install:Install an extension or bundle from the registry' \
'install-defaults:Install the default bundle of recommended extensions' \
    )
    _describe -t commands 'ferrumyx-runtime-core help registry commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__registry__info_commands] )) ||
_ferrumyx-runtime-core__help__registry__info_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help registry info commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__registry__install_commands] )) ||
_ferrumyx-runtime-core__help__registry__install_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help registry install commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__registry__install-defaults_commands] )) ||
_ferrumyx-runtime-core__help__registry__install-defaults_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help registry install-defaults commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__registry__list_commands] )) ||
_ferrumyx-runtime-core__help__registry__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help registry list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__run_commands] )) ||
_ferrumyx-runtime-core__help__run_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help run commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__service_commands] )) ||
_ferrumyx-runtime-core__help__service_commands() {
    local commands; commands=(
'install:Install the OS service (launchd on macOS, systemd on Linux)' \
'start:Start the installed service' \
'stop:Stop the running service' \
'status:Show service status' \
'uninstall:Uninstall the OS service and remove the unit file' \
    )
    _describe -t commands 'ferrumyx-runtime-core help service commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__service__install_commands] )) ||
_ferrumyx-runtime-core__help__service__install_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help service install commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__service__start_commands] )) ||
_ferrumyx-runtime-core__help__service__start_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help service start commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__service__status_commands] )) ||
_ferrumyx-runtime-core__help__service__status_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help service status commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__service__stop_commands] )) ||
_ferrumyx-runtime-core__help__service__stop_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help service stop commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__service__uninstall_commands] )) ||
_ferrumyx-runtime-core__help__service__uninstall_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help service uninstall commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__status_commands] )) ||
_ferrumyx-runtime-core__help__status_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help status commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__tool_commands] )) ||
_ferrumyx-runtime-core__help__tool_commands() {
    local commands; commands=(
'install:Install a WASM tool from source directory or .wasm file' \
'list:List installed tools' \
'remove:Remove an installed tool' \
'info:Show information about a tool' \
'auth:Configure authentication for a tool' \
'setup:Configure required secrets for a tool (from setup.required_secrets)' \
    )
    _describe -t commands 'ferrumyx-runtime-core help tool commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__tool__auth_commands] )) ||
_ferrumyx-runtime-core__help__tool__auth_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help tool auth commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__tool__info_commands] )) ||
_ferrumyx-runtime-core__help__tool__info_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help tool info commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__tool__install_commands] )) ||
_ferrumyx-runtime-core__help__tool__install_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help tool install commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__tool__list_commands] )) ||
_ferrumyx-runtime-core__help__tool__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help tool list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__tool__remove_commands] )) ||
_ferrumyx-runtime-core__help__tool__remove_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help tool remove commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__tool__setup_commands] )) ||
_ferrumyx-runtime-core__help__tool__setup_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help tool setup commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__help__worker_commands] )) ||
_ferrumyx-runtime-core__help__worker_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core help worker commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp_commands] )) ||
_ferrumyx-runtime-core__mcp_commands() {
    local commands; commands=(
'add:Add an MCP server' \
'remove:Remove an MCP server' \
'list:List configured MCP servers' \
'auth:Authenticate with an MCP server (OAuth flow)' \
'test:Test connection to an MCP server' \
'toggle:Enable or disable an MCP server' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core mcp commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp__add_commands] )) ||
_ferrumyx-runtime-core__mcp__add_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core mcp add commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp__auth_commands] )) ||
_ferrumyx-runtime-core__mcp__auth_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core mcp auth commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp__help_commands] )) ||
_ferrumyx-runtime-core__mcp__help_commands() {
    local commands; commands=(
'add:Add an MCP server' \
'remove:Remove an MCP server' \
'list:List configured MCP servers' \
'auth:Authenticate with an MCP server (OAuth flow)' \
'test:Test connection to an MCP server' \
'toggle:Enable or disable an MCP server' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core mcp help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp__help__add_commands] )) ||
_ferrumyx-runtime-core__mcp__help__add_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core mcp help add commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp__help__auth_commands] )) ||
_ferrumyx-runtime-core__mcp__help__auth_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core mcp help auth commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp__help__help_commands] )) ||
_ferrumyx-runtime-core__mcp__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core mcp help help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp__help__list_commands] )) ||
_ferrumyx-runtime-core__mcp__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core mcp help list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp__help__remove_commands] )) ||
_ferrumyx-runtime-core__mcp__help__remove_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core mcp help remove commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp__help__test_commands] )) ||
_ferrumyx-runtime-core__mcp__help__test_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core mcp help test commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp__help__toggle_commands] )) ||
_ferrumyx-runtime-core__mcp__help__toggle_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core mcp help toggle commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp__list_commands] )) ||
_ferrumyx-runtime-core__mcp__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core mcp list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp__remove_commands] )) ||
_ferrumyx-runtime-core__mcp__remove_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core mcp remove commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp__test_commands] )) ||
_ferrumyx-runtime-core__mcp__test_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core mcp test commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__mcp__toggle_commands] )) ||
_ferrumyx-runtime-core__mcp__toggle_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core mcp toggle commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__memory_commands] )) ||
_ferrumyx-runtime-core__memory_commands() {
    local commands; commands=(
'search:Search workspace memory (hybrid full-text + semantic)' \
'read:Read a file from the workspace' \
'write:Write content to a workspace file' \
'tree:Show workspace directory tree' \
'status:Show workspace status (document count, index health)' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core memory commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__memory__help_commands] )) ||
_ferrumyx-runtime-core__memory__help_commands() {
    local commands; commands=(
'search:Search workspace memory (hybrid full-text + semantic)' \
'read:Read a file from the workspace' \
'write:Write content to a workspace file' \
'tree:Show workspace directory tree' \
'status:Show workspace status (document count, index health)' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core memory help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__memory__help__help_commands] )) ||
_ferrumyx-runtime-core__memory__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core memory help help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__memory__help__read_commands] )) ||
_ferrumyx-runtime-core__memory__help__read_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core memory help read commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__memory__help__search_commands] )) ||
_ferrumyx-runtime-core__memory__help__search_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core memory help search commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__memory__help__status_commands] )) ||
_ferrumyx-runtime-core__memory__help__status_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core memory help status commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__memory__help__tree_commands] )) ||
_ferrumyx-runtime-core__memory__help__tree_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core memory help tree commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__memory__help__write_commands] )) ||
_ferrumyx-runtime-core__memory__help__write_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core memory help write commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__memory__read_commands] )) ||
_ferrumyx-runtime-core__memory__read_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core memory read commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__memory__search_commands] )) ||
_ferrumyx-runtime-core__memory__search_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core memory search commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__memory__status_commands] )) ||
_ferrumyx-runtime-core__memory__status_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core memory status commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__memory__tree_commands] )) ||
_ferrumyx-runtime-core__memory__tree_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core memory tree commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__memory__write_commands] )) ||
_ferrumyx-runtime-core__memory__write_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core memory write commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__onboard_commands] )) ||
_ferrumyx-runtime-core__onboard_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core onboard commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__pairing_commands] )) ||
_ferrumyx-runtime-core__pairing_commands() {
    local commands; commands=(
'list:List pending pairing requests' \
'approve:Approve a pairing request by code' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core pairing commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__pairing__approve_commands] )) ||
_ferrumyx-runtime-core__pairing__approve_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core pairing approve commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__pairing__help_commands] )) ||
_ferrumyx-runtime-core__pairing__help_commands() {
    local commands; commands=(
'list:List pending pairing requests' \
'approve:Approve a pairing request by code' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core pairing help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__pairing__help__approve_commands] )) ||
_ferrumyx-runtime-core__pairing__help__approve_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core pairing help approve commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__pairing__help__help_commands] )) ||
_ferrumyx-runtime-core__pairing__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core pairing help help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__pairing__help__list_commands] )) ||
_ferrumyx-runtime-core__pairing__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core pairing help list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__pairing__list_commands] )) ||
_ferrumyx-runtime-core__pairing__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core pairing list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__registry_commands] )) ||
_ferrumyx-runtime-core__registry_commands() {
    local commands; commands=(
'list:List available extensions in the registry' \
'info:Show detailed information about an extension or bundle' \
'install:Install an extension or bundle from the registry' \
'install-defaults:Install the default bundle of recommended extensions' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core registry commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__registry__help_commands] )) ||
_ferrumyx-runtime-core__registry__help_commands() {
    local commands; commands=(
'list:List available extensions in the registry' \
'info:Show detailed information about an extension or bundle' \
'install:Install an extension or bundle from the registry' \
'install-defaults:Install the default bundle of recommended extensions' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core registry help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__registry__help__help_commands] )) ||
_ferrumyx-runtime-core__registry__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core registry help help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__registry__help__info_commands] )) ||
_ferrumyx-runtime-core__registry__help__info_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core registry help info commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__registry__help__install_commands] )) ||
_ferrumyx-runtime-core__registry__help__install_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core registry help install commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__registry__help__install-defaults_commands] )) ||
_ferrumyx-runtime-core__registry__help__install-defaults_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core registry help install-defaults commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__registry__help__list_commands] )) ||
_ferrumyx-runtime-core__registry__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core registry help list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__registry__info_commands] )) ||
_ferrumyx-runtime-core__registry__info_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core registry info commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__registry__install_commands] )) ||
_ferrumyx-runtime-core__registry__install_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core registry install commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__registry__install-defaults_commands] )) ||
_ferrumyx-runtime-core__registry__install-defaults_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core registry install-defaults commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__registry__list_commands] )) ||
_ferrumyx-runtime-core__registry__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core registry list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__run_commands] )) ||
_ferrumyx-runtime-core__run_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core run commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__service_commands] )) ||
_ferrumyx-runtime-core__service_commands() {
    local commands; commands=(
'install:Install the OS service (launchd on macOS, systemd on Linux)' \
'start:Start the installed service' \
'stop:Stop the running service' \
'status:Show service status' \
'uninstall:Uninstall the OS service and remove the unit file' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core service commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__service__help_commands] )) ||
_ferrumyx-runtime-core__service__help_commands() {
    local commands; commands=(
'install:Install the OS service (launchd on macOS, systemd on Linux)' \
'start:Start the installed service' \
'stop:Stop the running service' \
'status:Show service status' \
'uninstall:Uninstall the OS service and remove the unit file' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core service help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__service__help__help_commands] )) ||
_ferrumyx-runtime-core__service__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core service help help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__service__help__install_commands] )) ||
_ferrumyx-runtime-core__service__help__install_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core service help install commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__service__help__start_commands] )) ||
_ferrumyx-runtime-core__service__help__start_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core service help start commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__service__help__status_commands] )) ||
_ferrumyx-runtime-core__service__help__status_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core service help status commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__service__help__stop_commands] )) ||
_ferrumyx-runtime-core__service__help__stop_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core service help stop commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__service__help__uninstall_commands] )) ||
_ferrumyx-runtime-core__service__help__uninstall_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core service help uninstall commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__service__install_commands] )) ||
_ferrumyx-runtime-core__service__install_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core service install commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__service__start_commands] )) ||
_ferrumyx-runtime-core__service__start_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core service start commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__service__status_commands] )) ||
_ferrumyx-runtime-core__service__status_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core service status commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__service__stop_commands] )) ||
_ferrumyx-runtime-core__service__stop_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core service stop commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__service__uninstall_commands] )) ||
_ferrumyx-runtime-core__service__uninstall_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core service uninstall commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__status_commands] )) ||
_ferrumyx-runtime-core__status_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core status commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool_commands] )) ||
_ferrumyx-runtime-core__tool_commands() {
    local commands; commands=(
'install:Install a WASM tool from source directory or .wasm file' \
'list:List installed tools' \
'remove:Remove an installed tool' \
'info:Show information about a tool' \
'auth:Configure authentication for a tool' \
'setup:Configure required secrets for a tool (from setup.required_secrets)' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core tool commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool__auth_commands] )) ||
_ferrumyx-runtime-core__tool__auth_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core tool auth commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool__help_commands] )) ||
_ferrumyx-runtime-core__tool__help_commands() {
    local commands; commands=(
'install:Install a WASM tool from source directory or .wasm file' \
'list:List installed tools' \
'remove:Remove an installed tool' \
'info:Show information about a tool' \
'auth:Configure authentication for a tool' \
'setup:Configure required secrets for a tool (from setup.required_secrets)' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ferrumyx-runtime-core tool help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool__help__auth_commands] )) ||
_ferrumyx-runtime-core__tool__help__auth_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core tool help auth commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool__help__help_commands] )) ||
_ferrumyx-runtime-core__tool__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core tool help help commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool__help__info_commands] )) ||
_ferrumyx-runtime-core__tool__help__info_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core tool help info commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool__help__install_commands] )) ||
_ferrumyx-runtime-core__tool__help__install_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core tool help install commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool__help__list_commands] )) ||
_ferrumyx-runtime-core__tool__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core tool help list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool__help__remove_commands] )) ||
_ferrumyx-runtime-core__tool__help__remove_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core tool help remove commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool__help__setup_commands] )) ||
_ferrumyx-runtime-core__tool__help__setup_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core tool help setup commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool__info_commands] )) ||
_ferrumyx-runtime-core__tool__info_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core tool info commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool__install_commands] )) ||
_ferrumyx-runtime-core__tool__install_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core tool install commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool__list_commands] )) ||
_ferrumyx-runtime-core__tool__list_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core tool list commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool__remove_commands] )) ||
_ferrumyx-runtime-core__tool__remove_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core tool remove commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__tool__setup_commands] )) ||
_ferrumyx-runtime-core__tool__setup_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core tool setup commands' commands "$@"
}
(( $+functions[_ferrumyx-runtime-core__worker_commands] )) ||
_ferrumyx-runtime-core__worker_commands() {
    local commands; commands=()
    _describe -t commands 'ferrumyx-runtime-core worker commands' commands "$@"
}

if [ "$funcstack[1]" = "_ferrumyx-runtime-core" ]; then
    _ferrumyx-runtime-core "$@"
else
    (( $+functions[compdef] )) && compdef _ferrumyx-runtime-core ferrumyx-runtime-core
fi

