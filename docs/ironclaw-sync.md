# Ironclaw Upstream Sync Workflow

This repository vendors Ironclaw at `crates/ironclaw` using a git subtree.  
Use this workflow to keep Ferrumyx current without breaking ongoing feature work.

## One-time setup

1. Ensure you are in repo root: `D:\AI\Ferrumyx`
2. Verify no uncommitted changes (recommended): `git status --short`
3. Run the sync script:

```powershell
.\scripts\ironclaw-sync.ps1
```

The script will:
- Add `ironclaw-upstream` remote if missing.
- Fetch the upstream branch (`main` by default).
- Create `integration/ironclaw-sync-YYYYMMDD` branch.
- Pull upstream into `crates/ironclaw` using `git subtree pull --squash`.
- Run compile checks for key Ferrumyx crates.

## Common options

```powershell
# Allow syncing with a dirty local working tree
.\scripts\ironclaw-sync.ps1 -AllowDirty

# Keep full Ironclaw history (no squash)
.\scripts\ironclaw-sync.ps1 -NoSquash

# Reuse current branch
.\scripts\ironclaw-sync.ps1 -NoBranch

# Skip cargo checks (use only if CI will validate immediately)
.\scripts\ironclaw-sync.ps1 -SkipChecks
```

## After sync

1. Inspect changes under `crates/ironclaw`.
2. Update `docs/ironclaw-version.md`.
3. Commit with a message like:
   - `chore(sync): update ironclaw subtree to <sha>`
4. Open PR into your main development branch.
5. Wait for `Ironclaw Sync Gate` CI checks to pass before merge.

## Recommended operating model

- Keep Ferrumyx-specific behavior in Ferrumyx crates when possible.
- Avoid editing `crates/ironclaw` directly unless patching a bug you intend to upstream.
- Run sync on a cadence (for example weekly) or when a needed upstream fix lands.
- If a sync fails, reduce scope by temporarily pinning to an earlier upstream commit and retry.

