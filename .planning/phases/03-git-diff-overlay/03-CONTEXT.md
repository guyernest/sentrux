# Phase 3: Git Diff Overlay - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Color-code treemap nodes by git change recency within selectable windows. Users switch between existing ColorModes (TDG/Coverage/Risk/Language/Heat/Git/Mono) and the new GitDiff mode via the toolbar. Supports both time-based and commit-count-based windows, plus "since last tag". Includes a color legend and session persistence.

</domain>

<decisions>
## Implementation Decisions

### Time Window Selection
- **Preset buttons** in toolbar (compact, one-click) — two groups:
  - Time-based: 15m | 1h | 1d | 1w | (since last tag)
  - Commit-based: 1 commit | 5 commits | custom N
- "Since last tag" uses `git describe --tags --abbrev=0` to find the most recent tag
- Custom commit count: user types a number in a small input field next to the preset buttons
- Window selector only visible when GitDiff ColorMode is active
- GSD phase/milestone commit ranges deferred to Phase 4

### Change Intensity
- **Lines changed + commit count** combined — files changed often AND with many lines are hottest
- Formula: combine total lines added/removed with number of commits touching the file in the window
- Unchanged files visually **muted** (same as Coverage/Risk behavior for missing data)
- **New files** (created within the window) get a **distinct color** (different hue from the hot-cold gradient) so they stand out
- **Deleted files** not shown (they're not in the snapshot — natural behavior)

### Metric Deltas
- File detail panel shows **score deltas** when a changed file is selected in GitDiff mode:
  - TDG grade change: "B → A (improved)"
  - Coverage change: "85% → 78% (-7%)"
  - Clippy change: "5 → 3 (-2)"
- Source: **git-stored analysis snapshots** — persist analysis results (TDG, coverage, clippy scores) alongside commits
- Storage strategy: use git itself for history — store analysis JSON in `.sentrux/` and commit it, so `git log` can retrieve any historical state
- Cache is effectively unbounded (git manages the history)
- On file click in GitDiff mode: retrieve the analysis snapshot from the selected window boundary, diff against current

### Overlay Persistence
- Selected overlay mode (ColorMode) persists across sessions (already implemented via prefs)
- Selected time window persists across sessions (save to prefs)

### Claude's Discretion
- Exact formula for combining lines-changed and commit-count into a single intensity value
- Color choice for "new file" distinct indicator
- Layout of the preset buttons vs custom input in the toolbar
- How to store/retrieve analysis snapshots from git history efficiently
- Whether to run analysis snapshot on every scan or on explicit save

</decisions>

<specifics>
## Specific Ideas

- "We should connect it to the phases and milestones of the GSD" — commit ranges mapped to GSD phases belong in Phase 4
- "We need the custom number as we might have a set of commits that are related" — supports reviewing a logical change set
- "We can use git for these states and store them for as long as needed" — analysis snapshots live in git, not a separate DB
- The "last 1 commit" preset acts as a visual `git diff HEAD~1` — powerful for reviewing what just changed

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `metrics/evo/git_walker.rs` — existing `walk_git_log` does time-bounded commit walking with per-file line deltas
- `git2 0.20` crate — already in dependencies, provides revwalk, diff, tag lookups
- `pmat_adapter.rs:run_pmat_command<T>` — subprocess pattern for re-running PMAT on historical states
- `renderer/colors.rs:coverage_color` — gradient function pattern reusable for change intensity
- `app/channels.rs:ScanMsg::CoverageReady` — existing on-demand result delivery pattern (model for DiffReady)

### Established Patterns
- `AppState` stores reports → `RenderContext` carries references → `file_color()` dispatches by ColorMode
- On-demand computation: Coverage uses toolbar button → background thread → ScanMsg → AppState update
- External weights via `WeightConfig.external_weights` for SizeMode (could add a "GitChurn" SizeMode too)

### Integration Points
- `layout/types.rs:ColorMode` — add `GitDiff` variant (before Monochrome, serde(other) already handles unknown)
- `renderer/rects.rs:file_color()` — add `color_by_git_diff` dispatch arm
- `app/toolbar.rs` — time window selector UI (only when GitDiff mode active)
- `app/state.rs` — add `git_diff_report: Option<GitDiffReport>`, window selection state

</code_context>

<deferred>
## Deferred Ideas

- GSD phase/milestone commit ranges as window presets — Phase 4
- Animated playback of git changes over time — v2
- PR/branch comparison view — v2

</deferred>

---

*Phase: 03-git-diff-overlay*
*Context gathered: 2026-03-15*
