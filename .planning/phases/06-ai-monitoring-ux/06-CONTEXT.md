# Phase 6: AI Monitoring UX - Context

**Gathered:** 2026-03-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Optimize the treemap for monitoring AI code assistants in real-time. Three capabilities: (1) phase-aware change visibility — auto-switch to GitDiff showing the current phase's changes on app open, with live refresh as new commits appear; (2) git diff +/- line counts rendered as badges on file and directory rects; (3) a smarter risk model that weights centrality by TDG grade complexity so simple hub files don't false-alarm.

</domain>

<decisions>
## Implementation Decisions

### Phase-aware change visibility
- **Auto-switch to GitDiff on open**: when the app opens on a GSD project with an in-progress phase, auto-set `DiffWindow::CommitRange` to that phase's commit range and switch to GitDiff color mode. User sees "what the AI did this phase" immediately.
- **Keep heat overlay alongside**: heat ripples continue for real-time "something just changed" feedback (seconds). GitDiff coloring shows phase-scoped changes (persistent, commit-based). Different layers, different purposes.
- **Auto-refresh on new commits**: when the watcher detects a file change during an active phase, auto-re-trigger the git diff to include the new commit. The user sees AI work appearing on the treemap in real-time.
- **Any phase via timeline click**: auto-diff defaults to the in-progress phase. User can click any phase in the timeline to view its changes (existing Phase 5 behavior — just making the default smarter).

### Git diff +/- counts on nodes
- **Bottom-right badge**: small "+42 -7" text at the bottom-right of file rects. Only shows when GitDiff mode is active. Doesn't compete with delta indicators (top-right) or file name/stats (left).
- **GitHub-style green/red coloring**: "+42" in green, "-7" in red. Universally understood from GitHub/GitLab.
- **Directory aggregation**: directory rects show summed +/- counts across all children — like `git diff --stat` for a subdirectory.

### Smarter risk model
- **Formula**: `centrality × complexity_penalty × coverage_gap × lint_factor`
  - `centrality` = PageRank (existing)
  - `complexity_penalty` = TDG grade converted to 0.0–1.0 multiplier (A+=0.0, A=0.1, B+=0.2...F=1.0)
  - `coverage_gap` = 1.0 − coverage% (existing)
  - `lint_factor` = ln(clippy+1) (existing)
- **Key change**: simple hub files (mod.rs with grade A+) get near-zero complexity_penalty, so their risk is negligible even with high centrality.
- **No-coverage default**: files without coverage data get 50% assumption (neutral — current behavior kept).
- **No user-tunable weights**: fixed formula for v1. Configurable weights are a future phase if needed.

### Change persistence across sessions
- **Phase commit range as session boundary**: on reopen, detect in-progress phase, use its first commit as baseline. No extra prefs state needed — the GSD phase boundary IS the natural session boundary.
- **Works across restarts naturally**: phase commit ranges are parsed from git history, so reopening the app on the same project automatically knows where the current phase started.

### Claude's Discretion
- Minimum rect size for showing +/- badges (below which they're hidden)
- Whether to show +0/-0 for files in the diff range but with no line changes
- How to handle the auto-switch when multiple phases are in-progress (pick the highest numbered one)
- Animation/transition when auto-switching color mode on open

</decisions>

<canonical_refs>
## Canonical References

No external specs — requirements are fully captured in decisions above.

### Prior phase context
- `.planning/phases/03-git-diff-overlay/03-CONTEXT.md` — Git diff color intensity formula, time window decisions
- `.planning/phases/04-gsd-phase-overlay/04-CONTEXT.md` — Phase commit range detection, unified time navigation
- `.planning/phases/05-improve-time-alignment/05-CONTEXT.md` — Timeline navigator, click-to-zoom, delta indicators

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `renderer/colors.rs:compute_raw_risk()` — current risk formula, needs TDG grade multiplier added
- `renderer/rects.rs:draw_delta_arrow()` — per-metric indicators pattern, reusable for +/- badge rendering
- `renderer/rects.rs:color_by_git_diff()` — existing GitDiff coloring, `GitDiffFileEntry` has `total_added`/`total_removed`
- `analysis/gsd_phase_adapter.rs:PhaseInfo.commit_range` — phase start/end commit SHAs
- `app/draw_panels.rs:draw_timeline_navigator()` — already sets DiffWindow::CommitRange on phase click
- `core/heat.rs:HeatTracker` — stays as-is for live ripple feedback

### Established Patterns
- Background thread via `maybe_spawn_*_thread` → `ScanMsg::*Ready` → AppState → RenderContext → file_color()
- `snapshot_write_requested` flag pattern for post-scan triggers
- `pre_timeline_color_mode` save/restore on selection

### Integration Points
- `app/scanning.rs:apply_scan_reports()` — add auto-diff trigger after GSD phase parse completes
- `renderer/rects.rs:draw_file_rect()` — add +/- badge rendering at bottom-right
- `renderer/colors.rs:compute_raw_risk()` — add TDG grade penalty parameter
- `app/state.rs` — add `auto_diff_active: bool` to track whether the auto-switch happened

</code_context>

<specifics>
## Specific Ideas

- "The main idea is that the software engineer is using GSD to plan milestones and phases, and monitor the work that the AI assistants are doing in the background"
- "The UI can be open and show the code changes from the start of the current phase"
- "Currently the change is fading too quickly" — heat overlay is ephemeral; phase-scoped diff is the persistent layer
- "The current false alarm should help us think on a better model" — renderer/mod.rs flagged as high risk because PageRank is high but code is trivially simple (just a data struct)

</specifics>

<deferred>
## Deferred Ideas

- User-tunable risk weights in settings panel — future phase if the fixed formula isn't flexible enough
- Animated playback of changes within a phase — v2
- Notification/alert when risk score increases significantly during AI work — future capability

</deferred>

---

*Phase: 06-ai-monitoring-ux*
*Context gathered: 2026-03-16*
