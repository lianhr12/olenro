# Git Asset Sync Guide

## Overview

Git sync commits your **AI agent assets** to a **private Git repository** as a **plaintext directory**, so you can "configure once, restore seamlessly on a new machine." It replaces the legacy WebDAV sync.

What gets synced:

| Asset | Form | Notes |
| --- | --- | --- |
| Providers (Claude / Codex / Gemini / OpenCode / OpenClaw / Hermes) | `db.sql` + `providers/<app>.json` | `db.sql` is the lossless authoritative backup; `providers/*.json` is a readable mirror |
| MCP servers | `db.sql` + `mcp/servers.json` | same as above |
| Skills | `skills/<name>/...` | plaintext copy of the SSOT skills |
| Claude `settings.json` (incl. hooks), `agents/` (subagents), `commands/` (slash commands), `CLAUDE.md`, `output-styles/` | `agents/claude/...` | cross-machine mirror only, no cross-tool conversion |
| Codex `AGENTS.md`, `prompts/` | `agents/codex/...` | same as above |
| Gemini `GEMINI.md`, `commands/` | `agents/gemini/...` | same as above |

> **Design principle**: Providers / MCP / Skills are always synced; the other "agent assets" are mirrored per tool as-is (toggleable in settings), with **no cross-tool format conversion**.

## Requirements

- **git** installed and on PATH.
- A **private** Git repo (GitHub / GitLab / self-hosted) and a **Personal Access Token (PAT)** with `repo` (read/write) scope.

> ⚠️ **Use a private repo**: the PAT stays local, but provider **API keys are committed in plaintext**.

## How to get a PAT

A PAT (Personal Access Token) authorizes the app to read/write your sync repo. Pick the steps for your host; at minimum you only need **repository contents read/write**.

> 💡 The token is **shown only once** at creation — copy it immediately. It is stored only on your machine and never committed to the sync repo.

### GitHub

**Option A: Fine-grained token (recommended)**

1. Top-right avatar → **Settings**
2. Bottom of the left sidebar → **Developer settings**
3. **Personal access tokens → Fine-grained tokens → Generate new token**
4. Configure:
   - **Token name**: e.g. `olenro-sync`
   - **Expiration**: as you prefer
   - **Repository access** → **Only select repositories** → choose your **private sync repo**
   - **Permissions → Repository permissions → Contents** → **Read and write** (required for git push/pull; leave the rest at No access)
5. **Generate token** and copy `github_pat_...`

**Option B: Classic token (simpler)**

1. **Settings → Developer settings → Personal access tokens → Tokens (classic) → Generate new token (classic)**
2. Check the **`repo`** scope (covers private repo read/write)
3. Generate and copy `ghp_...`

Direct link: `https://github.com/settings/tokens`

### GitLab

1. Avatar → **Edit profile → Access Tokens** (or `https://gitlab.com/-/user_settings/personal_access_tokens`)
2. Select scopes: **`read_repository`** + **`write_repository`**
3. Set expiration → **Create** → copy `glpat-...`

### Gitee

1. Avatar → **Settings → Private Tokens → Generate new token**
2. Select **`projects`** (repository read/write)
3. Copy the token

> Leave the **Username** field empty in most cases (defaults to `x-access-token`, which works for GitHub); use the HTTPS form for the repo URL, e.g. `https://github.com/yourname/your-private-repo.git`.

## Setup

1. Open **Settings → Cloud Sync**.
2. Fill in:
   - **Repository URL** (HTTPS), e.g. `https://github.com/yourname/ai-agent-config.git`
   - **Branch** (default `main`)
   - **Access Token (PAT)**
   - **Username** (optional, defaults to `x-access-token`), **Device Name** (optional, shown in commit messages)
3. Click **Test Connection**, then **Save**.
4. In **Assets to sync**, toggle the per-tool agent assets you want.

## Daily Use

- **Push (↑)**: commit and push current local assets to the remote.
  - If the remote has commits you don't (**diverged**), you'll be prompted to **Pull** (take remote, overwrites local) or **Force push** (overwrite remote with local).
- **Pull (↓)**: fetch the remote and restore locally (remote-authoritative).
  - Providers / MCP / Skills are restored wholesale; agent assets are restored **additively** (local files missing from the remote are kept).

> Sync is **manual** — nothing is pushed automatically in the background.

## New machine

1. Install the app and git, open **Settings → Cloud Sync**, enter the same repo URL and PAT, and save.
2. Click **Pull** to restore providers, MCP, Skills, and per-tool agent configs.

## Migrating from WebDAV

WebDAV sync has been **removed** in favor of Git. To migrate:

1. Before upgrading (old version): run a WebDAV sync once, or use **Settings → Data → Export Config** to keep a `.sql` backup.
2. After upgrading: old WebDAV settings are ignored; your local data (providers / MCP / Skills) is unchanged.
3. Configure the Git repo as above and click **Push** to commit your local assets as the new baseline.
4. On other machines, upgrade, configure the same repo, and **Pull**.

> If local data looks wrong after upgrading, use **Settings → Data → Import Config** to restore the `.sql` backup from step 1.

## Notes & Limitations

- **Plaintext secrets**: the repo must be private; enable access control/encryption on the host side for stronger protection.
- **Requires git**: the transport shells out to system `git`; the PAT is injected via `GIT_ASKPASS` and never appears in argv or the reflog.
- **Cross-OS absolute paths**: absolute paths or shell commands inside `settings.json` / hooks are restored verbatim and may need manual adjustment between Windows and macOS.
- **Excluded by default**: conversation history (`~/.claude/projects/`), logs, caches, `.git`, `.DS_Store`, `node_modules` are not committed.
- **Deletions don't propagate** (agent assets): on pull, assets missing from the remote are kept locally rather than deleted.
