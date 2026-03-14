# External Integrations

**Analysis Date:** 2026-03-14

## APIs & External Services

**Version Check & Telemetry:**
- `api.sentrux.dev/version` — Cloudflare Worker endpoint
  - Transport: HTTP GET via `curl` subprocess (not a Rust HTTP client)
  - Frequency: Once per day, cached in `~/.sentrux/last_update_check`
  - Payload: version, platform, mode (`gui`/`mcp`/`cli`/`plugin`), plugin count, license tier, scan count, MCP call count, gate run count
  - Disable: `SENTRUX_NO_UPDATE_CHECK=1`
  - Implementation: `sentrux-core/src/app/update_check.rs`

**Plugin Registry:**
- `https://github.com/sentrux/plugins/releases/download/{name}-v0.1.0/{name}-{platform}.tar.gz`
  - Transport: `curl` subprocess + `tar` subprocess
  - Purpose: Download pre-compiled tree-sitter grammar `.so`/`.dylib` files for language plugins
  - Triggered: First run auto-install, `sentrux plugin add <name>`, `sentrux plugin add-standard`
  - Implementation: `sentrux-bin/src/main.rs` (`auto_install_plugins_if_needed`, `run_plugin`)

## Data Storage

**Databases:**
- None. All data is in-memory or file-based.

**File Storage:**
- Local filesystem only
  - `~/.sentrux/plugins/` — installed language plugin directories (grammars + queries)
  - `~/.sentrux/last_update_check` — plaintext Unix timestamp
  - `.sentrux/rules.toml` — per-project architectural rule configuration
  - `.sentrux/baseline.json` — per-project structural metrics baseline for `sentrux gate`
  - `eframe` persistence stores GUI state (window size, preferences) via the `persistence` feature

**Caching:**
- In-memory only
  - Git status cache: `DashMap` with 2-second TTL (`sentrux-core/src/analysis/git.rs`)
  - MCP scan cache: `McpState` struct holds `cached_snapshot`, `cached_health`, `cached_arch`, `cached_evolution` per session (`sentrux-core/src/app/mcp_server/mod.rs`)

## Authentication & Identity

**Auth Provider:**
- None for free tier
- Pro/Team tier: Ed25519 license key validation, implemented in the private `sentrux-pro` crate, activated via the `pro` Cargo feature
- License tier stored in a `OnceLock<Tier>` global (`sentrux-core/src/license.rs`)

## MCP (Model Context Protocol) Integration

**Protocol:**
- JSON-RPC 2.0 over stdio (`sentrux --mcp` or `sentrux mcp`)
- Protocol version: `2024-11-05`
- Transport: stdin/stdout only — zero network calls during MCP operation
- Implementation: `sentrux-core/src/app/mcp_server/`

**MCP Tools exposed (free tier):**
- `scan` — scan a directory and compute all metrics
- `rescan` — re-scan the previously scanned directory
- `session_start` / `session_end` — session lifecycle management
- `health` — structural health report (grades A-F across 14 dimensions)
- `coupling` — file coupling analysis
- `cycles` — circular dependency detection
- `architecture` — architecture layer analysis
- `blast_radius` — impact analysis for a file change
- `hottest` — hottest files by churn/coupling
- `level` — abstraction level analysis
- `check_rules` — enforce `.sentrux/rules.toml` constraints
- `evolution` — git history-based evolution analysis
- `dsm` — dependency structure matrix
- `test_gaps` — test coverage gap analysis

**Pro MCP tools** (registered by `sentrux-pro` private crate): `gate`, `churn`, `coupling_history`, `bus_factor`, `whatif`

**MCP client configuration** (`claude-plugin/.mcp.json`):
```json
{
  "mcpServers": {
    "sentrux": {
      "command": "sentrux",
      "args": ["--mcp"]
    }
  }
}
```

## Git Integration

**Provider:** Local git repositories only — no remote API calls
- Client: `git2 0.20` (libgit2 Rust bindings)
- Purpose: Per-file git status display in GUI, evolution/churn metrics from commit history
- Implementation: `sentrux-core/src/analysis/git.rs`, `sentrux-core/src/metrics/evo/`

## Filesystem Watching

**Provider:** `notify 7` + `notify-debouncer-mini 0.5`
- Purpose: Live reload when source files change during GUI mode
- Implementation: `sentrux-core/src/app/watcher.rs`

## Monitoring & Observability

**Error Tracking:**
- None — errors surfaced as stderr output only

**Logs:**
- `eprintln!` to stderr only (no structured logging framework)

## CI/CD & Deployment

**Hosting:**
- GitHub Releases — binary artifacts for each platform
  - `sentrux-darwin-arm64`
  - `sentrux-linux-x86_64`
- Homebrew — `brew upgrade sentrux` mentioned in update notifications

**Install script:**
- `install.sh` — downloads binary from GitHub Releases for the detected OS/arch

**CI Pipeline:**
- Not detected in repository (no `.github/workflows/` directory present)

## Webhooks & Callbacks

**Incoming:** None

**Outgoing:** None (the telemetry ping is a one-way GET with no webhook semantics)

## Environment Configuration

**Required env vars:** None (all optional)

**Optional env vars:**
- `SENTRUX_NO_UPDATE_CHECK` — set to any value to disable telemetry/update pings
- `WGPU_BACKEND` — override GPU rendering backend (`vulkan`, `gl`, `metal`, `dx12`)

**Secrets location:**
- Pro license key management is handled by the private `sentrux-pro` crate; no secrets in this repository

---

*Integration audit: 2026-03-14*
