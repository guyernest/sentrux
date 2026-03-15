# Phase 5: Improve Time Alignment - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Align three temporal streams (wall-clock time, git commits, GSD milestones/phases) into a hierarchical navigation bar with click-to-zoom filtering and diff-over-time analysis. The hierarchical bar replaces the Phase 4 proportional phase navigator with a unified 3-tier stack: milestones → phases → commits, with time ticks above. Clicking any tier filters the view from that point to present. Score deltas (TDG, coverage, clippy) are shown on treemap nodes and in the detail panel, comparing selected past state to current state.

</domain>

<decisions>
## Implementation Decisions

### Hierarchical Bar Layout
- **Replace** the Phase 4 proportional phase navigator entirely with a unified 3-tier stack
- Stack order (top to bottom): time ticks → milestones → phases → commits
- **Equal sizing within parent**: milestones get equal width; phases divide their milestone's width equally; commits divide their phase's width equally
- **Milestone bar hidden when only one milestone exists** — time ticks sit directly above phases bar
- Stack adapts: if milestone bar is hidden, time ticks → phases → commits
- Position: below toolbar/legend, same location as the current phase navigator (between toolbar and treemap canvas)

### Click-to-Zoom Filtering
- Clicking a milestone/phase/commit filters the view to show **from selected point to present**
- **Treemap re-colors** to the filtered range: files changed in that range show intensity colors; unchanged files go gray
- Clicking a single commit shows diff from that commit to HEAD (cumulative changes since)
- **Reset mechanism: both** — a dedicated ×/↺ button appears when filtered, AND clicking the already-selected segment deselects it
- When clicking a phase, the commit bar shows only that phase's commits
- When clicking a milestone, the phase bar shows only that milestone's phases, commit bar shows those phases' commits

### Commit Segment Labels
- Short hash only (e.g., "a1ac93b") displayed in each commit segment
- Hover tooltip shows: full commit message, author, date, file count, lines changed

### Time Ticks
- **Dedicated thin row** above the milestone bar (or above phases when milestone bar is hidden)
- **Auto-scaling granularity**: adjusts based on visible range — hours when zoomed into a phase, days/weeks when viewing full project — keeps ~5-10 ticks visible
- **Compress gaps**: idle periods (weekends, no-commit stretches) don't take visual space; ticks only mark periods with activity; labels show actual dates/times

### Diff-over-Time Analysis
- **Metrics**: TDG grade change, coverage % change, clippy warning count change
- **Aggregation levels**: file-level deltas in detail panel + directory-level rollup on directory nodes (avg TDG change, total coverage change)
- **Treemap visualization**: small ↑↓ arrow indicators on file/directory nodes — green arrow up = improved, red arrow down = regressed — subtle overlay on existing treemap colors
- **Analysis snapshots**: stored automatically on every scan completion to `.sentrux/snapshots/{timestamp}.json` — git tracks the history
- Snapshot contains per-file: TDG grade, coverage %, clippy warning count
- Delta comparison: when a time range is selected, load the nearest snapshot to the range start and diff against current

### Claude's Discretion
- Exact time tick rendering (font, spacing, tick mark style)
- Commit bar segment minimum width and overflow handling for phases with many commits
- Snapshot file format details and pruning strategy for old snapshots
- Animation/transition effects when filtering (instant vs animated)
- How to handle the commit bar when there are 100+ commits in a phase (virtualization, grouping, or scroll)
- Arrow indicator sizing and exact placement on treemap nodes
- Color choice for the hierarchical bar segments (reuse phase status colors or new palette)

</decisions>

<specifics>
## Specific Ideas

- "If we have two milestones, 3 phases in the first milestone, and 10 phases in the second milestone, we will have a bar for the milestones divided to equal parts, and a bar for the phases that divides the first half to 3 equal parts and the second half to 10 equal parts" — nested equal division is the core layout principle
- "Above the top milestone we can have the time ticks of hours and days" — time ticks as a calendar reference layer
- "When a user clicks a milestone, the toolbar of the milestones shows only the milestones from the selected one to the present. Same logic for phases and commits" — progressive drill-down with consistent from-selected-to-present semantics
- "We should have a reset button to get all the milestones back" — explicit escape hatch for filtered state
- "The goal of the visualization project is to allow the user to track the progress of the project and improve not just the functionality, but also the quality — test coverage, reduced complexity, and risks" — quality improvement tracking is the driving purpose

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `app/draw_panels.rs:draw_gsd_phase_navigator()` — current proportional phase bar; will be replaced but its interaction pattern (click → set DiffWindow, hover → tooltip) is reusable
- `metrics/evo/git_walker.rs:DiffWindow` — already supports TimesSecs, CommitCount, SinceLastTag, CommitRange; extendable for new filtering
- `core/pmat_types.rs:PhaseInfo` — phase data with commit_range, status, files; feeds the phases tier
- `analysis/git_diff_adapter.rs:compute_git_diff_report()` — background git diff computation; reusable for filtered-range diffs
- `renderer/colors.rs:gsd_phase_color()` — phase status colors (green/amber/blue) for bar segments
- `app/state.rs:git_diff_window`, `selected_phase_idx` — existing filter state fields

### Established Patterns
- Background thread via `maybe_spawn_*_thread` → `ScanMsg::*Ready` → AppState → RenderContext → file_color()
- ColorMode dispatch in `rects.rs:file_color()` — add delta arrow overlay here
- On-demand data: git diff uses `git_diff_requested` flag → thread spawn → result stored on AppState
- UserPrefs persistence for mode state via `#[serde(default)]`
- Proportional bar rendering via `ui.painter()` with `ui.interact()` for click/hover on rects

### Integration Points
- `app/draw_panels.rs` — replace `draw_gsd_phase_navigator()` with `draw_timeline_navigator()`
- `app/state.rs` — add timeline filter state (selected_milestone_idx, selected_commit_hash, filter_active)
- `app/channels.rs` — add `ScanMsg::SnapshotStored` for scan-completion snapshot persistence
- `renderer/rects.rs` — add delta arrow overlay drawing in `draw_file_rect()`
- `.sentrux/snapshots/` — new directory for analysis snapshot JSON files

</code_context>

<deferred>
## Deferred Ideas

- Full unified time dial widget (all three dimensions in a single circular control) — v2
- Animated phase playback (watch the project evolve phase by phase) — v2
- PR/branch comparison view — v2
- Repo-wide summary delta widget (overall TDG/coverage trend) — could be added later as a small status bar widget
- Delta-based ColorMode (entire treemap colored by improvement/regression intensity) — v2

</deferred>

---

*Phase: 05-improve-time-alignment*
*Context gathered: 2026-03-15*
