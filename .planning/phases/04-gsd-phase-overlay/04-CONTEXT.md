# Phase 4: GSD Phase Overlay - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Color-code treemap files by which GSD planning phase touches them. Completed phases are green, in-progress phases amber, planned phases blue. Unassociated files muted. Hover shows phase name and goal. Phase-to-file mapping comes from PLAN.md files_modified and SUMMARY.md key-files. Phase-to-commit mapping uses commit message parsing with time-based fallback. A separate phase navigator panel shows the phase timeline with clickable regions for unified time navigation.

</domain>

<decisions>
## Implementation Decisions

### Phase-to-File Mapping
- **Two sources**: PLAN.md `files_modified` frontmatter + SUMMARY.md `key-files` sections
- **Exact path matching** after normalizing `./` prefixes — no fuzzy/basename matching
- When a file appears in **multiple phases**, color by the **most recent** phase that touched it
- Phase status (completed/in-progress/planned) read from ROADMAP.md checkbox status

### Color Scheme
- **Three states**: completed = green-ish, in-progress = amber/yellow, planned = blue-ish
- **Unassociated files** = muted gray (same `NO_DATA_GRAY` pattern)
- Hover tooltip shows phase number, name, and goal

### Phase Commit Ranges
- **Commit message parsing first** — look for GSD conventions like `feat(02-01):`, `docs(phase-3):`
- **Time-based fallback** — use completion dates from ROADMAP.md for commits without phase markers
- Phase boundaries defined by start/end commits
- Scrolling back one phase = show that phase's commit range

### Unified Time Navigation
- **Separate phase navigator panel** (not toolbar presets) showing clickable phase timeline
- Commits, phases, and time are **linked dimensions** — scrolling by one auto-syncs the others
- Example: scrolling back 2 phases → commit count and time range adjust to match those phases' boundaries
- The phase navigator serves as the anchor for the GSD overlay, while the existing toolbar presets handle git diff time/commit windows
- When a phase is clicked in the navigator, the GitDiff window auto-adjusts to that phase's commit range

### Claude's Discretion
- Navigator panel layout and positioning (side panel, bottom panel, floating?)
- How to render the phase timeline (horizontal bar, vertical list, or other)
- Exact commit message parsing regex patterns
- How to handle phases with no commits (purely planning phases)
- Performance of parsing all PLAN.md/SUMMARY.md files on scan

</decisions>

<specifics>
## Specific Ideas

- "We can scroll back in time by git commits/tags, gsd phases/milestones, or time minutes/hours/days... The other dimension should adjust automatically" — unified time navigation is the core UX innovation
- "If the last 2 phases had 7 commits, when scrolling back by another phase, the commit dimension will show the value 7 on the commit dial" — dimensions cross-reference and auto-sync
- "The focus of a review is starting with the most recent git commit or the most recent (current) phase" — most recent is the default view, scrolling goes backward
- The phase navigator connects the "future" (GSD planning) layer with the "past" (git history) layer — the triple overlay thesis is complete

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `analysis/git_diff_adapter.rs:compute_git_diff_report` — background computation pattern
- `metrics/evo/git_walker.rs:DiffWindow` — time/commit window model, extensible for phase boundaries
- `renderer/colors.rs:NO_DATA_GRAY` — muted gray constant
- `app/draw_panels.rs:draw_color_legend` — color legend dispatch pattern
- `app/toolbar.rs:draw_git_diff_controls` — preset button UI pattern
- `layout/types.rs:ColorMode` — add GsdPhase variant before Monochrome

### Established Patterns
- Background thread via `maybe_spawn_*_thread` → `ScanMsg::*Ready` → AppState → RenderContext → file_color()
- ColorMode dispatch in `rects.rs:file_color()` — add `color_by_gsd_phase` arm
- On-demand data: git diff uses `git_diff_requested` flag → thread spawn → result stored on AppState
- UserPrefs persistence for mode state via `#[serde(default)]`

### Integration Points
- `.planning/` directory: parse ROADMAP.md, PLAN.md files, SUMMARY.md files
- `app/channels.rs` — add `ScanMsg::GsdPhaseReady`
- `app/state.rs` — add `gsd_phase_report`, navigator panel state
- `renderer/mod.rs:RenderContext` — add `gsd_phase_report` reference
- Phase 3's `DiffWindow` may need a `PhaseRange(phase_start_commit, phase_end_commit)` variant

</code_context>

<deferred>
## Deferred Ideas

- Full unified time dial widget (all three dimensions in a single control) — v2 milestone
- Phase-level metric aggregation (average TDG grade per phase, coverage trend per phase) — v2
- Animated phase playback (watch the project evolve phase by phase) — v2

</deferred>

---

*Phase: 04-gsd-phase-overlay*
*Context gathered: 2026-03-15*
