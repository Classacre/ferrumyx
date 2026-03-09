param(
    [string]$UpstreamRemote = "ironclaw-upstream",
    [string]$UpstreamUrl = "https://github.com/nearai/ironclaw.git",
    [string]$UpstreamBranch = "main",
    [string]$Prefix = "crates/ironclaw",
    [string]$IntegrationBranchPrefix = "integration/ironclaw-sync",
    [switch]$NoSquash,
    [switch]$NoBranch,
    [switch]$SkipChecks,
    [switch]$AllowDirty
)

$ErrorActionPreference = "Stop"

function Invoke-Git {
    param(
        [Parameter(Mandatory = $true)][string[]]$Args
    )
    & git @Args
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Args -join ' ') failed with exit code $LASTEXITCODE."
    }
}

function Invoke-CommandOrThrow {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter(Mandatory = $true)][string]$Description
    )
    Write-Host ">> $Description"
    Invoke-Expression $Command
    if ($LASTEXITCODE -ne 0) {
        throw "'$Command' failed with exit code $LASTEXITCODE."
    }
}

Write-Host "Ferrumyx Ironclaw sync started."

if (-not (Test-Path "Cargo.toml")) {
    throw "Run this script from the Ferrumyx repository root."
}

if (-not $AllowDirty) {
    $status = git status --porcelain
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to read git status."
    }
    if ($status) {
        throw "Working tree is dirty. Commit/stash changes first, or rerun with -AllowDirty."
    }
}

$remoteExists = git remote | Where-Object { $_ -eq $UpstreamRemote }
if (-not $remoteExists) {
    Write-Host "Adding remote '$UpstreamRemote' -> $UpstreamUrl"
    Invoke-Git -Args @("remote", "add", $UpstreamRemote, $UpstreamUrl)
}
else {
    Write-Host "Remote '$UpstreamRemote' already exists."
}

Write-Host "Fetching $UpstreamRemote/$UpstreamBranch..."
Invoke-Git -Args @("fetch", $UpstreamRemote, $UpstreamBranch, "--tags")

if (-not $NoBranch) {
    $date = Get-Date -Format "yyyyMMdd"
    $branchName = "$IntegrationBranchPrefix-$date"
    $exists = git branch --list $branchName
    if ($exists) {
        $branchName = "$branchName-$(Get-Date -Format 'HHmmss')"
    }
    Write-Host "Creating integration branch: $branchName"
    Invoke-Git -Args @("checkout", "-b", $branchName)
}

$subtreeArgs = @("subtree", "pull", "--prefix=$Prefix", $UpstreamRemote, $UpstreamBranch)
if (-not $NoSquash) {
    $subtreeArgs += "--squash"
}

Write-Host "Pulling upstream Ironclaw subtree into $Prefix..."
Invoke-Git -Args $subtreeArgs

if (-not $SkipChecks) {
    Invoke-CommandOrThrow -Command "cargo check -p ferrumyx-agent" -Description "cargo check ferrumyx-agent"
    Invoke-CommandOrThrow -Command "cargo check -p ferrumyx-web" -Description "cargo check ferrumyx-web"
    Invoke-CommandOrThrow -Command "cargo check -p ferrumyx-ingestion" -Description "cargo check ferrumyx-ingestion"
}

$upstreamSha = git rev-parse "$UpstreamRemote/$UpstreamBranch"
if ($LASTEXITCODE -ne 0) {
    throw "Unable to resolve upstream commit SHA."
}

Write-Host ""
Write-Host "Sync complete."
Write-Host "Upstream commit: $upstreamSha"
Write-Host "Next steps:"
Write-Host "  1) Review changes under $Prefix"
Write-Host "  2) Update docs/ironclaw-version.md"
Write-Host "  3) Commit and open a PR"

