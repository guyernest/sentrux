# Requirements: Sentrux — PMAT-Powered Code Visualization

**Defined:** 2026-03-14
**Core Value:** Show developers the health and evolution of their codebase through an interactive treemap powered by PMAT's analysis — past, present, and future.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### PMAT Integration

- [x] **PMAT-01**: Sentrux integrates PMAT as analysis backend (library crate or subprocess, determined by API spike)
- [x] **PMAT-02**: PMAT TDG grades (A+ through F) are computed for scanned projects
- [x] **PMAT-03**: TDG grade badges are displayed on treemap file/directory nodes
- [x] **PMAT-04**: TDG grade color mode colors treemap nodes by grade (green A+ → red F gradient)
- [x] **PMAT-05**: PMAT health score is displayed in a dedicated GUI panel
- [x] **PMAT-06**: PMAT mutation testing results are accessible through a GUI panel
- [x] **PMAT-07**: Sentrux's own grading/rating system is removed and replaced by PMAT metrics

### Overlay System

- [x] **OVRL-01**: User can switch between overlay modes via toolbar toggle (TDG / Git Diff / GSD Phase)
- [x] **OVRL-02**: Active overlay mode has a visible color legend explaining the color mapping
- [x] **OVRL-03**: Overlay mode persists across sessions (saved in preferences)

### Git Diff Overlay

- [x] **GDIT-01**: User can see treemap nodes color-coded by git changes within a selectable time window
- [x] **GDIT-02**: Time window options include at minimum: 15 minutes, 1 hour, 1 day, 1 week
- [x] **GDIT-03**: Changed files show intensity based on number of lines changed (hotter = more changes)
- [x] **GDIT-04**: Unchanged files are visually muted so changed files stand out
- [x] **GDIT-05**: Git diff computation runs on a background thread without freezing the UI

### GSD Phase Overlay

- [x] **GSDP-01**: User can see treemap nodes color-coded by which GSD phase touches them
- [ ] **GSDP-02**: Past phases (completed) use a distinct color scheme from future phases (planned)
- [x] **GSDP-03**: Phase information is read from `.planning/` directory files (ROADMAP.md, plan files)
- [x] **GSDP-04**: Files not associated with any phase are visually muted
- [ ] **GSDP-05**: Hovering a colored node shows which phase and its goal

### PMAT Code Rank

- [x] **RANK-01**: PMAT code rank data (PageRank, centrality) is computed via `pmat analyze graph-metrics` subprocess
- [x] **RANK-02**: File detail panel shows PageRank score, centrality measures, and community assignment for selected file
- [x] **RANK-03**: A new ColorMode variant colors treemap nodes by architectural importance (PageRank rank)

### Test Coverage

- [x] **COV-01**: Test coverage data is collected via `cargo llvm-cov` subprocess and parsed into per-file percentages
- [x] **COV-02**: File detail panel shows line coverage percentage for selected file
- [x] **COV-03**: A new ColorMode variant colors treemap nodes by test coverage (green = well-covered, red = uncovered)

### Clippy Analysis

- [x] **CLIP-01**: Clippy warnings (pedantic + selected nursery lints) are collected via `cargo clippy` subprocess and counted per-file
- [x] **CLIP-02**: File detail panel shows clippy warning count and categories for selected file

### Cleanup

- [x] **CLEN-01**: MCP server mode is removed from sentrux (PMAT provides MCP)
- [x] **CLEN-02**: Plugin system (runtime tree-sitter grammar loading) is removed
- [x] **CLEN-03**: Language support is narrowed to Rust, TypeScript, and JavaScript only
- [x] **CLEN-04**: Unused analysis code (sentrux's own metrics engine) is removed after PMAT replaces it

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Advanced Overlays

- **AOVR-01**: User can combine two overlays simultaneously (e.g., TDG + git diff)
- **AOVR-02**: Custom time range picker for git diff (date-to-date instead of fixed windows)
- **AOVR-03**: Animated playback of git changes over time

### PMAT Deep Integration

- **PDEP-01**: PMAT semantic search results displayed on treemap (highlight matching files)
- **PDEP-02**: PMAT compliance check results in GUI panel
- **PDEP-03**: PMAT kaizen auto-fix suggestions surfaced in GUI

## Out of Scope

| Feature | Reason |
|---------|--------|
| MCP server mode | PMAT already provides MCP server for AI agents |
| Plugin system | Narrowing to Rust/TS/JS; PMAT handles analysis |
| Languages beyond Rust/TS/JS | User's projects don't need them |
| Mobile or web deployment | Desktop-only tool |
| Own analysis engine | PMAT replaces all code analysis |
| Real-time collaboration | Single-user desktop tool |
| CI/CD integration | Out of scope for visualization tool; PMAT CLI handles CI |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| PMAT-01 | Phase 2 | Complete |
| PMAT-02 | Phase 2 | Complete |
| PMAT-03 | Phase 2 | Complete |
| PMAT-04 | Phase 2 | Complete |
| PMAT-05 | Phase 2 | Complete |
| PMAT-06 | Phase 2 | Complete |
| PMAT-07 | Phase 2 | Complete |
| OVRL-01 | Phase 3 | Complete |
| OVRL-02 | Phase 3 | Complete |
| OVRL-03 | Phase 3 | Complete |
| GDIT-01 | Phase 3 | Complete |
| GDIT-02 | Phase 3 | Complete |
| GDIT-03 | Phase 3 | Complete |
| GDIT-04 | Phase 3 | Complete |
| GDIT-05 | Phase 3 | Complete |
| GSDP-01 | Phase 4 | Complete |
| GSDP-02 | Phase 4 | Pending |
| GSDP-03 | Phase 4 | Complete |
| GSDP-04 | Phase 4 | Complete |
| GSDP-05 | Phase 4 | Pending |
| CLEN-01 | Phase 1 | Complete |
| CLEN-02 | Phase 1 | Complete |
| CLEN-03 | Phase 1 | Complete |
| CLEN-04 | Phase 2 | Complete |
| RANK-01 | Phase 2.1 | Complete |
| RANK-02 | Phase 2.1 | Complete |
| RANK-03 | Phase 2.1 | Complete |
| COV-01 | Phase 2.1 | Complete |
| COV-02 | Phase 2.1 | Complete |
| COV-03 | Phase 2.1 | Complete |
| CLIP-01 | Phase 2.1 | Complete |
| CLIP-02 | Phase 2.1 | Complete |

**Coverage:**
- v1 requirements: 24 total
- Mapped to phases: 24
- Unmapped: 0

---
*Requirements defined: 2026-03-14*
*Last updated: 2026-03-14 after roadmap creation*
