# Roadmap: Sentrux — PMAT-Powered Code Visualization

## Overview

Sentrux ships four phases that transform an existing treemap visualizer into a three-layer codebase health tool. Phase 1 removes the dead weight (MCP server, plugin system, extra language support) so the codebase is clean going into the PMAT work. Phase 2 integrates PMAT as the analysis engine and displays TDG grades on every treemap node. Phases 3 and 4 add the two overlay modes that make the product unique — git diff showing the past, GSD phase overlay showing the future. By the end of Phase 4, a developer can open any Rust project and see past changes, present health, and planned future work on a single interactive treemap.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Cleanup** - Remove MCP server, plugin system, and non-Rust/TS/JS language support (completed 2026-03-15)
- [x] **Phase 2: PMAT Integration** - Integrate PMAT as analysis engine; display TDG grades and health/mutation panels (completed 2026-03-15)
- [ ] **Phase 3: Git Diff Overlay** - Color-code treemap nodes by git change recency with selectable time windows
- [ ] **Phase 4: GSD Phase Overlay** - Color-code treemap nodes by GSD planning phase; complete triple overlay system

## Phase Details

### Phase 1: Cleanup
**Goal**: The codebase contains only the capabilities it will carry forward — no MCP server, no plugin system, no languages beyond Rust/TypeScript/JavaScript
**Depends on**: Nothing (first phase)
**Requirements**: CLEN-01, CLEN-02, CLEN-03
**Success Criteria** (what must be TRUE):
  1. Running sentrux no longer starts or exposes an MCP server endpoint
  2. No plugin loading code executes at startup; grammar plugin files are absent from the build
  3. Sentrux correctly scans Rust, TypeScript, and JavaScript files and silently skips all other file types without error
  4. The binary builds cleanly with no dead-code warnings from removed subsystems
**Plans:** 2/2 plans complete

Plans:
- [x] 01-01-PLAN.md — Remove MCP server, plugin system, whatif module, and evolution alias
- [x] 01-02-PLAN.md — Rewrite lang_registry as static 3-language registry with compiled-in grammars

### Phase 2: PMAT Integration
**Goal**: Every file node in the treemap displays a PMAT TDG grade, and the health/mutation panels show PMAT data — sentrux's own analysis engine is gone
**Depends on**: Phase 1
**Requirements**: PMAT-01, PMAT-02, PMAT-03, PMAT-04, PMAT-05, PMAT-06, PMAT-07, CLEN-04
**Success Criteria** (what must be TRUE):
  1. Opening a Rust project shows TDG grade badges (A+ through F) on file nodes in the treemap within one scan cycle
  2. Treemap node color reflects TDG grade (green for A+/A, red for D/F) with no grades from the old sentrux grading system visible anywhere
  3. The health panel shows PMAT's health score for the scanned project
  4. The file detail panel shows TDG component breakdown (scores, penalties, critical defects) when a file is selected
  5. No sentrux-internal grading or rating code remains in the compiled binary (old `metrics/grading`, `metrics/stability`, `metrics/whatif` subtrees deleted)
**Plans:** 4/4 plans complete

Plans:
- [x] 02-01-PLAN.md — PMAT types, subprocess adapter, and .unwrap() cleanup
- [x] 02-02-PLAN.md — ColorMode pruning, TDG grade coloring, and badge rendering
- [x] 02-03-PLAN.md — PMAT scan pipeline wiring and health/file-detail panels
- [ ] 02-04-PLAN.md — Delete old metrics engine and rewire check/gate CLI to PMAT

### Phase 02.1: Rust Deep Analysis (INSERTED)

**Goal**: Sentrux provides deep Rust-specific analysis beyond TDG grades — PMAT code rank (PageRank, centrality, community detection), test coverage via cargo-llvm-cov, and clippy advanced linting — all visualized on the treemap and accessible in detail panels
**Depends on:** Phase 2
**Requirements**: RANK-01, RANK-02, RANK-03, COV-01, COV-02, COV-03, CLIP-01, CLIP-02
**Success Criteria** (what must be TRUE):
  1. PMAT code rank data (PageRank scores, centrality measures) is displayed per-file in the detail panel
  2. A new ColorMode variant shows files colored by architectural importance (PageRank rank)
  3. Test coverage percentages from cargo-llvm-cov are shown per-file in the detail panel
  4. A new ColorMode variant shows files colored by test coverage (green = well-covered, red = uncovered)
  5. Clippy warnings (pedantic/nursery categories) are counted per-file and shown in the detail panel
**Plans:** 3 plans

Plans:
- [ ] 02.1-01-PLAN.md — Types, subprocess adapters, color functions, and ColorMode variants for graph-metrics, coverage, and clippy
- [ ] 02.1-02-PLAN.md — Wire reports through scan pipeline, AppState, RenderContext, and file_color() dispatch
- [ ] 02.1-03-PLAN.md — File detail panel sections, coverage toolbar button, and community highlight interaction

### Phase 3: Git Diff Overlay
**Goal**: A developer can switch to git diff mode and immediately see which files changed recently — color intensity tells them how much changed, and they control the time window
**Depends on**: Phase 2
**Requirements**: GDIT-01, GDIT-02, GDIT-03, GDIT-04, GDIT-05, OVRL-01, OVRL-02, OVRL-03
**Success Criteria** (what must be TRUE):
  1. A toolbar toggle switches the treemap between TDG grade mode and git diff mode without triggering a full rescan
  2. In git diff mode, files changed within the selected time window are colored from cool (few lines changed) to hot (many lines changed); unchanged files are visually muted
  3. The time window selector in the toolbar offers at minimum 15 min, 1 h, 1 day, and 1 week; changing the window updates the overlay without freezing the UI
  4. A color legend below the toolbar explains the current overlay's color mapping
  5. The selected overlay mode is restored when the app is reopened
**Plans**: TBD

### Phase 4: GSD Phase Overlay
**Goal**: A developer working on a GSD-planned project can switch to GSD phase mode and see which files each phase will touch (or has touched), making the plan spatially visible on the treemap
**Depends on**: Phase 3
**Requirements**: GSDP-01, GSDP-02, GSDP-03, GSDP-04, GSDP-05
**Success Criteria** (what must be TRUE):
  1. In GSD phase mode, treemap nodes are colored by phase — completed phases use a distinct color from planned phases, and files not in any phase are muted
  2. Hovering a colored node shows a tooltip with the phase number, phase name, and phase goal
  3. Sentrux reads phase-to-file mappings from `.planning/` plan files in the scanned project without requiring any configuration
  4. The overlay mode switcher cycles through all three modes (TDG / Git Diff / GSD Phase) with a single toolbar control
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 2.1 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Cleanup | 2/2 | Complete    | 2026-03-15 |
| 2. PMAT Integration | 3/4 | Complete    | 2026-03-15 |
| 2.1 Rust Deep Analysis | 0/3 | Planning complete | - |
| 3. Git Diff Overlay | 0/TBD | Not started | - |
| 4. GSD Phase Overlay | 0/TBD | Not started | - |
