# Phase 05: Improve Time Alignment - Research

**Researched:** 2026-03-15
**Domain:** egui custom widget rendering, git2 commit enumeration, JSON snapshot persistence, Rust data modeling
**Confidence:** HIGH

## Summary

Phase 5 replaces the proportional phase navigator (drawn in `draw_panels.rs:draw_gsd_phase_navigator()`) with a 3-tier timeline navigator: time ticks → milestones → phases → commits. The layout uses equal-division of parent width, not proportional-to-files as Phase 4 did. Filtering (click-to-zoom) is "from selected point to present," controlled by new state fields on `AppState`. Diff-over-time adds a new snapshot system writing to `.sentrux/snapshots/`, with per-file TDG grade, coverage %, and clippy count compared against current scan data.

The codebase already provides all primitive infrastructure this phase needs: `git2` for commit walking, `egui` painter + interact for custom bar rendering, `DiffWindow::CommitRange` for range-based git diff, `AnalysisSnapshot`/`FileAnalysisSnapshot` types in `pmat_types.rs` (pre-defined and serializable), and the established `*_requested → background thread → ScanMsg::*Ready` pipeline. There are no library gaps to fill.

The milestone concept is new data in the GSD world — the current `GsdPhaseReport` has no milestone grouping. The parser must be extended or a new `MilestoneInfo` type added to group phases under milestones (from ROADMAP.md or STATE.md metadata). Since the project currently has only one milestone (v1.0), the milestone bar is hidden per the CONTEXT.md decision, so milestone parsing can be minimal in Wave 1.

**Primary recommendation:** Build in four plans — (1) data types + snapshot writer, (2) timeline bar widget, (3) click-to-zoom filtering + reset, (4) diff-over-time delta display — following the established `*_requested → thread → ScanMsg` pattern.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Hierarchical Bar Layout
- **Replace** the Phase 4 proportional phase navigator entirely with a unified 3-tier stack
- Stack order (top to bottom): time ticks → milestones → phases → commits
- **Equal sizing within parent**: milestones get equal width; phases divide their milestone's width equally; commits divide their phase's width equally
- **Milestone bar hidden when only one milestone exists** — time ticks sit directly above phases bar
- Stack adapts: if milestone bar is hidden, time ticks → phases → commits
- Position: below toolbar/legend, same location as the current phase navigator (between toolbar and treemap canvas)

#### Click-to-Zoom Filtering
- Clicking a milestone/phase/commit filters the view to show **from selected point to present**
- **Treemap re-colors** to the filtered range: files changed in that range show intensity colors; unchanged files go gray
- Clicking a single commit shows diff from that commit to HEAD (cumulative changes since)
- **Reset mechanism: both** — a dedicated x/reset button appears when filtered, AND clicking the already-selected segment deselects it
- When clicking a phase, the commit bar shows only that phase's commits
- When clicking a milestone, the phase bar shows only that milestone's phases, commit bar shows those phases' commits

#### Commit Segment Labels
- Short hash only (e.g., "a1ac93b") displayed in each commit segment
- Hover tooltip shows: full commit message, author, date, file count, lines changed

#### Time Ticks
- **Dedicated thin row** above the milestone bar (or above phases when milestone bar is hidden)
- **Auto-scaling granularity**: adjusts based on visible range — hours when zoomed into a phase, days/weeks when viewing full project — keeps ~5-10 ticks visible
- **Compress gaps**: idle periods (weekends, no-commit stretches) don't take visual space; ticks only mark periods with activity; labels show actual dates/times

#### Diff-over-Time Analysis
- **Metrics**: TDG grade change, coverage % change, clippy warning count change
- **Aggregation levels**: file-level deltas in detail panel + directory-level rollup on directory nodes (avg TDG change, total coverage change)
- **Treemap visualization**: small arrow indicators on file/directory nodes — green arrow up = improved, red arrow down = regressed — subtle overlay on existing treemap colors
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

### Deferred Ideas (OUT OF SCOPE)
- Full unified time dial widget (all three dimensions in a single circular control) — v2
- Animated phase playback (watch the project evolve phase by phase) — v2
- PR/branch comparison view — v2
- Repo-wide summary delta widget (overall TDG/coverage trend) — could be added later as a small status bar widget
- Delta-based ColorMode (entire treemap colored by improvement/regression intensity) — v2
</user_constraints>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `egui` | 0.31 (workspace) | Custom widget painting — bars, tick marks, arrows | Already in workspace; painter + interact pattern proven in draw_gsd_phase_navigator() |
| `git2` | 0.20 | Commit metadata (hash, timestamp, message, author, file count) for commit bar | Already in Cargo.toml; walk_git_log_windowed() fully functional |
| `serde` + `serde_json` | 1 (workspace) | Snapshot JSON persistence | Already in Cargo.toml; AnalysisSnapshot already derives Serialize/Deserialize |
| `std::fs` | stdlib | Writing snapshots to `.sentrux/snapshots/` | No external dep needed |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `chrono` | Not present | Human-readable date/time formatting for time ticks | AVOID — use `std::time::SystemTime` + manual formatting; adding chrono for one feature is overhead |
| `crossbeam-channel` | 0.5 | Snapshot write message routing | Already used for all other background ops |

**No new dependencies required.** All necessary libraries are already in `sentrux-core/Cargo.toml`.

### Date/Time Formatting Without chrono

```rust
// Format epoch seconds into "Jan 15" or "03:45" without chrono
// Source: std::time primitives + manual arithmetic
fn format_epoch_date(epoch: i64) -> String {
    // Days since 1970-01-01
    let days = epoch / 86400;
    let secs_in_day = epoch % 86400;
    let hours = secs_in_day / 3600;
    let mins = (secs_in_day % 3600) / 60;
    // Simple month/day via Gregorian calendar arithmetic
    // or just show "d+N" / "h:mm" for compact display
    format!("{:02}:{:02}", hours, mins) // for intra-day ticks
}
```

**Installation:** No new packages needed.

## Architecture Patterns

### Recommended Module Structure

New files to add:
```
sentrux-core/src/
├── app/
│   ├── draw_panels.rs          # Replace draw_gsd_phase_navigator with draw_timeline_navigator
│   └── state.rs                # Add: selected_milestone_idx, selected_commit_hash, filter_active,
│                               #      timeline_filter_window, delta_report, snapshot_stored
├── core/
│   └── pmat_types.rs           # AnalysisSnapshot, FileAnalysisSnapshot already here (pre-built)
│                               # Add: TimelineDeltaReport, FileDeltaEntry, MilestoneInfo
└── analysis/
    └── snapshot_writer.rs      # New: write_analysis_snapshot(), load_nearest_snapshot()
```

```
.sentrux/
└── snapshots/
    ├── 1710000000.json         # One file per scan completion (Unix timestamp filename)
    └── 1710086400.json
```

### Pattern 1: Equal-Division Bar Layout

The Phase 4 proportional bar computed widths based on file count. The timeline bar uses **equal width** within a parent's allocation.

```rust
// Source: derived from existing draw_gsd_phase_navigator pattern in draw_panels.rs
fn equal_segment_widths(parent_width: f32, count: usize) -> Vec<f32> {
    if count == 0 { return vec![]; }
    let w = parent_width / count as f32;
    vec![w; count]
}

// For nested layout: phases divide their milestone's width
fn draw_phases_for_milestone(
    painter: &egui::Painter,
    milestone_rect: egui::Rect,
    phases: &[PhaseInfo],
) {
    let phase_w = milestone_rect.width() / phases.len() as f32;
    for (i, phase) in phases.iter().enumerate() {
        let x = milestone_rect.left() + i as f32 * phase_w;
        let seg = egui::Rect::from_min_size(
            egui::pos2(x, milestone_rect.top()),
            egui::vec2(phase_w, milestone_rect.height()),
        );
        // draw segment
    }
}
```

### Pattern 2: Multi-Row Stacked Bars in egui TopBottomPanel

The current legend + phase navigator already renders two rows inside a single `TopBottomPanel::top("toolbar")`. The timeline navigator adds more rows to the same panel. Each row uses `ui.allocate_exact_size()`.

```rust
// Source: draw_panels.rs:draw_toolbar_panel() — existing pattern
egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
    draw_toolbar(ui, &mut app.state);     // row 1
    draw_color_legend(ui, &app.state);    // row 2
    draw_timeline_navigator(ui, &mut app.state);  // rows 3-6 (ticks, milestones?, phases, commits)
});
```

Row heights (recommended, discretion area):
- Time ticks row: 12px thin strip
- Milestone row: 16px (hidden when single milestone)
- Phase row: 18px (same as existing)
- Commit row: 14px (smaller, dense)

### Pattern 3: Commit Data Collection for Navigator

The current `gsd_phase_adapter.rs` already walks commits for `detect_phase_commit_ranges()` up to 2000 commits. The commit bar needs per-commit metadata (short hash, message, author, timestamp, file count). A new `CommitSummary` type collects this during the same walk.

```rust
// Source: gsd_phase_adapter.rs:detect_phase_commit_ranges pattern + git_walker.rs
pub struct CommitSummary {
    pub sha: String,          // full OID hex
    pub short_sha: String,    // first 7 chars
    pub message: String,      // first line only
    pub author: String,
    pub epoch: i64,
    pub file_count: usize,
    pub phase_idx: Option<usize>,  // which phase this commit belongs to
}
```

This is collected during the same `parse_gsd_phases()` walk or a parallel walk, then attached to `GsdPhaseReport` or stored in a new field on `AppState`.

### Pattern 4: Snapshot Write on Scan Completion

The scan pipeline ends at `ScanMsg::Complete(snap, gen, reports)` handled in `update_loop.rs`. After storing reports on `AppState`, a background thread writes the snapshot. Follow the existing `coverage_requested → maybe_spawn_coverage_thread` pattern.

```rust
// In update_loop.rs ScanMsg::Complete handler — after storing reports:
app.state.snapshot_write_requested = true;

// In draw_panels.rs draw_toolbar_panel:
if app.state.snapshot_write_requested {
    app.state.snapshot_write_requested = false;
    maybe_spawn_snapshot_writer_thread(app);
}

// Snapshot writer thread sends ScanMsg::SnapshotStored(path) on completion
```

### Pattern 5: Delta Computation on Filter Selection

When user clicks a phase/milestone/commit, the UI:
1. Sets `state.timeline_selected = Some(TimelineSelection { epoch_start, ... })`
2. Sets `state.delta_requested = true`
3. Background thread: loads nearest snapshot from `.sentrux/snapshots/` ≤ epoch_start, diffs against current `pmat_report`/`coverage_report`/`clippy_report`
4. Sends `ScanMsg::DeltaReady(TimelineDeltaReport)`
5. `AppState.delta_report` stores result; treemap arrow overlays read from it

```rust
pub struct TimelineDeltaReport {
    /// Per-file metric deltas
    pub by_file: HashMap<String, FileDeltaEntry>,
    /// Epoch of the snapshot used as baseline
    pub baseline_epoch: i64,
}

pub struct FileDeltaEntry {
    /// Change in TDG grade rank: positive = improved, negative = regressed
    pub tdg_grade_delta: i32,
    /// Change in coverage % (can be None if no coverage data)
    pub coverage_pct_delta: Option<f64>,
    /// Change in clippy warning count (negative = fewer warnings = improved)
    pub clippy_count_delta: Option<i32>,
}
```

### Pattern 6: Arrow Overlay in draw_file_rect()

The delta arrows are drawn on top of the existing file rect, inside `renderer/rects.rs:draw_file_rect()`. The `RenderContext` already carries per-file data via `pmat_report`, `coverage_report`, etc. The `delta_report` is added to `RenderContext` the same way.

```rust
// Source: rects.rs draw_file_rect pattern — add after existing label drawing
if let Some(delta) = ctx.delta_report.and_then(|d| d.by_file.get(path)) {
    draw_delta_arrow(painter, screen_rect, delta);
}

fn draw_delta_arrow(painter: &egui::Painter, rect: egui::Rect, delta: &FileDeltaEntry) {
    // Show up arrow (green) if improved, down arrow (red) if regressed
    // Place in top-right corner of rect; small font (8px)
    let net = delta.tdg_grade_delta + delta.coverage_pct_delta.map(|d| d as i32).unwrap_or(0)
                                    - delta.clippy_count_delta.unwrap_or(0);
    if net > 0 {
        painter.text(rect.right_top() - egui::vec2(8.0, 0.0),
            egui::Align2::RIGHT_TOP, "\u{25B2}", egui::FontId::monospace(8.0),
            egui::Color32::from_rgb(80, 200, 80));
    } else if net < 0 {
        painter.text(rect.right_top() - egui::vec2(8.0, 0.0),
            egui::Align2::RIGHT_TOP, "\u{25BC}", egui::FontId::monospace(8.0),
            egui::Color32::from_rgb(220, 60, 60));
    }
}
```

### Pattern 7: Time Tick Auto-Scaling

Time ticks are positioned by mapping epoch time to x-coordinate. The commit bar already maps commits to horizontal segments; tick marks reference the same coordinate space.

```rust
// Map an epoch to an x position within the bar_rect
fn epoch_to_x(epoch: i64, epoch_min: i64, epoch_max: i64, rect: egui::Rect) -> f32 {
    if epoch_max == epoch_min { return rect.left(); }
    let t = (epoch - epoch_min) as f32 / (epoch_max - epoch_min) as f32;
    rect.left() + t * rect.width()
}

// Auto-scale: choose granularity so ~5-10 ticks fit
fn choose_tick_granularity_secs(span_secs: i64) -> i64 {
    match span_secs {
        0..=7200      => 900,      // < 2 hours: 15-min ticks
        0..=86400     => 3600,     // < 1 day: hourly ticks
        0..=604800    => 86400,    // < 1 week: daily ticks
        0..=2592000   => 604800,   // < 1 month: weekly ticks
        _             => 2592000,  // monthly ticks
    }
}
```

**Gap compression:** Since commits cluster around work sessions, the time axis maps commit timestamps to segment positions (equal segments by commit count), not wall-clock positions. Ticks show actual dates/times of commit timestamps, not proportional calendar time. This naturally compresses idle periods.

### Anti-Patterns to Avoid

- **Proportional width by file count for commits**: Phase 4 used file count to size phase segments. The timeline bar uses EQUAL width per item. Don't copy the proportional logic.
- **Blocking scan thread for snapshot writes**: Write snapshots on a separate background thread after scan completes, not inline in the scanner thread.
- **Loading all snapshots into memory**: Only load the snapshot nearest to the filter epoch. Keep snapshots as individual files by timestamp.
- **Storing CommitSummary inside PhaseInfo**: Commits are a new concept; add a separate `Vec<CommitSummary>` to `GsdPhaseReport` or a new `TimelineReport` type rather than mutating `PhaseInfo`.
- **Re-using `selected_phase_idx` for milestone selection**: Add distinct state fields for milestone, phase, and commit selection to avoid ambiguity.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Git commit enumeration | Custom shell-out to `git log` | `git2::Repository::revwalk()` | Already used in `gsd_phase_adapter.rs` and `git_walker.rs`; handles bare repos, OID resolution, merge skipping |
| JSON snapshot I/O | Custom format | `serde_json::to_string_pretty()` + `std::fs::write()` | `AnalysisSnapshot` already derives Serialize/Deserialize |
| Commit short hash | String slicing | `commit.id().to_string()[..7]` | Simple enough but document it — 7 chars is git convention |
| Time formatting | chrono crate | Manual epoch arithmetic | chrono not in Cargo.toml; simple "HH:MM" or "MMM DD" output is achievable with stdlib arithmetic |
| Segment interaction | Custom hit-testing | `ui.interact(rect, id, egui::Sense::click())` | Proven in draw_gsd_phase_navigator(); handles hover + click correctly |

**Key insight:** The commit bar and time ticks are cosmetically complex but algorithmically simple — the hard parts (git2 walking, egui painter, DiffWindow, AnalysisSnapshot types) are all already solved.

## Common Pitfalls

### Pitfall 1: Snapshot Writer Race with AppState Reports

**What goes wrong:** The snapshot writer thread reads `pmat_report`, `coverage_report`, `clippy_report` from AppState. But AppState is main-thread-only. If we pass a reference, the borrow checker rejects it. If we clone the data, we must clone before spawning.

**Why it happens:** Same as `maybe_spawn_coverage_thread` — AppState fields must be cloned out before thread spawn.

**How to avoid:** Clone the relevant report fields before `std::thread::Builder::new().spawn(move || {...})` — exactly as done in `maybe_spawn_git_diff_thread` cloning `app.state.git_diff_window`.

**Warning signs:** Compiler errors about `app` not being `Send`, or lifetime errors with `&app.state`.

### Pitfall 2: Missing Snapshot Directory

**What goes wrong:** First run writes to `.sentrux/snapshots/{ts}.json` but `.sentrux/snapshots/` doesn't exist.

**Why it happens:** `std::fs::write()` fails if parent directory is absent.

**How to avoid:** Call `std::fs::create_dir_all(".sentrux/snapshots")` before writing. Already established — `.sentrux/rules.toml` exists, so `.sentrux/` is present, but `snapshots/` subdir is new.

**Warning signs:** `ScanMsg::SnapshotStored` never fires; error message in eprintln.

### Pitfall 3: Snapshot Nearest-Lookup Fails if No Snapshots Exist

**What goes wrong:** User clicks a phase filter before any scan has completed a snapshot write. `delta_requested` fires, background thread finds zero snapshots, delta is empty.

**Why it happens:** New feature: first run has no baseline.

**How to avoid:** When snapshot directory is empty or no snapshot predates the filter epoch, send `ScanMsg::DeltaReady(TimelineDeltaReport { by_file: HashMap::new(), baseline_epoch: 0 })` — empty delta means no arrows, which is correct behavior. Do not block or error.

**Warning signs:** Thread spawns but never sends `DeltaReady`, leaving `delta_running = true` forever.

### Pitfall 4: Commit Bar Overflow (100+ Commits per Phase)

**What goes wrong:** A phase with 100 commits renders 100 tiny segments, each < 1px wide. Interaction breaks; labels are invisible.

**Why it happens:** Equal-width division of a 1200px bar by 100 = 12px per segment. Labels need ~40px minimum.

**How to avoid:** Apply minimum segment width = 7px (just enough to interact with, no label). For phases with > 50 commits where segments < 7px, show a "..." overflow indicator instead of individual segments. This is in Claude's discretion. Use the same `MIN_SEG_WIDTH` clamping pattern from `draw_gsd_phase_navigator()`.

**Warning signs:** Commit segments vanish or overlap each other.

### Pitfall 5: Milestone Bar Always Hidden for Single-Milestone Projects

**What goes wrong:** Forgetting the rule — developer sees milestone bar appear for a project with 1 milestone when it should be hidden.

**Why it happens:** CONTEXT.md decision is clear but easy to miss in implementation.

**How to avoid:** Guard: `if milestones.len() > 1 { draw_milestone_bar(...) }`. Document with a comment citing the CONTEXT.md decision.

### Pitfall 6: DiffWindow::CommitRange "from" Before "to"

**What goes wrong:** When clicking a commit C for "from C to HEAD," the `from` hash must be the SELECTED commit and `to` must be HEAD (or the phase's last commit). Getting them reversed causes an empty walk.

**Why it happens:** `walk_commit_range()` uses `revwalk.hide(from_oid)` — it excludes commits reachable from `from`. If from/to are swapped, all commits are hidden.

**How to avoid:** Convention: `CommitRange { from: oldest_sha, to: newest_sha }`. Add a comment in the click handler. Test with a known pair.

### Pitfall 7: serde(other) Must Remain Last in ColorMode

**What goes wrong:** ColorMode enum uses `#[serde(other)]` on `Monochrome` to handle unknown variants from old prefs. If a new variant is inserted AFTER `Monochrome`, deserialization of unknown values fails.

**Why it happens:** Phase 4 decision: "New ColorMode variants must be inserted BEFORE serde(other) Monochrome."

**How to avoid:** Phase 5 does NOT add a new ColorMode variant (the timeline navigator is a separate nav widget, not a color mode). No change to ColorMode enum needed. If delta arrows are added later as a ColorMode in v2, insert before Monochrome.

## Code Examples

### How Existing draw_gsd_phase_navigator is Structured (to be replaced)

The full function is in `draw_panels.rs` lines 340–523. Key pattern to preserve:

```rust
// Source: draw_panels.rs:draw_gsd_phase_navigator()
// 1. Guard on mode
if state.color_mode != ColorMode::GsdPhase { return; }
// 2. Check running/loading state
if state.gsd_phase_running { ui.label("Scanning..."); return; }
// 3. Clone report (to release borrow before mutations)
let report = match &state.gsd_phase_report { Some(r) => r.clone(), None => { return; } };
// 4. Allocate exact bar rect
let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(total_width, bar_height), egui::Sense::hover());
// 5. Use painter directly (no egui layout inside the rect)
let painter = ui.painter();
// 6. Interact after paint (collect new_* variables to avoid borrow conflict)
let seg_id = ui.id().with(("phase_seg", idx));
let seg_response = ui.interact(seg_rect, seg_id, egui::Sense::click());
// 7. Apply mutations after the loop
state.selected_phase_idx = new_selected;
```

The replacement `draw_timeline_navigator()` follows this exact pattern but draws 3-4 stacked rows.

### Snapshot Writer Thread Pattern

```rust
// Source: derived from maybe_spawn_coverage_thread in draw_panels.rs
fn maybe_spawn_snapshot_writer_thread(app: &mut SentruxApp) {
    let root = match app.state.root_path.clone() {
        Some(r) => r,
        None => return,
    };
    if app.state.snapshot_write_running { return; }
    // Clone report data before thread spawn (AppState is not Send)
    let pmat = app.state.pmat_report.clone();
    let coverage = app.state.coverage_report.clone();
    let clippy = app.state.clippy_report.clone();
    app.state.snapshot_write_running = true;
    let msg_tx = app.scan_msg_tx.clone();
    let _ = std::thread::Builder::new()
        .name("snapshot-writer".into())
        .spawn(move || {
            let result = crate::analysis::snapshot_writer::write_analysis_snapshot(
                &root, &pmat, &coverage, &clippy
            );
            let msg = match result {
                Ok(path) => ScanMsg::SnapshotStored(path),
                Err(e) => {
                    eprintln!("[snapshot] write failed: {}", e);
                    ScanMsg::SnapshotStored(String::new()) // non-fatal
                }
            };
            let _ = msg_tx.send(msg);
        });
}
```

### Snapshot Filename Convention

```rust
// Source: pattern consistent with GitDiffReport.computed_at field
fn snapshot_filename(root: &str, epoch: i64) -> std::path::PathBuf {
    std::path::Path::new(root)
        .join(".sentrux")
        .join("snapshots")
        .join(format!("{}.json", epoch))
}
```

### Nearest Snapshot Lookup

```rust
// Source: std::fs + serde_json pattern consistent with existing analysis adapters
fn load_nearest_snapshot(root: &str, target_epoch: i64) -> Option<AnalysisSnapshot> {
    let dir = std::path::Path::new(root).join(".sentrux").join("snapshots");
    let mut best: Option<(i64, AnalysisSnapshot)> = None;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(ts_str) = name.strip_suffix(".json") {
                if let Ok(ts) = ts_str.parse::<i64>() {
                    if ts <= target_epoch {
                        // Prefer the closest snapshot that predates target
                        if best.as_ref().map_or(true, |(best_ts, _)| ts > *best_ts) {
                            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                                if let Ok(snap) = serde_json::from_str::<AnalysisSnapshot>(&content) {
                                    best = Some((ts, snap));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    best.map(|(_, snap)| snap)
}
```

### Grade Rank for Delta Comparison

TDG grades are ordered strings. To compute a delta, map to an integer rank:

```rust
// Source: grade_to_t() in pmat_types.rs — reuse this or define parallel rank
fn grade_to_rank(grade: &str) -> i32 {
    match grade {
        "APLus" => 10, "A" => 9, "AMinus" => 8,
        "BPlus" => 7, "B" => 6, "BMinus" => 5,
        "CPlus" => 4, "C" => 3, "CMinus" => 2,
        "D" => 1, "F" => 0, _ => -1,
    }
}

fn grade_delta(old_grade: &str, new_grade: &str) -> i32 {
    grade_to_rank(new_grade) - grade_to_rank(old_grade)
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Proportional phase bar (file count) | Equal-width 3-tier timeline | Phase 5 | More spatially honest; milestones/phases/commits take equal visual weight |
| No baseline comparison | Snapshot-based diff-over-time | Phase 5 | Enables quality regression tracking |
| Phase click → immediate GitDiff mode switch | Phase click → timeline filter (from-point-to-present) | Phase 5 | More meaningful: "what changed since this phase started" |

**Deprecated/outdated:**
- `draw_gsd_phase_navigator()`: Replaced by `draw_timeline_navigator()`. The function is removed; its click/hover patterns are preserved.
- Proportional width logic (`widths: Vec<f32>` with MIN_SEG_WIDTH redistribution): Not used in timeline bar. Equal width is simpler and correct.

## Open Questions

1. **MilestoneInfo data source**
   - What we know: ROADMAP.md has no explicit milestone markers; STATE.md has `milestone: v1.0`. Current `GsdPhaseReport` has no milestone concept.
   - What's unclear: Should milestones be defined in ROADMAP.md (new markdown section), inferred from tags, or hardcoded from STATE.md `milestone` field?
   - Recommendation: For Phase 5 v1, use the `milestone` field from STATE.md `---` frontmatter as a single milestone group. All phases belong to it. Since `milestones.len() == 1`, the milestone bar is hidden per CONTEXT.md. Milestone parsing can be deferred to when the project has 2+ milestones.

2. **Snapshot Pruning Policy**
   - What we know: CONTEXT.md marks this as Claude's discretion.
   - What's unclear: How many snapshots to keep before pruning old ones.
   - Recommendation: Keep at most 50 snapshots (prune oldest when > 50). Write a `prune_snapshots()` call after successful write. This is a one-liner max-count check.

3. **Commit Bar Overflow for Dense Phases**
   - What we know: CONTEXT.md marks overflow handling as Claude's discretion. MIN_SEG_WIDTH = 7px is workable.
   - What's unclear: Whether to group commits by day or simply truncate with "...".
   - Recommendation: If a phase has more than `bar_width / 7.0` commits, show first N and then a "..." segment indicating overflow count. Clicking "..." does nothing (tooltip shows "N more commits"). This is simple and correct.

4. **Timeline Navigator Visibility Gating**
   - What we know: Phase 4 gated `draw_gsd_phase_navigator()` on `color_mode == ColorMode::GsdPhase`.
   - What's unclear: Should the timeline navigator always be visible, or only in GsdPhase mode?
   - Recommendation: Always visible when `gsd_phase_report.is_some()`, regardless of color mode. The timeline is a navigation tool, not a color overlay. This matches the CONTEXT.md description ("hierarchical navigation bar").

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `#[cfg(test)]` inline unit tests (cargo test) |
| Config file | none (workspace default) |
| Quick run command | `cargo test -p sentrux-core --lib 2>&1 \| tail -20` |
| Full suite command | `cargo test -p sentrux-core 2>&1 \| tail -30` |

### Phase Requirements → Test Map

Phase 5 requirements are TBD per REQUIREMENTS.md, but the implementation behaviors map to testable units:

| Behavior | Test Type | Automated Command | File Exists? |
|----------|-----------|-------------------|-------------|
| equal_segment_widths returns equal slices | unit | `cargo test -p sentrux-core timeline_widget -- --nocapture` | Wave 0 |
| grade_delta() computes correct rank difference | unit | `cargo test -p sentrux-core grade_delta` | Wave 0 |
| load_nearest_snapshot returns closest without exceeding target epoch | unit | `cargo test -p sentrux-core snapshot_writer` | Wave 0 |
| write_analysis_snapshot creates `.sentrux/snapshots/TS.json` | unit | `cargo test -p sentrux-core snapshot_writer` | Wave 0 |
| FileDeltaEntry computed correctly from two snapshots | unit | `cargo test -p sentrux-core timeline_delta` | Wave 0 |
| Milestone bar hidden when milestones.len() == 1 | unit (logic test) | `cargo test -p sentrux-core milestone_visibility` | Wave 0 |
| choose_tick_granularity_secs returns correct granularity for span | unit | `cargo test -p sentrux-core time_ticks` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p sentrux-core --lib 2>&1 | tail -20`
- **Per wave merge:** `cargo test -p sentrux-core 2>&1 | tail -30`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `sentrux-core/src/analysis/snapshot_writer.rs` — covers snapshot write + load tests
- [ ] `sentrux-core/src/core/pmat_types.rs` — extend with `TimelineDeltaReport`, `FileDeltaEntry`; tests inline
- [ ] Test helper: temporary snapshot directory in `/tmp` for snapshot load/write tests

## Sources

### Primary (HIGH confidence)
- Direct codebase inspection — `draw_panels.rs`, `state.rs`, `channels.rs`, `gsd_phase_adapter.rs`, `git_walker.rs`, `pmat_types.rs`, `rects.rs` (all read above)
- `sentrux-core/Cargo.toml` — confirmed git2 0.20, egui 0.31, serde_json 1 are available
- `pmat_types.rs` lines 432–457 — `AnalysisSnapshot` and `FileAnalysisSnapshot` already defined with Serialize/Deserialize

### Secondary (MEDIUM confidence)
- `draw_panels.rs:draw_gsd_phase_navigator()` patterns generalized to multi-row timeline (same API, extended)
- egui 0.31 `painter.text()` and `ui.interact()` signatures confirmed via existing working code

### Tertiary (LOW confidence)
- Auto-scaling time tick granularity boundaries — chosen to give ~5-10 ticks based on common project spans; exact boundaries are discretion

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crates already in Cargo.toml; no new dependencies needed
- Architecture: HIGH — all patterns directly derived from existing working code in the repo
- Pitfalls: HIGH — most pitfalls identified from existing decision log in STATE.md and known egui patterns
- Snapshot/delta system: HIGH — `AnalysisSnapshot` types pre-built; only writer/loader function needs authoring

**Research date:** 2026-03-15
**Valid until:** 2026-06-15 (egui 0.31 API stable; git2 0.20 stable; no fast-moving deps)
