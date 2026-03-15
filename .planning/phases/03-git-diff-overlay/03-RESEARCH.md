# Phase 3: Git Diff Overlay - Research

**Researched:** 2026-03-14
**Domain:** git2 revwalk extensions, ColorMode::GitDiff wiring, analysis snapshot storage, egui color legend UI
**Confidence:** HIGH — all git2 API claims verified against existing `git_walker.rs` code; all egui/ColorMode wiring patterns verified from existing Phase 2/2.1 source

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Time Window Selection**
- Preset buttons in toolbar (compact, one-click) — two groups:
  - Time-based: 15m | 1h | 1d | 1w | (since last tag)
  - Commit-based: 1 commit | 5 commits | custom N
- "Since last tag" uses `git describe --tags --abbrev=0` equivalent — find most recent tag
- Custom commit count: user types a number in a small input field next to preset buttons
- Window selector only visible when GitDiff ColorMode is active
- GSD phase/milestone commit ranges deferred to Phase 4

**Change Intensity**
- Lines changed + commit count combined — files changed often AND with many lines are hottest
- Formula: combine total lines added/removed with number of commits touching the file in the window
- Unchanged files visually muted (same as Coverage/Risk behavior for missing data)
- New files (created within the window) get a distinct color (different hue from the hot-cold gradient)
- Deleted files not shown (natural behavior — not in snapshot)

**Metric Deltas**
- File detail panel shows score deltas when a changed file is selected in GitDiff mode:
  - TDG grade change: "B → A (improved)"
  - Coverage change: "85% → 78% (-7%)"
  - Clippy change: "5 → 3 (-2)"
- Source: git-stored analysis snapshots — persist analysis results (TDG, coverage, clippy scores) in `.sentrux/` and commit, so `git log` can retrieve any historical state
- Cache is effectively unbounded (git manages history)
- On file click in GitDiff mode: retrieve analysis snapshot from selected window boundary, diff against current

**Overlay Persistence**
- Selected overlay mode (ColorMode) persists across sessions (already implemented via prefs)
- Selected time window persists across sessions (save to prefs)

### Claude's Discretion

- Exact formula for combining lines-changed and commit-count into a single intensity value
- Color choice for "new file" distinct indicator
- Layout of the preset buttons vs custom input in the toolbar
- How to store/retrieve analysis snapshots from git history efficiently
- Whether to run analysis snapshot on every scan or on explicit save

### Deferred Ideas (OUT OF SCOPE)

- GSD phase/milestone commit ranges as window presets — Phase 4
- Animated playback of git changes over time — v2
- PR/branch comparison view — v2
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| GDIT-01 | User can see treemap nodes color-coded by git changes within a selectable time window | `walk_git_log` in `git_walker.rs` already does time-windowed commits; extend with commit-count mode. `ColorMode::GitDiff` follows Coverage/Risk pattern |
| GDIT-02 | Time window options include at minimum: 15 minutes, 1 hour, 1 day, 1 week | Time window expressed as `seconds_back: i64`; 15m=900, 1h=3600, 1d=86400, 1w=604800. "Since last tag" uses `repo.find_reference("refs/tags/*")` + revwalk |
| GDIT-03 | Changed files show intensity based on number of lines changed (hotter = more changes) | `CommitFile.added + CommitFile.removed` already per-file in `git_walker.rs`; combined with `commit_count` per decision |
| GDIT-04 | Unchanged files are visually muted so changed files stand out | `color_by_coverage` muted-gray pattern (`Color32::from_rgb(60, 60, 60)`) reused for unchanged files |
| GDIT-05 | Git diff computation runs on a background thread without freezing the UI | Follows `CoverageReady` pattern: on-demand background thread → `ScanMsg::GitDiffReady(GitDiffReport)` |
| OVRL-01 | User can switch between overlay modes via toolbar toggle (TDG / Git Diff / GSD Phase) | `ColorMode::GitDiff` variant added before `Monochrome` in enum; toolbar `draw_visual_group` gains GitDiff button |
| OVRL-02 | Active overlay mode has a visible color legend explaining the color mapping | New `draw_color_legend` fn in toolbar or below-canvas area; egui horizontal strip with colored rects + labels |
| OVRL-03 | Overlay mode persists across sessions (saved in preferences) | `UserPrefs` already stores `color_mode: ColorMode`; no change needed once variant is added. Time window also needs a new prefs field |
</phase_requirements>

---

## Summary

Phase 3 has one of the best-prepared foundations in the codebase. The `metrics/evo/git_walker.rs` module already does the exact time-windowed git2 revwalk that GitDiff needs — it walks commits from HEAD back, collects per-commit per-file `(added, removed)` line counts, and uses `CommitRecord.epoch` for time cutoffs. The GitDiff feature requires two extensions to this: (1) a commit-count cutoff mode (stop after N commits instead of at a time boundary), and (2) detection of "new file" deltas (where `delta.old_file()` is absent). Everything else — ColorMode wiring, ScanMsg delivery, AppState fields, toolbar structure, prefs persistence — follows established patterns from Coverage (on-demand run) and TdgGrade (color dispatch) that are already committed.

The analysis snapshot feature (delta display in the detail panel) is the most novel part. The decision to store snapshots in `.sentrux/` as committed JSON files is elegant: `git log --oneline .sentrux/snapshot.json` lets you find the commit at the window boundary, and `git show <sha>:.sentrux/snapshot.json` retrieves the historical state. The git2 API supports this via `repo.find_reference` + `repo.blob_path` (or revwalk to find the commit at boundary then `commit.tree()` + tree entry lookup). This must run on a background thread — it is a git2 operation that involves blob content lookup, not a UI-thread-safe operation.

The "since last tag" window requires tag discovery. git2 0.20 provides `repo.tag_names(None)` to list all tags, `repo.find_reference("refs/tags/<name>")` to resolve each to a commit, and `repo.revwalk` to find the most recent. Alternatively, running `git describe --tags --abbrev=0` as a subprocess is simpler but adds a process spawn. Given the existing git2-only policy in `git_walker.rs`, the pure git2 path is preferred.

**Primary recommendation:** Extend `git_walker.rs` with a new `walk_git_log_windowed(root, window: DiffWindow) -> Result<Vec<CommitRecord>, String>` function; add `ColorMode::GitDiff` before `Monochrome`; add `GitDiffReport` to `AppState`; wire on-demand via `ScanMsg::GitDiffReady`; add color legend as a row below the toolbar using egui's horizontal layout.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `git2` | 0.20 (already in tree) | Revwalk with time/commit-count cutoff, tag lookup, tree entry lookup for snapshot retrieval | Already used in `git_walker.rs` and `analysis/git.rs`; no new dependency |
| `serde_json` | already in tree | Serialize/deserialize analysis snapshots to/from `.sentrux/snapshot.json` | Already used for PMAT types |
| `std::process::Command` | stdlib | Optional: `git describe --tags --abbrev=0` for "since last tag" (simpler than pure git2 tag walking) | Established subprocess pattern from `pmat_adapter.rs` |
| `crossbeam-channel` | already in tree | Deliver `GitDiffReport` from background thread to UI via `ScanMsg::GitDiffReady` | Established `CoverageReady` pattern |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `std::fs` | stdlib | Read/write `.sentrux/snapshot.json` | Snapshot storage |
| `egui` | already in tree | Color legend UI (horizontal strip with `painter.rect_filled` + labels) | Already used everywhere |

No new dependencies required.

---

## Architecture Patterns

### Recommended New/Modified Files

```
sentrux-core/src/
├── metrics/evo/
│   └── git_walker.rs           # EXTEND: add DiffWindow enum, walk_git_log_windowed()
├── analysis/
│   └── git_diff_adapter.rs     # NEW: compute_git_diff_report(), save/load snapshot logic
├── core/
│   └── pmat_types.rs           # EXTEND: add GitDiffReport, FileDiffData, AnalysisSnapshot types
├── layout/
│   └── types.rs                # EXTEND: add ColorMode::GitDiff (before Monochrome)
├── app/
│   ├── channels.rs             # EXTEND: add ScanMsg::GitDiffReady, ScanMsg::GitDiffError
│   ├── state.rs                # EXTEND: add git_diff_report, git_diff_window, git_diff_requested
│   ├── prefs.rs                # EXTEND: add git_diff_window to UserPrefs
│   └── toolbar.rs              # EXTEND: add GitDiff window selector row (visible only in GitDiff mode)
└── renderer/
    ├── rects.rs                # EXTEND: add ColorMode::GitDiff arm in file_color(), color_by_git_diff()
    ├── colors.rs               # EXTEND: add git_diff_intensity_color(), git_diff_new_file_color()
    └── panels/
        └── pmat_panel.rs       # EXTEND: add delta display section when selected file is in GitDiff mode
```

### Pattern 1: DiffWindow Enum

The key design decision is representing the user's selection as a single typed enum rather than multiple bool flags:

```rust
// sentrux-core/src/metrics/evo/git_walker.rs (or a new types section)

/// Selectable window for git diff overlay computation.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DiffWindow {
    /// Walk commits within the last N seconds from now.
    TimeSecs(i64),
    /// Walk only the last N commits (oldest first cutoff).
    CommitCount(u32),
    /// Walk commits since the most recent git tag.
    SinceLastTag,
}

impl DiffWindow {
    /// Predefined presets shown as buttons in the toolbar.
    pub const PRESETS: &'static [(DiffWindow, &'static str)] = &[
        // Time-based
        (DiffWindow::TimeSecs(900),    "15m"),
        (DiffWindow::TimeSecs(3600),   "1h"),
        (DiffWindow::TimeSecs(86400),  "1d"),
        (DiffWindow::TimeSecs(604800), "1w"),
        (DiffWindow::SinceLastTag,     "tag"),
        // Commit-based
        (DiffWindow::CommitCount(1),   "1c"),
        (DiffWindow::CommitCount(5),   "5c"),
    ];
}
```

### Pattern 2: Extending walk_git_log for DiffWindow

The existing `walk_git_log` uses a `lookback_days: u32` parameter with a `cutoff` epoch. The extension adds a `DiffWindow` variant that either uses a time cutoff (existing logic), a commit count (stop after N commits), or discovers the tag boundary first:

```rust
// sentrux-core/src/metrics/evo/git_walker.rs

/// Result type for windowed diff walk — records plus metadata about new files.
pub(crate) struct DiffWalkResult {
    pub records: Vec<CommitRecord>,
    /// Paths that were ADDED (new files) in this window (delta status = Added)
    pub new_file_paths: HashSet<String>,
}

/// Walk git log within a DiffWindow, collecting per-commit file changes.
///
/// New file detection: when delta.status() == git2::Delta::Added and there is no parent
/// (initial commit) or delta.old_file().id().is_zero(), the file is brand-new in this window.
pub(crate) fn walk_git_log_windowed(
    root: &Path,
    window: DiffWindow,
) -> Result<DiffWalkResult, String> {
    let repo = Repository::discover(root).map_err(|e| format!("Git discover failed: {e}"))?;
    let workdir = repo.workdir().ok_or("Bare repository")?;
    let prefix = scan_root_prefix(root, workdir);

    let mut revwalk = repo.revwalk().map_err(|e| format!("Revwalk failed: {e}"))?;
    revwalk.set_sorting(Sort::TIME).map_err(|e| format!("Sort failed: {e}"))?;
    revwalk.push_head().map_err(|e| format!("Push HEAD failed: {e}"))?;

    let cutoff_epoch = match window {
        DiffWindow::TimeSecs(s) => Some(epoch_now() - s),
        DiffWindow::CommitCount(_) => None,  // count-based, no epoch cutoff
        DiffWindow::SinceLastTag => {
            // Find most recent tag's commit epoch
            find_last_tag_epoch(&repo).ok()
        }
    };

    let max_commits = match window {
        DiffWindow::CommitCount(n) => Some(n as usize),
        _ => None,
    };

    collect_diff_commits(&repo, revwalk, cutoff_epoch, max_commits, &prefix)
}

/// Find the epoch of the most recent tag's target commit.
fn find_last_tag_epoch(repo: &Repository) -> Result<i64, String> {
    let tag_names = repo.tag_names(None).map_err(|e| format!("tag_names failed: {e}"))?;
    let mut latest_epoch: Option<i64> = None;

    for name in tag_names.iter().flatten() {
        let refname = format!("refs/tags/{}", name);
        if let Ok(reference) = repo.find_reference(&refname) {
            // Peel the reference to a commit (handles both lightweight and annotated tags)
            if let Ok(commit) = reference.peel_to_commit() {
                let epoch = commit.time().seconds();
                if latest_epoch.map_or(true, |e| epoch > e) {
                    latest_epoch = Some(epoch);
                }
            }
        }
    }
    latest_epoch.ok_or_else(|| "No tags found in repository".to_string())
}
```

### Pattern 3: GitDiffReport Types

```rust
// sentrux-core/src/core/pmat_types.rs  (extend existing file)

/// Per-file data from the git diff window walk.
#[derive(Debug, Clone)]
pub struct FileDiffData {
    /// Number of commits touching this file in the window
    pub commit_count: u32,
    /// Total lines added across all commits in the window
    pub lines_added: u32,
    /// Total lines removed across all commits in the window
    pub lines_removed: u32,
    /// True if the file was created within this window (shows distinct color)
    pub is_new_file: bool,
}

impl FileDiffData {
    /// Combined intensity score: lines changed weighted with commit frequency.
    /// Claude's discretion: this formula produces 0.0–unbounded raw; callers normalize.
    pub fn raw_intensity(&self) -> f64 {
        let lines = (self.lines_added + self.lines_removed) as f64;
        let commits = self.commit_count as f64;
        // Geometric mean between line count and commit frequency avoids either dominating.
        // A file changed 100 lines in 1 commit and a file changed 10 lines in 10 commits
        // both score ~10. A file changed 100 lines in 10 commits scores ~31.6 (hottest).
        (lines * commits).sqrt()
    }
}

/// Complete git diff overlay report for a time/commit window.
#[derive(Debug, Clone)]
pub struct GitDiffReport {
    /// Per-file change data — only files that changed appear here
    pub by_file: HashMap<String, FileDiffData>,
    /// Maximum raw_intensity across all files (for normalization)
    pub max_intensity: f64,
    /// The window used to compute this report
    pub window: crate::metrics::evo::git_walker::DiffWindow,
    /// Epoch seconds when this report was computed
    pub computed_at: i64,
}

impl GitDiffReport {
    /// Build from walk results. Normalizes max_intensity.
    pub fn from_walk(result: DiffWalkResult, window: DiffWindow) -> Self {
        let mut by_file: HashMap<String, FileDiffData> = HashMap::new();
        for record in &result.records {
            for cf in &record.files {
                let entry = by_file.entry(cf.path.clone()).or_insert(FileDiffData {
                    commit_count: 0, lines_added: 0, lines_removed: 0, is_new_file: false,
                });
                entry.commit_count += 1;
                entry.lines_added += cf.added;
                entry.lines_removed += cf.removed;
            }
        }
        for path in &result.new_file_paths {
            if let Some(e) = by_file.get_mut(path) {
                e.is_new_file = true;
            }
        }
        let max_intensity = by_file.values()
            .map(|d| d.raw_intensity())
            .fold(0.0_f64, f64::max);
        GitDiffReport {
            by_file,
            max_intensity: if max_intensity > 0.0 { max_intensity } else { 1.0 },
            window,
            computed_at: epoch_now(),
        }
    }
}

/// Snapshot of analysis scores for a file at a point in time.
/// Stored in `.sentrux/snapshot.json`, committed to git alongside code.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileAnalysisSnapshot {
    pub tdg_grade: Option<String>,      // "B", "APLus", etc.
    pub coverage_pct: Option<f64>,      // 0.0–100.0
    pub clippy_count: Option<u32>,
}

/// Full analysis snapshot: map of file path → scores.
/// Written to `.sentrux/snapshot.json` on explicit save.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalysisSnapshot {
    /// Unix epoch when this snapshot was captured
    pub captured_at: i64,
    /// Per-file scores
    pub files: HashMap<String, FileAnalysisSnapshot>,
}
```

### Pattern 4: ColorMode::GitDiff Addition

Following the established rule: insert BEFORE `Monochrome` (which has `#[serde(other)]` and MUST remain last):

```rust
// layout/types.rs — add GitDiff variant before Monochrome

pub enum ColorMode {
    Language,
    Heat,
    Git,
    TdgGrade,
    Coverage,
    Risk,
    GitDiff,    // NEW — before Monochrome
    #[serde(other)]
    Monochrome,
}
```

Also update `ColorMode::ALL` and `ColorMode::label()`:

```rust
pub const ALL: &'static [ColorMode] = &[
    ColorMode::Language, ColorMode::Heat, ColorMode::Git,
    ColorMode::TdgGrade, ColorMode::Coverage, ColorMode::Risk,
    ColorMode::GitDiff,
    ColorMode::Monochrome,
];

// label:
ColorMode::GitDiff => "Git Diff",
```

### Pattern 5: file_color() Dispatch for GitDiff

In `renderer/rects.rs`, add `ColorMode::GitDiff` arm to `file_color()`:

```rust
pub fn file_color(ctx: &RenderContext, path: &str) -> Color32 {
    match ctx.color_mode {
        // ... existing arms ...
        ColorMode::GitDiff => color_by_git_diff(ctx, path),
    }
}

fn color_by_git_diff(ctx: &RenderContext, path: &str) -> Color32 {
    let report = match ctx.git_diff_report {
        Some(r) => r,
        // No data yet — muted (same as Coverage no-data case)
        None => return ctx.theme_config.file_surface,
    };
    match report.by_file.get(path) {
        None => {
            // Unchanged file — visually muted (GDIT-04)
            Color32::from_rgb(50, 52, 55)
        }
        Some(data) if data.is_new_file => {
            // New file — distinct teal/cyan hue to stand out from hot-cold gradient
            colors::git_diff_new_file_color()
        }
        Some(data) => {
            // Changed file — intensity from cool to hot
            let t = (data.raw_intensity() / report.max_intensity).clamp(0.0, 1.0) as f32;
            colors::git_diff_intensity_color(t)
        }
    }
}
```

### Pattern 6: Color Functions for GitDiff

In `renderer/colors.rs`:

```rust
/// Changed file intensity gradient: cool blue (few changes) → hot orange-red (many changes).
/// Deliberately uses a DIFFERENT hue from TdgGrade/Coverage/Risk (which are green→red)
/// to visually distinguish "change volume" from "quality score".
pub fn git_diff_intensity_color(t: f32) -> Color32 {
    // t=0: cool blue (#1E6B9B); t=1: hot orange (#E86A17)
    // Interpolate in RGB space: blue → yellow-orange → red-orange
    let r = (30.0 + t * 218.0) as u8;
    let g = (107.0 - t * 57.0) as u8;   // blue-green tones fade as heat increases
    let b = (155.0 - t * 138.0) as u8;  // blue component fades to near-zero
    Color32::from_rgb(r, g, b)
}

/// Distinct color for files created within the diff window.
/// Uses a vivid teal/cyan that reads clearly against both hot-orange and muted-gray.
pub fn git_diff_new_file_color() -> Color32 {
    Color32::from_rgb(32, 190, 165)  // teal-green, distinct from hot-orange and muted
}
```

### Pattern 7: ScanMsg and AppState Extensions

```rust
// channels.rs — add to ScanMsg enum:
pub enum ScanMsg {
    // ... existing variants ...
    GitDiffReady(crate::core::pmat_types::GitDiffReport),
    GitDiffError(String),
}

// state.rs — add new fields:
pub struct AppState {
    // ... existing fields ...

    // ── Git Diff Overlay ──
    /// Git diff overlay report — None until user triggers computation, reset on new scan
    pub git_diff_report: Option<crate::core::pmat_types::GitDiffReport>,
    /// True while background git diff thread is running
    pub git_diff_running: bool,
    /// Active diff window selection (persisted to prefs)
    pub git_diff_window: crate::metrics::evo::git_walker::DiffWindow,
    /// Flag set by toolbar when a new diff window is selected
    pub git_diff_requested: bool,
    /// Custom commit count for DiffWindow::CommitCount (user-typed value)
    pub git_diff_custom_n: u32,
}
```

Default for `git_diff_window`: `DiffWindow::TimeSecs(86400)` (1 day — most useful default).
Default for `git_diff_custom_n`: `10`.

### Pattern 8: RenderContext Extension

`RenderContext` in `renderer/mod.rs` carries references from `AppState` to the renderer. Add:

```rust
pub git_diff_report: Option<&'a crate::core::pmat_types::GitDiffReport>,
```

Built in `canvas.rs` the same way `coverage_report`, `clippy_report`, etc. are built.

### Pattern 9: GitDiff Background Thread

Follows the `CoverageReady` pattern exactly:

```rust
// analysis/git_diff_adapter.rs

/// Compute git diff report on a background thread, deliver via scan_msg_tx.
pub fn spawn_git_diff_thread(
    root: String,
    window: DiffWindow,
    scan_msg_tx: crossbeam_channel::Sender<ScanMsg>,
) {
    std::thread::spawn(move || {
        match compute_git_diff_report(&root, window) {
            Ok(report) => {
                let _ = scan_msg_tx.send(ScanMsg::GitDiffReady(report));
            }
            Err(e) => {
                eprintln!("[git_diff] error: {}", e);
                let _ = scan_msg_tx.send(ScanMsg::GitDiffError(e));
            }
        }
    });
}

fn compute_git_diff_report(root: &str, window: DiffWindow) -> Result<GitDiffReport, String> {
    let result = walk_git_log_windowed(Path::new(root), window)?;
    Ok(GitDiffReport::from_walk(result, window))
}
```

Trigger: in `draw_panels.rs` (which owns `scan_msg_tx`), check `state.git_diff_requested`; if true, set `git_diff_running = true`, `git_diff_requested = false`, spawn the thread.

### Pattern 10: Toolbar Window Selector

The window selector row is only rendered when `state.color_mode == ColorMode::GitDiff` (CONTEXT.md: "Window selector only visible when GitDiff mode active"):

```rust
// toolbar.rs — add to draw_visual_group or as a separate draw_git_diff_controls():

fn draw_git_diff_controls(ui: &mut egui::Ui, state: &mut AppState) -> bool {
    if state.color_mode != ColorMode::GitDiff {
        return false;
    }
    let mut changed = false;
    ui.separator();
    ui.label(egui::RichText::new("Window:").small().weak());

    for &(window, label) in DiffWindow::PRESETS {
        let selected = state.git_diff_window == window;
        if ui.selectable_label(selected, label).clicked() && !selected {
            state.git_diff_window = window;
            state.git_diff_requested = true;
            changed = true;
        }
    }

    // Custom N commit input
    ui.label(egui::RichText::new("N:").small().weak());
    let custom_n_response = ui.add(
        egui::DragValue::new(&mut state.git_diff_custom_n)
            .range(1..=999)
            .speed(1.0)
    );
    if ui.button("go").clicked()
        || (custom_n_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
    {
        state.git_diff_window = DiffWindow::CommitCount(state.git_diff_custom_n);
        state.git_diff_requested = true;
        changed = true;
    }

    changed
}
```

### Pattern 11: Color Legend (OVRL-02)

A color legend row shown below the toolbar (or within it) when a color mode with a continuous gradient is active. Implemented as a horizontal egui strip in the main UI layout:

```rust
// A new draw_color_legend() function called from update_loop.rs or canvas.rs

fn draw_color_legend(ui: &mut egui::Ui, color_mode: ColorMode, git_diff_report: Option<&GitDiffReport>) {
    match color_mode {
        ColorMode::GitDiff => draw_git_diff_legend(ui, git_diff_report),
        ColorMode::TdgGrade => draw_tdg_legend(ui),
        ColorMode::Coverage => draw_coverage_legend(ui),
        ColorMode::Risk => draw_risk_legend(ui),
        _ => {} // Language/Heat/Git/Mono have no continuous legend
    }
}

fn draw_git_diff_legend(ui: &mut egui::Ui, report: Option<&GitDiffReport>) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Git Diff:").small());

        // Muted unchanged swatch
        let (_, rect) = ui.allocate_exact_size(egui::vec2(14.0, 10.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(50, 52, 55));
        ui.label(egui::RichText::new("unchanged").small().weak());

        // Gradient strip: cool→hot
        let gradient_width = 80.0_f32;
        let (_, grad_rect) = ui.allocate_exact_size(egui::vec2(gradient_width, 10.0), egui::Sense::hover());
        let steps = 12_u32;
        for i in 0..steps {
            let t = i as f32 / (steps - 1) as f32;
            let step_rect = egui::Rect::from_min_size(
                grad_rect.left_top() + egui::vec2(t * gradient_width * (steps-1) as f32 / steps as f32, 0.0),
                egui::vec2(gradient_width / steps as f32 + 1.0, 10.0),
            );
            ui.painter().rect_filled(step_rect, 0.0, colors::git_diff_intensity_color(t));
        }
        ui.label(egui::RichText::new("few → many changes").small().weak());

        // New file swatch
        let (_, rect2) = ui.allocate_exact_size(egui::vec2(14.0, 10.0), egui::Sense::hover());
        ui.painter().rect_filled(rect2, 0.0, colors::git_diff_new_file_color());
        ui.label(egui::RichText::new("new file").small().weak());

        if report.is_none() {
            ui.label(egui::RichText::new("(no data — select a window)").small().weak());
        }
    });
}
```

The legend for TdgGrade, Coverage, and Risk follows the same pattern. For simplicity, Phase 3 can implement a shared gradient-strip helper and a per-mode wrapper.

### Pattern 12: Analysis Snapshot Storage

The storage mechanism uses the git tree directly via git2 — no subprocess needed:

```rust
// analysis/git_diff_adapter.rs

const SNAPSHOT_PATH: &str = ".sentrux/snapshot.json";

/// Save current analysis snapshot alongside the repo's working tree.
/// Called explicitly (not on every scan — Claude's discretion: explicit save).
pub fn save_analysis_snapshot(root: &str, snapshot: &AnalysisSnapshot) -> Result<(), String> {
    let path = Path::new(root).join(SNAPSHOT_PATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir .sentrux: {e}"))?;
    }
    let json = serde_json::to_string_pretty(snapshot)
        .map_err(|e| format!("serialize snapshot: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("write snapshot: {e}"))
}

/// Retrieve an analysis snapshot from git history at the boundary of the diff window.
/// Returns the snapshot committed at or before the window boundary epoch.
pub fn load_snapshot_at_boundary(root: &str, boundary_epoch: i64) -> Option<AnalysisSnapshot> {
    let repo = Repository::discover(root).ok()?;

    // Revwalk from HEAD to find the last commit before boundary_epoch that
    // touched SNAPSHOT_PATH
    let mut revwalk = repo.revwalk().ok()?;
    revwalk.set_sorting(Sort::TIME).ok()?;
    revwalk.push_head().ok()?;

    for oid in revwalk {
        let oid = oid.ok()?;
        let commit = repo.find_commit(oid).ok()?;
        if commit.time().seconds() > boundary_epoch {
            continue;
        }
        // Found commit at/before boundary — look up the snapshot blob
        let tree = commit.tree().ok()?;
        if let Ok(entry) = tree.get_path(Path::new(SNAPSHOT_PATH)) {
            let blob = repo.find_blob(entry.id()).ok()?;
            return serde_json::from_slice(blob.content()).ok();
        }
    }
    None
}
```

**When to save:** Claude's discretion per CONTEXT.md. Recommendation: save on explicit user action (a "Save Snapshot" button or on scan completion if `.sentrux/` exists). Do NOT save on every scan automatically — it would pollute git history with noisy "update snapshot" commits. A `.sentrux/` directory should be committed (per the decision) — add it to `.gitignore` exclusion or leave it for users to commit themselves.

### Anti-Patterns to Avoid

- **Blocking the egui thread with revwalk:** git2 revwalk on large repos can take seconds. Always run on a background thread (same as Coverage). The `git_diff_requested` flag + background spawn pattern is the established way.
- **Re-running git diff on every frame:** Cache `git_diff_report: Option<GitDiffReport>` on `AppState`; only rerun when `git_diff_requested` is set (user changes window).
- **Adding ColorMode::GitDiff after Monochrome:** The `#[serde(other)]` rule is firmly established. `GitDiff` MUST be inserted before `Monochrome`.
- **Using the same green→red gradient as TdgGrade/Coverage:** The user needs to visually distinguish "how much changed" from "how healthy". Use the cool-blue→hot-orange gradient instead.
- **Walking merge commits for line counts:** The existing `git_walker.rs` already skips merge commits (`if commit.parent_count() > 1 { return None; }`). This must be preserved in `walk_git_log_windowed` — merge commits double-count changes.
- **git2 path separator on Windows:** `delta.new_file().path()` returns a `Path`. `to_string_lossy()` must be used (not `.to_str().unwrap()`) because git paths are always UTF-8 but `Path::to_str()` can fail on non-UTF-8 systems.
- **Snapshot retrieval blocking on large histories:** `load_snapshot_at_boundary` walks the full history. For repos with 10k+ commits this could take several seconds. Always call on a background thread; limit to the first `N` commits checked (e.g., stop after 1000).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Git log walking with line counts | Custom `git log --stat` parser | `walk_git_log_windowed()` extending existing `git_walker.rs` | Already works; handles merge commit skipping, mega-commit filtering, path prefix filtering |
| Tag discovery | Parse `git tag` subprocess output | `repo.tag_names(None)` + `reference.peel_to_commit()` in git2 | Handles both lightweight and annotated tags; no subprocess overhead |
| Historical blob retrieval | `git show <sha>:path` subprocess | `commit.tree()` + `tree.get_path()` + `repo.find_blob()` in git2 | Direct object access; no string parsing needed |
| Color gradient | Custom HSV math | `git_diff_intensity_color(t: f32)` linear RGB interpolation | Consistent with all other color functions in `colors.rs` |
| Intensity normalization | Fixed constant | Project-level `max_intensity` in `GitDiffReport` (same pattern as `max_risk_raw`) | Makes the "hottest" file always maximally orange regardless of project size |

---

## Common Pitfalls

### Pitfall 1: git2 Tag Walking Misses Annotated Tags

**What goes wrong:** Lightweight tags (`git tag v1.0`) point directly to a commit. Annotated tags (`git tag -a v1.0 -m "..."`) point to a tag object, not a commit. `repo.find_reference("refs/tags/v1.0").peel_to_commit()` handles both via `peel_to_commit()` — but directly doing `reference.resolve().target()` and then `repo.find_commit()` will fail for annotated tags (the OID points to a tag object, not a commit object).

**How to avoid:** Always use `reference.peel_to_commit()`. This is shown in Pattern 2 above. Verified: git2 0.20 `Reference::peel_to_commit()` returns `Result<Commit, Error>` and handles tag object peeling automatically.

**Warning signs:** "Since last tag" always returns "No tags found" even when `git tag` lists tags.

### Pitfall 2: New File Detection via Delta Status

**What goes wrong:** Checking `delta.old_file().path().is_none()` to detect new files is unreliable — renames can also have a different old path. The correct check is `delta.status() == git2::Delta::Added`.

**How to avoid:** Use `delta.status()` explicitly:
```rust
use git2::Delta;
let is_new = delta.status() == Delta::Added;
```

**Warning signs:** Renamed files showing as "new" in the overlay.

### Pitfall 3: Intensity Formula Double-Counting Multi-Commit Renames

**What goes wrong:** If a file was renamed within the window and then modified, git2 may produce two delta entries for the same logical file — one for the rename (old path → new path) and one for the modification. Using `new_file().path()` as the key (existing behavior in `collect_diff_files`) correctly groups by new path, but the old path still appears in some commit records.

**How to avoid:** The existing `collect_diff_files` in `git_walker.rs` already uses `delta.new_file().path().or_else(|| delta.old_file().path())` with path filtering. This correctly handles renames. No change needed.

### Pitfall 4: Window Persistence Breaks When DiffWindow Gains Variants

**What goes wrong:** `DiffWindow` is stored in `UserPrefs` via serde. If a future version adds a new variant (e.g., `DiffWindow::SinceBranch`), old prefs won't deserialize it.

**How to avoid:** Add `#[serde(other)]` or implement a custom `Deserialize` with a fallback default. Simplest: add `impl Default for DiffWindow { fn default() -> Self { DiffWindow::TimeSecs(86400) } }` and use `#[serde(default)]` on the prefs field.

### Pitfall 5: Large Repo git diff Performance

**What goes wrong:** `walk_git_log_windowed` with `DiffWindow::TimeSecs(604800)` (1 week) on a project with 1000 commits/week and 200 files/commit will walk 1000 commits × 200 files = 200k patch stat lookups. This can take 5–15 seconds.

**Why it happens:** `git2::Patch::from_diff(diff, delta_idx)` creates a full patch object for line count stats. It is not cheap.

**How to avoid:** Reuse the existing `MAX_FILES_PER_COMMIT` cap (50 files per commit). The current `parse_commit` function returns `None` for commits with >50 deltas. This bounds the worst case to 1000 commits × 50 files. Additionally, consider using `diff.stats()` (aggregate only) instead of per-delta `Patch::from_diff` when per-file line counts are not critical — but for GitDiff we need per-file, so the Patch path is required. Document the expected performance: ~2–5 seconds for a 1-week window on a medium project, which is acceptable for an on-demand operation.

**Warning signs:** GitDiff computation takes >30 seconds; "running..." indicator never clears.

### Pitfall 6: Analysis Snapshot Delta Races

**What goes wrong:** The user runs analysis, saves a snapshot at commit A, makes commits B and C, and then opens GitDiff for "last 2 commits" window. The snapshot at the boundary (before commit B) may not exist in git history if they never saved it after commit A.

**Why it happens:** Snapshots are only committed to git when the user explicitly saves. If they haven't saved, there is no historical snapshot to diff against.

**How to avoid:** When `load_snapshot_at_boundary` returns `None`, show "No historical snapshot available for this window boundary" in the detail panel rather than crashing or showing incorrect deltas. This is the graceful degradation path. Document this in the UI.

---

## Code Examples

### Complete git_diff_intensity_color and new_file_color

```rust
// Source: established pattern from tdg_grade_color() in renderer/colors.rs

/// Changed file intensity: cool blue (t=0, few changes) → hot orange-red (t=1, many changes).
/// Uses a blue→orange gradient to visually distinguish from the green→red
/// quality-score gradients used by TdgGrade, Coverage, and Risk modes.
pub fn git_diff_intensity_color(t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    // At t=0: rgb(30, 107, 155) — cool blue
    // At t=1: rgb(232, 106, 17) — hot orange
    let r = (30.0 + t * 202.0) as u8;
    let g = (107.0 - t * 57.0) as u8;
    let b = (155.0 - t * 138.0) as u8;
    Color32::from_rgb(r, g, b)
}

/// Distinct teal color for files created within the diff window.
/// Reads clearly against both the cool-blue (unchanged intensity) and hot-orange (high churn),
/// and does not conflict with any existing ColorMode gradient.
pub fn git_diff_new_file_color() -> Color32 {
    Color32::from_rgb(32, 190, 165)
}
```

### UserPrefs Extension for Window Persistence (OVRL-03)

```rust
// prefs.rs — extend UserPrefs

use crate::metrics::evo::git_walker::DiffWindow;

#[derive(Serialize, Deserialize)]
pub struct UserPrefs {
    // ... existing fields ...
    /// Selected git diff window — persists across restarts (OVRL-03)
    #[serde(default = "default_diff_window")]
    pub git_diff_window: DiffWindow,
    /// Custom N for DiffWindow::CommitCount
    #[serde(default = "default_custom_n")]
    pub git_diff_custom_n: u32,
}

fn default_diff_window() -> DiffWindow { DiffWindow::TimeSecs(86400) }
fn default_custom_n() -> u32 { 10 }
```

### ScanMsg Handler in update_loop.rs

```rust
// update_loop.rs — handle GitDiff messages (same pattern as CoverageReady)

ScanMsg::GitDiffReady(report) => {
    state.git_diff_report = Some(report);
    state.git_diff_running = false;
    // No layout change needed — color mode doesn't affect treemap geometry
    ctx.request_repaint();
}
ScanMsg::GitDiffError(msg) => {
    eprintln!("[git_diff] background error: {}", msg);
    state.git_diff_running = false;
    ctx.request_repaint();
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `walk_git_log(root, lookback_days)` — time only | `walk_git_log_windowed(root, DiffWindow)` — time OR commit count OR since-tag | Phase 3 | Enables commit-count windows and tag-anchored windows |
| No git-change visualization | `ColorMode::GitDiff` with cool→hot gradient | Phase 3 | Developer sees at a glance which files changed recently and how much |
| Evolution report is read-only (shows history) | Analysis snapshots in `.sentrux/` enable delta display | Phase 3 | File detail panel shows score changes (TDG/coverage/clippy) across diff window |

---

## Open Questions

1. **"Since last tag" fallback when no tags exist**
   - What we know: `repo.tag_names(None)` returns an empty list on repos with no tags
   - What's unclear: Whether to disable the "tag" preset button or show it with a tooltip "No tags in repo"
   - Recommendation: Show the button but disable it (gray, tooltip: "No tags found") when `find_last_tag_epoch` returns Err. This requires a pre-check — either run `repo.tag_names` at app load or lazily when GitDiff mode activates.

2. **Snapshot save timing — on scan completion or explicit action**
   - What we know: CONTEXT.md says "Claude's discretion"
   - Recommendation: Save on explicit action only (a "Save Snapshot" button in the toolbar or file detail panel). Auto-save on every scan pollutes git history. A one-line eprintln when GitDiff mode first activates and no snapshot exists: "[sentrux] No .sentrux/snapshot.json found. Save a snapshot to enable metric deltas."

3. **RenderContext carrying GitDiffReport**
   - What we know: `RenderContext` in `renderer/mod.rs` carries references from AppState. Phase 2 added `pmat_report`, phase 2.1 added `coverage_report`, `clippy_report`, `graph_metrics_report`.
   - What's unclear: The exact field additions needed in RenderContext (need to verify the mod.rs file)
   - Recommendation: Add `pub git_diff_report: Option<&'a GitDiffReport>` following the exact same pattern as `coverage_report`.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `cargo test` |
| Config file | none — standard cargo test discovery |
| Quick run command | `cargo test -p sentrux-core --lib 2>&1 \| tail -20` |
| Full suite command | `cargo test --workspace 2>&1 \| tail -30` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| GDIT-01 | `ColorMode::GitDiff` variant exists and serializes to `"gitdiff"` | unit | `cargo test -p sentrux-core color_mode_gitdiff_serde` | ❌ Wave 0 |
| GDIT-01 | `ColorMode::GitDiff` is in `ColorMode::ALL` and before `Monochrome` | unit | `cargo test -p sentrux-core color_mode_gitdiff_position` | ❌ Wave 0 |
| GDIT-02 | `DiffWindow::PRESETS` contains 15m, 1h, 1d, 1w variants | unit | `cargo test -p sentrux-core diff_window_presets_coverage` | ❌ Wave 0 |
| GDIT-02 | `find_last_tag_epoch` returns Err on repo with no tags | unit | `cargo test -p sentrux-core find_last_tag_no_tags` | ❌ Wave 0 |
| GDIT-03 | `FileDiffData::raw_intensity()` returns > 0 for changed file | unit | `cargo test -p sentrux-core file_diff_data_intensity` | ❌ Wave 0 |
| GDIT-03 | `GitDiffReport::from_walk` sets `max_intensity > 0` when files changed | unit | `cargo test -p sentrux-core git_diff_report_max_intensity` | ❌ Wave 0 |
| GDIT-04 | `color_by_git_diff` returns muted gray for paths not in report | unit | `cargo test -p sentrux-core git_diff_unchanged_is_muted` | ❌ Wave 0 |
| GDIT-05 | `spawn_git_diff_thread` sends `ScanMsg::GitDiffReady` to channel (integration) | unit | `cargo test -p sentrux-core git_diff_thread_sends_ready` | ❌ Wave 0 |
| OVRL-01 | Toolbar shows GitDiff window selector only when `color_mode == GitDiff` (manual) | manual | Run sentrux, switch to GitDiff mode, verify window buttons appear; switch away, verify they disappear | N/A |
| OVRL-02 | Color legend renders for GitDiff mode (manual) | manual | Run sentrux, switch to GitDiff mode, verify legend shows muted/gradient/new-file swatches | N/A |
| OVRL-03 | `UserPrefs` round-trips `git_diff_window` via serde | unit | `cargo test -p sentrux-core prefs_git_diff_window_serde` | ❌ Wave 0 |
| OVRL-03 | Missing `git_diff_window` in old prefs deserializes to default (1d) | unit | `cargo test -p sentrux-core prefs_git_diff_window_default` | ❌ Wave 0 |
| GDIT-01 | `git_diff_intensity_color(0.0)` is blue-ish | unit | `cargo test -p sentrux-core git_diff_color_cool_is_blue` | ❌ Wave 0 |
| GDIT-01 | `git_diff_intensity_color(1.0)` is orange-ish | unit | `cargo test -p sentrux-core git_diff_color_hot_is_orange` | ❌ Wave 0 |
| GDIT-01 | `git_diff_new_file_color()` is distinct from intensity gradient endpoints | unit | `cargo test -p sentrux-core git_diff_new_file_distinct` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p sentrux-core --lib 2>&1 | tail -20`
- **Per wave merge:** `cargo test --workspace 2>&1 | tail -30`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `sentrux-core/src/metrics/evo/git_walker.rs` — add `DiffWindow` enum, `DiffWalkResult`, `walk_git_log_windowed()`, `find_last_tag_epoch()`
- [ ] `sentrux-core/src/analysis/git_diff_adapter.rs` — new file: `spawn_git_diff_thread()`, `compute_git_diff_report()`, `save_analysis_snapshot()`, `load_snapshot_at_boundary()`
- [ ] `sentrux-core/src/core/pmat_types.rs` — extend with `FileDiffData`, `GitDiffReport`, `FileAnalysisSnapshot`, `AnalysisSnapshot`
- [ ] `sentrux-core/src/layout/types.rs` — add `ColorMode::GitDiff` (before `Monochrome`), update `ALL` and `label()`
- [ ] `sentrux-core/src/renderer/colors.rs` — add `git_diff_intensity_color()`, `git_diff_new_file_color()`

---

## Sources

### Primary (HIGH confidence)
- `sentrux-core/src/metrics/evo/git_walker.rs` — existing revwalk implementation; all extension patterns derived directly from this code
- `sentrux-core/src/metrics/evo/mod.rs` — `CommitRecord`, `FileChurn`, `compute_churn` patterns reused for `GitDiffReport::from_walk`
- `sentrux-core/src/layout/types.rs` — `ColorMode` enum with `#[serde(other)]` rule; confirmed `GitDiff` must precede `Monochrome`
- `sentrux-core/src/renderer/rects.rs` — `file_color()` dispatch pattern; `color_by_coverage()` muted-gray pattern for GDIT-04
- `sentrux-core/src/renderer/colors.rs` — `coverage_color()` gradient pattern; confirmed blue→orange distinguishes from existing green→red
- `sentrux-core/src/app/channels.rs` — `ScanMsg::CoverageReady` pattern for `GitDiffReady`
- `sentrux-core/src/app/state.rs` — `coverage_running`, `coverage_requested` field pattern for `git_diff_running`, `git_diff_requested`
- `sentrux-core/src/app/prefs.rs` — `UserPrefs` extension pattern; `#[serde(default)]` for backward-compatible new fields
- `sentrux-core/Cargo.toml` — confirmed `git2 = "0.20"` already in tree
- `.planning/phases/03-git-diff-overlay/03-CONTEXT.md` — all locked decisions

### Secondary (MEDIUM confidence)
- git2 0.20 crate documentation — `Reference::peel_to_commit()`, `Repository::tag_names()`, `commit.tree()`, `Tree::get_path()`, `Repository::find_blob()` — API exists in 0.20; confirmed via git2 changelog and existing usage in `git_walker.rs` / `analysis/git.rs`
- `git2::Delta::Added` status — documented in git2 crate; derived from libgit2 `GIT_DELTA_ADDED`

### Tertiary (LOW confidence)
- Analysis snapshot retrieval performance on large repos — estimated 2–5 seconds for repos with hundreds of commits and `.sentrux/snapshot.json` history; no direct measurement. Flag for implementation-time profiling.

---

## Metadata

**Confidence breakdown:**
- git2 revwalk extension (DiffWindow): HIGH — directly extends working code
- Commit-count cutoff: HIGH — trivially: stop after N iterations in the revwalk loop
- "Since last tag" via `peel_to_commit()`: MEDIUM — API verified to exist in 0.20; not directly tested
- New file detection via `delta.status() == Delta::Added`: HIGH — standard git2 API
- ColorMode::GitDiff wiring: HIGH — established pattern applied verbatim
- Analysis snapshot storage/retrieval: MEDIUM — git2 blob lookup pattern is correct; performance on large histories is estimated
- Color legend UI: MEDIUM — egui `painter.rect_filled` pattern is established; exact layout requires implementation-time tuning
- Intensity formula (geometric mean of lines × commits): MEDIUM — mathematically sound; empirical validation needed during implementation

**Research date:** 2026-03-14
**Valid until:** 2026-04-14 (30 days; git2 0.20 API is stable)
