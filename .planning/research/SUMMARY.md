# Project Research Summary

**Project:** Sentrux — PMAT Integration + Git/GSD Overlays
**Domain:** Native Rust desktop code visualization — treemap + static analysis + planning overlays
**Researched:** 2026-03-14
**Confidence:** MEDIUM (HIGH on codebase, LOW on PMAT library API surface)

## Executive Summary

Sentrux is a native eframe/egui desktop tool that visualizes codebases as interactive treemaps with dependency edges, health grades, and file-level overlays. The current milestone adds three major capabilities: replacing sentrux's own analysis engine with PMAT's TDG grades and health scoring, adding a git diff overlay that colors files by change recency within a selectable time window, and adding a GSD phase overlay that colors files by which planning phase will touch them. The combined result — past (git), present (TDG), and future (GSD) visible on a single treemap — is the unique thesis of the product and has no direct competitor.

The recommended technical approach uses no new Cargo dependencies. PMAT is integrated via subprocess spawn (the library API is unconfirmed), git history uses the existing `git2` walker, and GSD phase data is parsed from `.planning/` markdown using the existing `regex` crate. The architecture extends existing patterns without restructuring: new `ColorMode` variants, new fields on `ScanReports` and `AppState`, and a new adapter module wrapping the external PMAT call. The existing thread model, `Snapshot`, layout engine, and renderer remain structurally unchanged.

The dominant risk is that PMAT exposes no stable library API — it is published as a binary crate with no confirmed `lib.rs`. This is a confirmed blocker: integration cannot proceed without first verifying PMAT's public module surface. The secondary risk is dependency version conflicts (especially `tree-sitter`) when adding PMAT to the workspace. Both must be resolved before any integration code is written. All other risks are manageable with the mitigation patterns documented in this research.

---

## Key Findings

### Recommended Stack

The existing stack (eframe 0.31, egui 0.31, git2 0.20, rayon, crossbeam-channel, serde, serde_json, dashmap, toml, regex) is fully sufficient. No new crates are needed for the git or GSD overlay milestones.

PMAT integration is the only dependency question. The PMAT crate (v3.7.0) has no confirmed `lib.rs` — GitHub search returns zero results for `filename:lib.rs` in the PMAT repository. The recommended integration path is subprocess: spawn `pmat analyze tdg --output json`, parse stdout via `serde_json`. This matches the existing `scan_threads.rs` channel pattern and adds no Cargo dependency. If a library path is confirmed viable, `pmat = { git = "..." }` is viable but requires a `cargo tree -d` conflict audit before any code changes.

**Core technologies:**
- `git2 0.20` — git history walk for churn overlay — already implemented, no change needed
- `serde_json` — PMAT subprocess JSON deserialization — already in tree
- `regex 1` — GSD markdown file-path extraction — already in tree
- `crossbeam-channel 0.5` — new `DiffOverlay` command + `DiffReady` message — existing infrastructure
- `dashmap 6` — churn cache and overlay color map caching — existing infrastructure

### Expected Features

**Must have (table stakes — must not regress):**
- Treemap renders on open with no config — already exists, must survive PMAT integration
- File nodes sized by LOC — already exists via tokei; LOC source must be consolidated post-PMAT
- Color encodes one quality signal by default — currently health grade; must switch to TDG grade
- Grade label readable on node (A+ through F) — new, depends on PMAT integration
- Pan, zoom, click-for-details, live reload — already exist; must remain intact
- Clear indication of worst files — implied by TDG coloring + top-N panel

**Should have (differentiators):**
- TDG grade on every treemap node — core thesis, depends on PMAT
- Git diff overlay with 15min/1h/1d/1w/30d time windows — no other local tool does this on a treemap canvas
- GSD phase overlay — no other tool maps planning phases to file locations
- Overlay mode switcher (single toggle: TDG / git-diff / GSD) — low effort, high impact
- PMAT health score + mutation testing results in file detail panel

**Defer indefinitely:**
- PR/branch comparison view — git diff overlay with wide window approximates this
- Inline code viewer — OS default editor on double-click is sufficient
- Historical timeline playback — out of scope
- Remote/team scan sharing, CI badge integration, web deployment

### Architecture Approach

The architecture extends the existing component structure without restructuring. PMAT analysis runs as an additional step on the scanner thread after the filesystem scan, delivering a `PmatReport` through `ScanReports` → `AppState` via the existing immutable-report-via-channel pattern. The git diff overlay operates on an independent channel/command pair (`ScanCommand::DiffOverlay` / `ScanMsg::DiffReady`) so the time window can be changed without triggering a full rescan or layout recalculation. The GSD phase overlay is computed during the main scan as a best-effort read of `.planning/` files. All three overlays enter the renderer via new `ColorMode` variants dispatched in the existing `file_color` function — zero changes to the layout thread or `Snapshot` structure.

**Major components:**
1. `analysis/pmat_adapter.rs` (new) — isolates PMAT API surface; subprocess or library call; maps to `PmatReport`
2. `analysis/git_diff_window.rs` (new) — time-windowed git log walk; extends existing `evo/git_walker.rs` patterns
3. `analysis/gsd_phase_reader.rs` (new) — reads `.planning/` markdown; builds `GsdPhaseMap` (best-effort, never blocks)
4. `core/pmat_types.rs` + `core/overlay_types.rs` (new) — data types for the three overlays, separate from `Snapshot`
5. `renderer/rects.rs` (extend) — three new `file_color` branches: `TdgGrade`, `GitDiff`, `GsdPhase`
6. `app/toolbar.rs` (extend) — overlay mode switcher + time window selector
7. `metrics/` subtrees (delete) — `grading`, `stability`, `arch`, `dsm`, `testgap`, `whatif` deleted post-PMAT
8. `app/mcp_server/` + `analysis/plugin/` (delete) — removed in Phase 1

### Critical Pitfalls

1. **PMAT has no confirmed library API** — Verify PMAT's `Cargo.toml` for a `[lib]` section and public types before writing any integration code. If absent, the integration design defaults to subprocess. This is the Phase 1 Task 1 blocker — nothing else in Phase 1 can proceed until confirmed.

2. **Dependency version conflicts on `tree-sitter` and `tokei`** — Run `cargo tree -d` immediately after adding PMAT to `Cargo.toml`. Resolve every duplicate before writing integration code. `tree-sitter` ABI mismatches produce runtime panics, not build errors.

3. **`ColorMode` enum is serialized to disk** — Adding new variants (`GitDiff`, `GsdPhase`) breaks deserialization of saved user preferences. Add `#[serde(other)]` fallback to an existing variant before adding the first new `ColorMode` variant. Write a test that deserializes an old prefs blob.

4. **Git history walk blocks the scanner thread on large repos** — The existing `walk_git_log` is synchronous. The git diff overlay must run on a separate channel/thread delivering `DiffReady`, not blocking `ScanMsg::Complete`. Apply a hard commit-count cap and surface it in the status bar.

5. **GSD path matching produces zero matches** — Snapshot paths are scan-root-relative; GSD plan files may reference absolute paths, `./`-prefixed paths, or wrong-case paths on macOS. Implement a path normalization function and test it with at least four path format cases before shipping the GSD overlay.

---

## Implications for Roadmap

Based on research, the feature dependencies are clear and the phase order is constrained. GSD and git overlays require a stable PMAT baseline first. Git and GSD phases are then independent of each other.

### Phase 1: PMAT Foundation

**Rationale:** All subsequent work depends on PMAT being integrated and the old analysis engine removed. The overlay UI is useless without TDG grades. The MCP server and plugin system are explicitly targeted for removal in this milestone. This phase also resolves the highest-risk unknown (PMAT library API).

**Delivers:** PMAT TDG grades visible on treemap nodes; PMAT health/mutation panel replacing current health panels; MCP server and plugin system removed; custom metrics engine deleted.

**Addresses:** Table-stakes TDG grade display, PMAT health score panel, mutation testing results, anti-features (MCP removal, plugin removal, custom analysis engine removal).

**Avoids:** Mixed PMAT + sentrux grades in same frame (explicit source tagging + `None`-out strategy); monolithic migration (additive-first: introduce `ScanReports.pmat` before deleting old fields).

**Research flag:** NEEDS RESEARCH. PMAT's exact public API surface is unconfirmed (LOW confidence). Phase planning must begin with a spike to read PMAT's `Cargo.toml` and `lib.rs` (if it exists) and determine subprocess vs. library path before committing to the full phase plan.

### Phase 2: Git Diff Overlay

**Rationale:** Git history is the "past" layer of the triple overlay thesis. It is independent of the GSD overlay and can be shipped before Phase 3. It uses only existing `git2` infrastructure with no new dependencies. The `ColorMode` serde fix (Pitfall 4) must land here before the first new variant is added.

**Delivers:** Git diff overlay on treemap with 15min/1h/1d/1w/30d selectable time windows; overlay mode switcher (TDG / git-diff toggle); time window selector in toolbar.

**Addresses:** Git diff overlay differentiator, overlay mode switcher, time window selector UI.

**Avoids:** Scanner thread blocking (separate `DiffOverlay` command / `DiffReady` message on its own channel, not blocking `ScanMsg::Complete`); `ColorMode` serde breakage (`#[serde(other)]` fallback added before this phase); `MAX_FILES_PER_COMMIT` duplication consolidated before writing the new git walk.

**Research flag:** Standard patterns — git2 revwalk, crossbeam channel extension. Skip research phase.

### Phase 3: GSD Phase Overlay

**Rationale:** GSD overlay is the "future" layer. It is independent of Phase 2 and can proceed in parallel or sequentially. It requires Phase 1 (overlay infrastructure, `AppState.gsd_phase_map`) but not Phase 2. It is higher complexity than the git overlay due to path matching brittleness.

**Delivers:** GSD phase overlay on treemap coloring files by planning phase; overlay mode switcher extended to three modes (TDG / git-diff / GSD); GSD overlay panel showing phase name and status per file.

**Addresses:** GSD phase overlay differentiator, file-to-phase mapping, overlay mode switcher (three modes).

**Avoids:** Path matching brittleness (path normalization utility tested with absolute, relative, `./`-prefixed, and wrong-case paths before shipping); zero-match silent failures (status bar warning when `GsdPhaseMap` is empty despite `.planning/` existing).

**Research flag:** Low confidence on `.planning/` file format stability. Phase planning should confirm the exact GSD markdown schema used in production projects before building the parser. Reading this project's own `.planning/` files is the best source.

### Phase Ordering Rationale

- Phase 1 must come first because TDG grades are a declared prerequisite and the old analysis engine removal creates the architectural space for overlays to coexist cleanly.
- Phases 2 and 3 are independent in code but Phase 2 is lower risk (established patterns, no path matching complexity) so ships first to deliver user value earlier.
- The overlay mode switcher is built incrementally: Phase 1 introduces the pattern with a single mode, Phase 2 adds the second mode, Phase 3 completes the triple overlay.
- Deprecated `ScanReports` fields (`health`, `arch`, `test_gaps`) coexist with new `pmat` field during Phase 1 transition; deletion happens at end of Phase 1 only after all consuming UI is ported.

### Research Flags

Phases needing deeper research during planning:
- **Phase 1:** PMAT library API surface — spike required before phase can be fully planned. Confirm `[lib]` target, public types, and subprocess vs. library integration decision. This is the single most important unknown in the project.
- **Phase 3:** GSD markdown schema — read production `.planning/` files to confirm the exact file format before building the parser. The regex approach is sound but the patterns depend on knowing the actual file format.

Phases with standard patterns (skip research phase):
- **Phase 2:** git diff overlay — `git2` revwalk pattern is well-established and the existing `git_walker.rs` is a working template. Channel extension pattern is standard for this codebase.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Existing codebase is the authoritative source; no new crates needed for git/GSD overlays; subprocess PMAT integration verified as viable pattern |
| Features | MEDIUM | Existing codebase confirms table stakes; PMAT capability claims from README only (not verified via docs.rs); competitor analysis from training knowledge, not live research |
| Architecture | HIGH | Directly grounded in existing source files (`channels.rs`, `scan_threads.rs`, `layout/types.rs`, `renderer/rects.rs`); patterns are extensions of working code, not speculation |
| Pitfalls | HIGH (sentrux) / LOW (PMAT) | All sentrux-internal pitfalls are grounded in code; PMAT API pitfalls are inferred from README-only access |

**Overall confidence:** MEDIUM

### Gaps to Address

- **PMAT library API surface (blocker):** The single most important unknown. Cannot confirm subprocess vs. library integration until PMAT's `Cargo.toml` and public module surface is read. Handle by making this Phase 1 Task 1 — a spike that reads the source before any integration code is written.
- **PMAT TypeScript/JS analysis quality:** PMAT's Rust support is documented; TS/JS support is inferred. Before removing `analysis/plugin/` and tree-sitter query files for TypeScript, verify PMAT produces correct import graphs on a representative TypeScript project.
- **PMAT transitive dependency versions:** `tree-sitter`, `tokio`, and `tokei` version conflicts are likely but not confirmed. `cargo tree -d` is the resolution tool — must run before any integration code.
- **GSD markdown schema in production:** The `.planning/` parser will be built against the markdown format used in actual GSD projects. The parser design should be confirmed against real files, not just the template. Sentrux's own `.planning/` directory is the best test corpus.
- **tokei vs. PMAT LOC counts:** If PMAT exposes per-file LOC, there will be a discrepancy with tokei's counts (treemap node size). One source of truth must be chosen. Handle during Phase 1 metric consolidation.

---

## Sources

### Primary (HIGH confidence)
- `sentrux-core/src/app/channels.rs` — ScanReports and channel architecture
- `sentrux-core/src/layout/types.rs` — ColorMode enum and serialization
- `sentrux-core/src/metrics/evo/git_walker.rs` — git walk implementation and constants
- `sentrux-core/src/analysis/git.rs` — path normalization pattern
- `sentrux-core/src/app/state.rs` — AppState structure and per-frame amortization
- `sentrux-core/src/core/heat.rs` — overlay color pattern (template for new overlays)
- `.planning/codebase/ARCHITECTURE.md` — layer architecture (authoritative)
- `.planning/codebase/CONCERNS.md` — known issues (eprintln noise, MAX_FILES_PER_COMMIT duplication)

### Secondary (MEDIUM confidence)
- PMAT GitHub README (paiml/paiml-mcp-agent-toolkit) — TDG grading, health scoring, mutation testing features confirmed; library API not confirmed
- `.planning/PROJECT.md` — milestone structure and integration constraints
- Training knowledge of peer tools (CodeScene, SonarQube, Structure101, CodeCharta) — competitive positioning

### Tertiary (LOW confidence)
- PMAT Cargo.toml and lib.rs — not directly read; library surface inferred from README and GitHub file search (0 results for `lib.rs`)
- PMAT docs.rs — could not be fetched during research; treat all PMAT API signatures as unverified

---

*Research completed: 2026-03-14*
*Ready for roadmap: yes*
