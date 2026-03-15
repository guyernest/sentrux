# Technology Stack

**Project:** Sentrux — PMAT Integration + Git/GSD Overlays
**Researched:** 2026-03-14
**Scope:** Subsequent milestone additions to an existing eframe/egui Rust desktop app

---

## Critical Finding: PMAT Is Not a Library Crate

**Confidence: HIGH** (verified via GitHub search + README analysis)

The `pmat` crate (v3.7.0, published to crates.io) has **no `lib.rs`**. GitHub search returns 0 results for `filename:lib.rs` in `paiml/paiml-mcp-agent-toolkit`. The README documents only CLI usage (`pmat analyze tdg`, `pmat context`, etc.) with no Rust API examples, no Cargo dependency examples, and no docs.rs library documentation.

**This directly blocks the stated constraint:** "PMAT must be usable as a Cargo library dependency (not CLI or MCP client)."

The viable integration paths are documented in the Integration Approaches section below. The roadmap must address this before attempting PMAT integration.

---

## Recommended Stack Additions

These are additions to the existing stack. The existing stack (eframe 0.31, egui 0.31, git2 0.20, rayon, crossbeam-channel, serde, serde_json, dashmap, toml) remains unchanged.

### PMAT Integration

| Approach | Mechanism | Cargo Change | Confidence |
|----------|-----------|--------------|------------|
| **Git path dependency** | `pmat = { git = "https://github.com/paiml/paiml-mcp-agent-toolkit" }` | Yes, if `lib.rs` can be conditionally compiled or exposed | LOW — requires verifying build succeeds and that desired modules are pub |
| **Subprocess (spawn)** | `std::process::Command::new("pmat")` + JSON stdout parsing | None (pmat must be installed separately) | MEDIUM — works today, adds runtime dependency |
| **Embed via build.rs** | Build script runs `pmat` against project, embeds JSON output as compile-time data | None | LOW — not viable for live GUI updates |

**Recommended approach for milestone: subprocess with serde_json deserialization.**

Rationale: PMAT is already installed as a CLI tool. Sentrux already has `serde_json` and `serde` in-tree. The subprocess approach delivers PMAT's TDG grades and health scores reliably today without waiting for an upstream library API. The integration is a background worker thread (fits the existing `scan_threads.rs` pattern) that spawns `pmat analyze tdg --output json` for a given path and deserializes the result into a `PmatReport` struct. The MCP transport (`pmat mcp` over stdio) is also viable if the JSON-RPC overhead is acceptable.

**If git path dependency works:** Add to `sentrux-core/Cargo.toml`:
```toml
pmat = { git = "https://github.com/paiml/paiml-mcp-agent-toolkit", default-features = false }
```
This requires auditing PMAT's transitive dependencies for conflicts with sentrux's existing deps (especially `tree-sitter 0.25` and `tokei 12`, which PMAT likely also depends on at potentially different versions).

### Git History Overlay

No new dependencies needed. **Use existing `git2 0.20`.**

The existing `sentrux-core/src/metrics/evo/git_walker.rs` already implements `walk_git_log(root, lookback_days)` returning `Vec<CommitRecord>` with per-file `added`/`removed` line counts and commit timestamps. The git diff overlay needs:

1. A configurable time window (15min, 1h, 1d, 7d, 30d, etc.)
2. Per-file churn score computed from `CommitRecord.files` filtered to the window
3. A new `ColorMode::GitChurn` variant in `layout/types.rs` using the churn score

The `HeatTracker` + `heat_color()` pattern in `core/heat.rs` is the template: churn data feeds a color ramp the same way heat does. No new crate needed.

| Aspect | Approach | Rationale |
|--------|----------|-----------|
| Git walking | Existing `git2 0.20` + `walk_git_log` | Already implemented, tested, skip megacommits |
| Time windows | `epoch_now() - window_secs` cutoff | Matches existing lookback_days pattern |
| Color mapping | Extend `ColorMode` enum + reuse `heat_color()` | Same rendering path, no new overlay infrastructure |
| Caching | Extend `DashMap` in `analysis/git.rs` | TTL cache already proven for status; same pattern for churn |

### GSD Phase Overlay

No new dependencies needed. **Use existing `serde_json` + `serde` + `std::fs`.**

GSD planning files live at `.planning/` within a project directory. The relevant data for the overlay is in milestone/phase files that list which files each phase will touch. The parsing approach:

1. Scan `.planning/` at the project root for `*.md` phase files
2. Parse markdown for file path mentions (regex or simple line scanning)
3. Build a `HashMap<String, GsdPhaseInfo>` mapping file path → phase metadata
4. Add `ColorMode::GsdPhase` variant, color by phase number or status (done/active/future)

The existing `regex 1` crate is already in-tree. No new parser needed.

| Aspect | Approach | Rationale |
|--------|----------|-----------|
| File format | Markdown plain text | GSD produces `.md` files; no dedicated parser needed |
| Parsing | `regex` pattern matching or line scan for `.rs`, `.ts`, `.js` paths | `regex 1` already in `Cargo.toml` |
| Deserialization | None required (markdown, not JSON/TOML) | Simpler than adding a markdown parser |
| Color assignment | Phase number → `egui::Color32` palette (fixed 8-color cycle) | Matches existing badge color approach |

### No New egui Extensions Needed

The existing egui 0.31 `Painter` API has everything required for overlays:
- `Painter::rect_filled()` — overlay color on treemap blocks
- `Painter::text()` — grade badges (TDG A+/F labels)
- Alpha blending via `Color32::from_rgba_unmultiplied()` — semi-transparent overlays over existing blocks

The existing `renderer/badges.rs` already renders per-file health indicators. TDG grade badges follow the same pattern. No `egui_extras`, `egui_plot`, or other extension crates are needed.

---

## Dependency Version Table

All versions below are the existing versions in `sentrux-core/Cargo.toml`. No version bumps recommended until PMAT integration is assessed.

| Crate | Current Version | Role in New Work | Action |
|-------|-----------------|-----------------|--------|
| `git2` | 0.20 | Git churn overlay, window-based commit walking | No change — already sufficient |
| `serde` | 1 | PMAT JSON deserialization, GSD struct models | No change |
| `serde_json` | 1 | Parse `pmat` subprocess JSON output | No change |
| `dashmap` | 6 | Churn cache (path → score) | No change |
| `crossbeam-channel` | 0.5 | New `PmatMsg` channel on scanner thread | No change |
| `rayon` | 1 | Not needed for new work (git walk is sequential) | No change |
| `regex` | 1 | GSD markdown file-path extraction | No change |
| `toml` | 0.8 | Not needed for GSD (markdown, not TOML) | No change |
| `pmat` | 3.7.0 | PMAT analysis — integration approach TBD | ADD (path TBD) |

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| PMAT integration | Subprocess + JSON | Direct library dep | No lib.rs — library API does not exist |
| PMAT integration | Subprocess + JSON | MCP stdio client | More IPC complexity; JSON-RPC overhead; MCP removal is an existing project goal |
| Git churn | Existing `git2` walker | New `gix` crate | `gix` is a newer pure-Rust alternative but adding a second git library alongside `git2` creates dependency conflict risk; existing code is working |
| GSD parsing | Regex line scan | `pulldown-cmark` markdown parser | Markdown parser adds a dependency for a simple file-path extraction task; regex already in-tree |
| Overlay rendering | Existing egui Painter | `egui_extras` tables / plots | Not needed — overlays are color fills on rectangles, not charts |

---

## Integration Warning: PMAT Transitive Dependencies

**Confidence: MEDIUM** (inferred from PMAT's documented capabilities; Cargo.toml not directly readable due to rate limits)

PMAT almost certainly depends on `tree-sitter` and `tokei`, which sentrux also depends on. If PMAT pins different versions (e.g., `tree-sitter 0.24` vs sentrux's `0.25`), Cargo will either unify to one version (if semver-compatible) or fail to resolve. Before adding `pmat` to `Cargo.toml`, run:

```bash
cargo add pmat --dry-run
cargo tree --duplicates
```

to detect version conflicts before they break the build. This is the first task in the PMAT integration milestone phase.

---

## Subprocess Integration Pattern

When using subprocess as the PMAT integration strategy, the recommended pattern follows the existing `scan_threads.rs` channel model:

1. Add `PmatCmd` and `PmatMsg` to `app/channels.rs`
2. Spawn a `pmat_thread` in `scan_threads.rs` analogous to the scanner thread
3. Thread spawns `pmat analyze tdg --output json <path>`, reads stdout, sends `PmatMsg::Ready(PmatReport)` to main thread
4. `update_loop.rs` receives `PmatMsg::Ready` and stores `PmatReport` on `AppState`
5. Renderer reads TDG grades from `AppState.pmat_report` when `ColorMode::TdgGrade` is active

This is the same immutable-report-via-channel pattern already used for `HealthReport`, `ArchReport`, and `EvolutionReport`. No shared mutable state, no locking.

---

## Sources

- PMAT repository (paiml/paiml-mcp-agent-toolkit): GitHub search confirmed no `lib.rs` — binary-only crate
- PMAT README: CLI-only usage documented, no Rust library API surface
- PMAT release v3.7.0: March 9, 2026 — latest confirmed version
- `sentrux-core/Cargo.toml`: Existing dependency versions (authoritative, local)
- `sentrux-core/src/metrics/evo/git_walker.rs`: Existing git walk implementation (authoritative, local)
- `sentrux-core/src/core/heat.rs`: Existing overlay color pattern (authoritative, local)
- `sentrux-core/src/app/state.rs`: AppState + channel architecture (authoritative, local)
- `.planning/codebase/ARCHITECTURE.md`: Sentrux layer architecture (authoritative, local)
