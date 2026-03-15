# Architecture Patterns

**Domain:** PMAT integration into an egui treemap visualization tool
**Researched:** 2026-03-14

---

## Recommended Architecture

The existing architecture is already well-suited for this integration. The key insight is that PMAT replaces the `metrics` layer (and partially `analysis`) without touching the thread model, `Snapshot`, layout, or renderer. Overlay data (git diff windows, GSD phases) enters as new `ScanReports` fields and new `ColorMode` variants — the renderer's `file_color` dispatch table already handles this pattern.

### High-Level Diagram

```
┌────────────────────────────────────────────────────────────────────┐
│  sentrux-bin (unchanged)                                           │
│  main.rs — CLI parse, GPU probe, mode dispatch                     │
└─────────────────────────┬──────────────────────────────────────────┘
                          │
┌─────────────────────────▼──────────────────────────────────────────┐
│  sentrux-core                                                       │
│                                                                     │
│  ┌─────────────┐     ┌──────────────────────────────────────────┐  │
│  │  analysis/  │     │  pmat (external Cargo dependency)        │  │
│  │  scanner    │────▶│  pmat::analyze(path) → PmatReport        │  │
│  │  (keeps     │     │  pmat::tdg_grade(files) → TdgReport      │  │
│  │   fs scan,  │     │  pmat::health_score(path) → HealthScore  │  │
│  │   graph     │     └──────────────┬───────────────────────────┘  │
│  │   building) │                    │                               │
│  └──────┬──────┘                    │                               │
│         │ Arc<Snapshot>             │ PmatReport                    │
│         ▼                           ▼                               │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  scanner thread (scan_threads.rs)                           │   │
│  │  1. scan_directory → Snapshot                               │   │
│  │  2. pmat::analyze(root) → PmatReport              NEW       │   │
│  │  3. git_diff_window(root, window) → DiffReport    NEW       │   │
│  │  4. gsd_phase_map(root) → GsdPhaseMap             NEW       │   │
│  │  5. ScanMsg::Complete(snap, gen, ScanReports)               │   │
│  └──────────────────────────┬──────────────────────────────────┘   │
│                             │ ScanMsg                               │
│                             ▼                                       │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  app layer (unchanged thread model)                          │  │
│  │  AppState: + pmat_report, + diff_report, + gsd_phase_map    │  │
│  │  ScanReports: + pmat, + diff, + gsd                         │  │
│  └──────────────────────────┬─────────────────────────────────┘   │
│                             │ LayoutRequest (unchanged)             │
│                             ▼                                       │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  layout thread (unchanged)                                   │  │
│  │  RenderData = layout(Snapshot) — no change needed            │  │
│  └──────────────────────────┬─────────────────────────────────┘   │
│                             │ RenderData                            │
│                             ▼                                       │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  renderer (file_color dispatch extended)                     │  │
│  │  ColorMode::TdgGrade  → color_by_tdg_grade(ctx, path)  NEW  │  │
│  │  ColorMode::GitDiff   → color_by_diff_window(ctx, path) NEW │  │
│  │  ColorMode::GsdPhase  → color_by_gsd_phase(ctx, path)  NEW  │  │
│  └──────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────┘
```

---

## Component Boundaries

### What PMAT Replaces

| Current Component | Fate | Rationale |
|------------------|------|-----------|
| `metrics/grading.rs` | Delete | PMAT's TDG grading (A+ through F) replaces this entirely |
| `metrics/stability.rs` | Delete | PMAT provides coupling/cohesion metrics |
| `metrics/arch/` | Delete | PMAT provides architecture metrics (Martin's metrics) |
| `metrics/dsm/` | Delete | PMAT provides DSM capability |
| `metrics/evo/` | Keep (partially) | Git diff window logic is a superset of churn; may coexist during transition |
| `metrics/testgap/` | Replace | PMAT mutation testing is the new source of truth for test quality |
| `metrics/whatif/` | Delete | PMAT's impact analysis replaces this |
| `metrics/rules/` | Keep or adapt | `.sentrux/rules.toml` may still apply custom rules on top of PMAT grades |
| `analysis/parser/` | Delete | PMAT handles all tree-sitter parsing for Rust/TS/JS |
| `analysis/plugin/` | Delete | Narrowing to 3 languages, no runtime plugin loading needed |
| `queries/` (most) | Delete | Keep only what PMAT doesn't cover; likely delete all after migration |
| `app/mcp_server/` | Delete | PMAT provides its own MCP server; this entire subtree removed |

### What Stays Intact

| Component | Fate | Rationale |
|-----------|------|-----------|
| `analysis/scanner/` | Keep | Filesystem walk, file tree construction, `Snapshot` building |
| `analysis/git.rs` | Keep | Git status on files (adds/removes/untracked) — still needed |
| `analysis/resolver/` | Keep | JS/TS import resolution for graph building |
| `analysis/graph/` | Keep | Import/call/inherit graph building from parsed data |
| `core/` | Extend | `Snapshot` unchanged; add new overlay types |
| `layout/` | Unchanged | Treemap, edge routing, spatial index — no changes |
| `renderer/` | Extend | Add 3 new `file_color` branches; rest unchanged |
| `app/` (shell) | Extend | Thread model unchanged; add overlay UI panels |

### What Is Added

| Component | Location | Purpose |
|-----------|----------|---------|
| `PmatReport` type | `core/pmat_types.rs` | Per-file TDG grades, health score, mutation results |
| `DiffReport` type | `core/overlay_types.rs` | Per-file diff stats for a selected time window |
| `GsdPhaseMap` type | `core/overlay_types.rs` | Per-file GSD phase assignment |
| PMAT adapter | `analysis/pmat_adapter.rs` | Calls PMAT library, maps results to sentrux types |
| Git diff window | `analysis/git_diff_window.rs` | Walks git log for a given time window (extends existing `evo/git_walker.rs`) |
| GSD phase reader | `analysis/gsd_phase_reader.rs` | Reads `.planning/` to build file-to-phase mapping |
| Overlay UI | `app/panels/tdg_display.rs` | Shows PMAT TDG grade breakdown |
| Time window picker | `app/toolbar.rs` (extend) | 15min/1h/1d/1w/1M window selector for git diff overlay |
| GSD overlay panel | `app/panels/gsd_display.rs` | Shows which phase owns each file |

---

## Data Flow

### PMAT Integration Flow

```
scanner_thread:
  1. analysis::scanner::scan_directory(root) → ScanResult
  2. analysis::pmat_adapter::run_pmat(root) → PmatReport
     └── internally: pmat::analyze_directory(root, options)
         returns per-file TDG grades + repo health score
  3. Merge: PmatReport fields indexed by file path
  4. ScanReports { pmat: Some(pmat_report), health: None, arch: None, ... }
  5. ScanMsg::Complete → main thread
  6. AppState.pmat_report = Some(pmat_report)
  7. RenderContext includes &PmatReport (read-only borrow)
  8. renderer::rects::file_color dispatches ColorMode::TdgGrade
     → color_by_tdg_grade(ctx, path) → Color32
```

Key constraint: `PmatReport` is computed on the scanner thread alongside the scan. It is NOT recomputed on every frame. It flows through `ScanReports` → `AppState` just like `HealthReport` does today.

### Git Diff Overlay Flow

The git diff overlay operates on a separate axis from the main scan. It can be refreshed independently when the user changes the time window.

```
User selects time window (toolbar):
  AppState.diff_window = DiffWindow::OneHour  (or 15min, 1d, etc.)
  scan_threads.rs sends: ScanCommand::DiffOverlay { root, window, gen }

DiffOverlay handler (scanner thread):
  analysis::git_diff_window::compute_diff_window(root, window)
  └── walk git log since (now - window)
      for each commit: record files touched + lines added/removed
      aggregate: HashMap<FilePath, DiffStats { adds, removes, commits_touching }>
  Returns: DiffReport

Main thread receives LayoutMsg::DiffReady(DiffReport, gen):
  AppState.diff_report = Some(diff_report)
  (no layout recalculation needed — DiffReport flows into renderer only)

renderer::rects::file_color(ColorMode::GitDiff):
  color_by_diff_window(ctx, path)
  └── ctx.diff_report.get(path)
      → intensity gradient: no changes = cool grey, many changes = hot orange/red
```

Design choice: `DiffReport` does NOT go through the layout thread. It only affects color. Adding a new `DiffMsg` variant (or reusing `ScanMsg`) is sufficient — the layout `RenderData` stays stable.

### GSD Phase Overlay Flow

GSD phase data is static relative to the scan — it reads files from `.planning/`, not from the codebase. It can be computed on the scanner thread during the main scan.

```
scanner_thread (during FullScan):
  analysis::gsd_phase_reader::read_gsd_phases(root)
  └── Look for .planning/ROADMAP.md or .planning/milestones/*.md
      Parse phase definitions and file path mentions
      Build: HashMap<FilePath, GsdPhaseInfo { phase_name, phase_number, status }>
  Returns: GsdPhaseMap

ScanReports.gsd = Some(gsd_phase_map)
AppState.gsd_phase_map = Some(gsd_phase_map)

renderer::rects::file_color(ColorMode::GsdPhase):
  color_by_gsd_phase(ctx, path)
  └── ctx.gsd_phase_map.get(path)
      → color per phase (phase 1 = blue, phase 2 = green, unassigned = neutral)
```

---

## What Existing Abstractions Need to Change

### `Snapshot` — No structural change

`Snapshot` stays immutable and structurally identical. PMAT analysis runs separately and its results do NOT embed into `Snapshot`. Rationale: PMAT analysis may take longer than filesystem scanning (especially mutation testing); they run as separate tasks. Keeping `Snapshot` clean preserves the early `TreeReady` rendering path.

If PMAT returns per-file structural data that improves graph building (e.g., better call graph extraction for Rust), that may eventually replace the `StructuralAnalysis` inside `FileNode`. This is a follow-on concern, not a first-milestone requirement.

### `ScanReports` — New fields added

```rust
pub struct ScanReports {
    pub health: Option<HealthReport>,     // Deprecated — PMAT replaces
    pub arch: Option<ArchReport>,         // Deprecated — PMAT replaces
    pub evolution: Option<EvolutionReport>,
    pub test_gaps: Option<TestGapReport>, // Deprecated — PMAT mutation testing replaces
    pub rules: Option<RuleCheckResult>,   // Keep or adapt

    // New in PMAT milestone:
    pub pmat: Option<PmatReport>,         // PMAT per-file TDG grades + health score
    pub gsd: Option<GsdPhaseMap>,         // GSD phase overlay data
    // DiffReport is NOT here — it comes via its own command/message pair
}
```

During the transition, deprecated fields coexist with new PMAT fields. Remove deprecated fields only after the corresponding metrics/ modules are deleted and all panel UI is ported to PMAT data.

### `AppState` — New fields added

```rust
// New fields alongside existing ones:
pub pmat_report: Option<Arc<PmatReport>>,
pub gsd_phase_map: Option<Arc<GsdPhaseMap>>,
pub diff_report: Option<Arc<DiffReport>>,
pub diff_window: DiffWindow,   // currently selected time window (default: 1 day)
```

`Arc<>` wrapping because these flow into `RenderContext` which the renderer borrows per-frame — avoids cloning.

### `ColorMode` — New variants added

```rust
pub enum ColorMode {
    // Existing variants stay:
    Monochrome, Language, Heat, Age, Churn, Risk, Git, ExecDepth, BlastRadius,

    // New:
    TdgGrade,    // Color by PMAT TDG grade (A+ green → F red)
    GitDiff,     // Color by git changes in selected time window
    GsdPhase,    // Color by GSD phase assignment
}
```

`ColorMode::ALL` and `ColorMode::FREE`/`is_pro()` need updating. TdgGrade should be free (replaces the existing grading). GitDiff and GsdPhase are natural pro-tier candidates.

### `RenderContext` — New borrow fields added

```rust
pub struct RenderContext<'a> {
    // Existing fields unchanged
    pub snapshot: &'a Snapshot,
    pub render_data: &'a RenderData,
    pub viewport: &'a ViewportTransform,
    pub settings: &'a Settings,
    pub color_mode: ColorMode,
    pub theme_config: &'a ThemeConfig,
    pub heat_tracker: &'a HeatTracker,
    // ... other existing fields ...

    // New:
    pub pmat_report: Option<&'a PmatReport>,
    pub diff_report: Option<&'a DiffReport>,
    pub gsd_phase_map: Option<&'a GsdPhaseMap>,
}
```

All three are `Option` because they may not be computed yet (e.g., `pmat_report` is `None` before the first scan completes).

### `ScanCommand` — New variant for diff refresh

```rust
pub enum ScanCommand {
    FullScan { root, limits, gen },
    Rescan { root, changed, old_snap, limits, gen },

    // New:
    DiffOverlay { root: String, window: DiffWindow, gen: u64 },
}
```

The scanner thread handles `DiffOverlay` as a lightweight git-log walk — no file parsing, no layout recalculation needed. Result comes back via a new `ScanMsg` variant or a dedicated channel.

### `ScanMsg` — New variant for diff result

```rust
pub enum ScanMsg {
    Progress(ScanProgress),
    TreeReady(Arc<Snapshot>, u64),
    Complete(Arc<Snapshot>, u64, Box<ScanReports>),
    Error(String, u64),

    // New:
    DiffReady(Arc<DiffReport>, u64),
}
```

---

## Patterns to Follow

### Pattern 1: Adapter Module for External Library

Wrap the PMAT library call in `analysis/pmat_adapter.rs` rather than calling PMAT directly from `scan_threads.rs`. This keeps the scanner thread code readable and isolates the PMAT API surface to one file.

```rust
// sentrux-core/src/analysis/pmat_adapter.rs
pub fn run_pmat_analysis(root: &Path) -> Result<PmatReport, AppError> {
    // Call pmat::analyze_directory or equivalent
    // Map pmat's per-file results into sentrux's PmatReport type
    // Convert pmat errors into AppError
}
```

Confidence on PMAT's exact API: LOW — docs.rs/pmat could not be accessed during this research. The specific function signature (`analyze_directory`, `TdgReport`, etc.) must be verified against PMAT source at integration time. The adapter pattern keeps this uncertainty isolated.

### Pattern 2: Overlay Data Never Enters Snapshot

Neither `PmatReport`, `DiffReport`, nor `GsdPhaseMap` should be embedded in `Snapshot` or `FileNode`. They live separately on `AppState` and are injected into `RenderContext` per-frame. This preserves the existing invariant that `Snapshot` represents raw filesystem + graph state, and overlays are computed interpretations layered on top.

### Pattern 3: DiffWindow as Independent Dimension

The git diff time window is UI state, not scan state. The user can change the window (15min → 1d) without re-scanning. The `DiffOverlay` command runs on the existing scanner thread but is a fast git-log walk, not a full scan. This means:
- No layout recalculation when window changes
- No `TreeReady` for diff — just `DiffReady`
- Toolbar reflects a separate "diff window" selector alongside the main `ColorMode` picker

### Pattern 4: GSD Phase Reader as Best-Effort

The GSD phase reader should be best-effort and never block the scan. If `.planning/` doesn't exist or can't be parsed, `gsd_phase_map` is `None` and the `GsdPhase` ColorMode falls back to the neutral color. This mirrors how `EvolutionReport` is currently `Option` when the directory isn't a git repo.

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Embedding PMAT Data in Snapshot

**What goes wrong:** Adding `tdg_grade: Option<TdgGrade>` to `FileNode` or `Snapshot`.

**Why bad:** Snapshot construction becomes dependent on PMAT completing first, eliminating the `TreeReady` early-rendering path. Rescan becomes entangled with PMAT re-analysis. `Snapshot` serialization format changes.

**Instead:** Keep `PmatReport` as a separate `HashMap<String, PmatFileData>` on `AppState`, looked up by path in the renderer.

### Anti-Pattern 2: Re-running PMAT on Every Rescan

**What goes wrong:** Calling `run_pmat_analysis` inside `handle_rescan` for every file watcher event.

**Why bad:** PMAT analysis (especially mutation testing) can be slow. Filesystem changes like saving a file should not block on a full PMAT re-analysis.

**Instead:** Run PMAT only on `FullScan`. On rescan, preserve the existing `PmatReport` from `AppState`. Optionally, run a partial PMAT re-analysis for only the changed files if PMAT supports incremental analysis.

### Anti-Pattern 3: Putting DiffReport in LayoutRequest

**What goes wrong:** Passing `DiffReport` to the layout thread via `LayoutRequest`.

**Why bad:** Layout does not use color data. The layout thread computes geometry only. Adding `DiffReport` to `LayoutRequest` increases message size, wastes channel capacity, and pollutes the layout/renderer boundary.

**Instead:** `DiffReport` flows directly from scanner thread → `AppState` via `ScanMsg::DiffReady`. It is injected into `RenderContext` per-frame from `AppState`, never touching the layout thread.

### Anti-Pattern 4: Monolithic PMAT Migration

**What goes wrong:** Attempting to delete all of `metrics/`, `analysis/parser/`, and `queries/` in a single commit.

**Why bad:** The visualization will break entirely if PMAT's API is slightly different than expected, or if some metric has no PMAT equivalent.

**Instead:** Introduce PMAT as additive first (new `ScanReports.pmat` field, new `ColorMode::TdgGrade`), verify data quality, then progressively replace old metric panels and finally delete old code. The deprecated fields coexist until all consuming code is ported.

---

## Scalability Considerations

| Concern | Current scale (0.3.12) | At PMAT milestone |
|---------|------------------------|-------------------|
| Scan time | seconds for medium repos | PMAT adds overhead; mutation testing is expensive — run async or on-demand, not every scan |
| Memory | Arc<Snapshot> shared efficiently | PmatReport is another HashMap; Arc-wrap it too |
| Diff window computation | n/a | git2 log walk is bounded by window size; 15min window over a large repo with many commits is fast |
| GSD phase parsing | n/a | Reads a small number of text files; negligible |

---

## Build Order (Phase Dependencies)

The components have clear dependencies that constrain build order:

```
Phase 1: PMAT as library + TDG overlay
  1a. Add pmat to Cargo.toml, verify it compiles in workspace
  1b. Define PmatReport type in core/pmat_types.rs
  1c. Write analysis/pmat_adapter.rs (isolates PMAT API uncertainty)
  1d. Integrate pmat_adapter into scan_threads.rs (after FullScan)
  1e. Add ScanReports.pmat field; thread PmatReport to AppState
  1f. Add ColorMode::TdgGrade; implement file_color branch
  1g. Add TDG panel UI (panels/tdg_display.rs)
      ↓ can proceed in parallel:
  1h. Delete deprecated metrics modules (grading, arch, dsm, testgap, whatif)
  1i. Delete mcp_server/ subtree
  1j. Delete plugin system
  1k. Delete non-Rust/TS/JS query files

Phase 2: Git diff overlay
  Requires: Phase 1 complete, DiffWindow type defined
  2a. Define DiffReport and DiffWindow types in core/overlay_types.rs
  2b. Write analysis/git_diff_window.rs (extends evo/git_walker.rs patterns)
  2c. Add ScanCommand::DiffOverlay + ScanMsg::DiffReady to channels
  2d. Handle DiffOverlay in scanner_thread
  2e. Add diff_window + diff_report to AppState
  2f. Add time window selector to toolbar
  2g. Add ColorMode::GitDiff; implement file_color branch
      (Phase 2 is independent of Phase 3)

Phase 3: GSD phase overlay
  Requires: Phase 1 complete, .planning/ format known
  3a. Define GsdPhaseMap type in core/overlay_types.rs
  3b. Write analysis/gsd_phase_reader.rs
  3c. Add ScanReports.gsd field; thread GsdPhaseMap to AppState
  3d. Add ColorMode::GsdPhase; implement file_color branch
  3e. Add GSD overlay panel UI
```

Phases 2 and 3 are independent of each other and can be built in parallel by separate contributors.

---

## Sources

- Sentrux codebase direct analysis: `sentrux-core/src/core/snapshot.rs`, `channels.rs`, `scan_threads.rs`, `layout/types.rs`, `renderer/rects.rs`, `app/state.rs` (HIGH confidence — read directly)
- PMAT GitHub README (https://github.com/paiml/paiml-mcp-agent-toolkit): confirms published to crates.io as `pmat`, confirms TDG grading / health scoring / mutation testing features (MEDIUM confidence — GitHub landing page only, specific API signatures not verified)
- PMAT API specifics (docs.rs/pmat): could not be fetched during research — treat all PMAT API signatures as LOW confidence requiring verification at integration time
- Architecture patterns (adapter, overlay separation, Arc sharing): derived from existing codebase patterns and standard egui/crossbeam practices (HIGH confidence — evidence in code)
