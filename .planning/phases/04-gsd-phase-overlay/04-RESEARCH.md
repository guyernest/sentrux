# Phase 4: GSD Phase Overlay - Research

**Researched:** 2026-03-15
**Domain:** GSD plan file parsing, ROADMAP.md status detection, phase-to-file mapping, ColorMode::GsdPhase wiring, phase navigator panel
**Confidence:** HIGH — all patterns verified against existing sentrux source code; GSD file formats verified by reading actual .planning/ files in this repo

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Phase-to-File Mapping**
- Two sources: PLAN.md `files_modified` frontmatter + SUMMARY.md `key-files` sections
- Exact path matching after normalizing `./` prefixes — no fuzzy/basename matching
- When a file appears in multiple phases, color by the most recent phase that touched it
- Phase status (completed/in-progress/planned) read from ROADMAP.md checkbox status

**Color Scheme**
- Three states: completed = green-ish, in-progress = amber/yellow, planned = blue-ish
- Unassociated files = muted gray (same `NO_DATA_GRAY` pattern)
- Hover tooltip shows phase number, name, and goal

**Phase Commit Ranges**
- Commit message parsing first — look for GSD conventions like `feat(02-01):`, `docs(phase-3):`
- Time-based fallback — use completion dates from ROADMAP.md for commits without phase markers
- Phase boundaries defined by start/end commits
- Scrolling back one phase = show that phase's commit range

**Unified Time Navigation**
- Separate phase navigator panel (not toolbar presets) showing clickable phase timeline
- Commits, phases, and time are linked dimensions — scrolling by one auto-syncs the others
- Example: scrolling back 2 phases → commit count and time range adjust to match those phases' boundaries
- The phase navigator serves as the anchor for the GSD overlay, while the existing toolbar presets handle git diff time/commit windows
- When a phase is clicked in the navigator, the GitDiff window auto-adjusts to that phase's commit range

### Claude's Discretion

- Navigator panel layout and positioning (side panel, bottom panel, floating?)
- How to render the phase timeline (horizontal bar, vertical list, or other)
- Exact commit message parsing regex patterns
- How to handle phases with no commits (purely planning phases)
- Performance of parsing all PLAN.md/SUMMARY.md files on scan

### Deferred Ideas (OUT OF SCOPE)

- Full unified time dial widget (all three dimensions in a single control) — v2 milestone
- Phase-level metric aggregation (average TDG grade per phase, coverage trend per phase) — v2
- Animated phase playback (watch the project evolve phase by phase) — v2
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| GSDP-01 | User can see treemap nodes color-coded by which GSD phase touches them | ColorMode::GsdPhase variant; color_by_gsd_phase() in rects.rs; GsdPhaseReport::by_file lookup map |
| GSDP-02 | Past phases (completed) use a distinct color scheme from future phases (planned) | Three-state coloring: completed=green, in-progress=amber, planned=blue; PhaseStatus enum drives color dispatch |
| GSDP-03 | Phase information is read from `.planning/` directory files (ROADMAP.md, plan files) | parse_gsd_phases() reads ROADMAP.md for status, then all PLAN.md/SUMMARY.md for file lists; regex-based frontmatter and section parsing using existing `regex` crate |
| GSDP-04 | Files not associated with any phase are visually muted | NO_DATA_GRAY for unassociated files — identical pattern to GitDiff overlay for unchanged files |
| GSDP-05 | Hovering a colored node shows which phase and its goal | GsdPhaseReport::by_file stores (phase_number, phase_name, phase_goal, status) per path; status bar and/or egui tooltip renders it on hover |
</phase_requirements>

---

## Summary

Phase 4 adds `ColorMode::GsdPhase` to the existing overlay system. The GSD plan file formats are now fully understood from reading actual `.planning/` files in this repository: PLAN.md files have a YAML frontmatter block with `files_modified: [list]`; SUMMARY.md files have a `key-files:` YAML section listing `modified:` paths. ROADMAP.md uses `- [x]` vs `- [ ]` checkbox syntax for phase status, with `completed YYYY-MM-DD` dates in the text. The `regex` crate is already in `Cargo.toml`, so no new dependencies are needed for parsing.

The established background-thread data pipeline (flag → spawn → `ScanMsg` variant → `scanning.rs` handler → AppState field → `RenderContext` ref) is used exactly as git diff uses it. The `GsdPhaseReport` type goes in `pmat_types.rs` alongside `GitDiffReport`. Auto-trigger on mode switch fires `gsd_phase_requested = true`, then `draw_panels.rs:maybe_spawn_gsd_phase_thread()` spawns the parser on a background thread. The parse is fast (< 10ms for typical plan counts) so it could run during scan, but on-demand via mode switch follows the established pattern and avoids wasted work if the user never switches to this mode.

The phase navigator panel is the main UI innovation. It renders a horizontal timeline of phases (completed green bars → in-progress amber bar → planned blue bars) below the toolbar as a second row, visible only when `ColorMode::GsdPhase` is active. This mirrors how `draw_git_diff_controls()` is only shown in GitDiff mode. Clicking a phase fires `state.git_diff_window = DiffWindow::CommitRange(start_oid, end_oid)` — which requires adding a `CommitRange` variant to `DiffWindow`. The GitDiff overlay then shows the diff for that phase's commits.

**Primary recommendation:** Three-plan structure: Plan 01 types + parser + color functions; Plan 02 pipeline wiring + navigator panel; Plan 03 commit range integration + visual verification.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `regex` | already in tree (1.x) | Parse ROADMAP.md checkboxes, PLAN.md YAML frontmatter, SUMMARY.md key-files sections | Already in Cargo.toml; zero new dependency |
| `std::fs` | stdlib | Read `.planning/` directory tree | No external library needed for simple file reads |
| `std::path::Path` | stdlib | Path normalization (strip `./` prefix, canonicalize separators) | Avoids `./` vs relative path mismatch |
| `git2` | already in tree (0.20) | Walk commits for phase boundary detection | Same crate used in `git_walker.rs` |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `crossbeam-channel` | already in tree | Deliver `GsdPhaseReport` from background thread | Same pattern as GitDiffReady/CoverageReady |
| `std::collections::HashMap` | stdlib | `GsdPhaseReport::by_file` path → phase info lookup | O(1) lookup; same as all other reports |

No new dependencies required.

**Installation:** None needed.

---

## Verified GSD File Formats

### ROADMAP.md Phase Status Format

Verified by reading `.planning/ROADMAP.md` in this repo:

```markdown
- [x] **Phase 1: Cleanup** - Remove MCP ... (completed 2026-03-15)
- [x] **Phase 2: PMAT Integration** - Integrate PMAT ... (completed 2026-03-15)
- [ ] **Phase 4: GSD Phase Overlay** - Color-code treemap ...
```

**Status detection rules:**
- `- [x]` prefix → completed
- `- [ ]` prefix → planned OR in-progress
- Detect in-progress: scan all phases; the FIRST `- [ ]` phase in numeric order is in-progress (current); subsequent `- [ ]` phases are planned

**Completion date extraction:** `(completed YYYY-MM-DD)` at end of the line — regex `\(completed (\d{4}-\d{2}-\d{2})\)`.

**Phase name and number extraction:**
```
- [x] **Phase N: Name** - Goal text
```
Regex: `\*\*Phase\s+([\d.]+):\s+([^*]+)\*\*\s+-\s+(.+?)(?:\s+\(completed|$)`

**Phase detail blocks** (further down in ROADMAP.md):
```markdown
### Phase 1: Cleanup
**Goal**: The codebase contains only the capabilities it will carry forward...
**Plans:** 2/2 plans complete
Plans:
- [x] 01-01-PLAN.md — Remove MCP server...
```

### PLAN.md frontmatter format

Verified by reading `01-01-PLAN.md`, `03-03-PLAN.md`:

```yaml
---
phase: 01-cleanup
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - sentrux-bin/src/main.rs
  - sentrux-core/src/app/mod.rs
  - sentrux-core/src/app/mcp_server/
  - sentrux-core/Cargo.toml
autonomous: true
requirements:
  - CLEN-01
---
```

**Key facts:**
- Delimited by `---` triple-dash fences
- `files_modified` is a YAML sequence of strings
- Entries may end with `/` to indicate a whole directory (e.g., `sentrux-core/src/app/mcp_server/`)
- No `./` prefix (already clean)
- Some entries may be directory paths — these match against any file whose path starts with that prefix

### SUMMARY.md format

Verified by reading `01-01-SUMMARY.md`, `03-03-SUMMARY.md`:

```yaml
---
phase: 01-cleanup
plan: 01
...
key-files:
  created: []
  modified:
    - sentrux-bin/src/main.rs
    - sentrux-core/src/app/mod.rs
    - sentrux-core/src/analysis/lang_registry.rs
    - sentrux-core/Cargo.toml
---
```

**Key facts:**
- Same `---` frontmatter block
- `key-files:` section has `created:` and `modified:` sub-lists
- Paths are scan-root-relative, no `./` prefix
- SUMMARY.md covers files actually touched (may differ from PLAN.md `files_modified` which lists planned files)

### GSD Commit Message Convention

Verified by running `git log --oneline` in this repo:

```
68cc5a3 docs(04): capture phase context
0d99a61 docs(phase-3): complete phase execution
91e3c5a docs(03-03): complete toolbar UI and color legend plan
77ec1fe refactor: simplify Phase 3 git diff overlay code
a3dd567 feat(03-03): toolbar window selector, auto-trigger, and color legend
797aeb9 feat(03-02): UserPrefs persistence for git diff window
be61649 feat(03-02): wire git diff pipeline through AppState
6675994 docs(03-01): complete git-diff-overlay foundation plan
2db5d13 feat(03-01): git walker extension and diff adapter
77f3770 feat(03-01): types, ColorMode::GitDiff variant, and color functions
ddf72b7 docs(03-git-diff-overlay): create phase plan
```

**Convention detected:**
- `type(phase-plan):` — e.g., `feat(03-01):`, `docs(03-02):`
- `type(phase):` — e.g., `docs(04):`, `docs(phase-3):`
- Phase number appears as `NN` or `NN-MM` in the scope
- Regex to extract phase: `\((\d+)(?:-(\d+))?[^)]*\)` or `\(phase-(\d+)\)` or `\((\d+)-[a-z]`

---

## Architecture Patterns

### Recommended New File Structure
```
sentrux-core/src/
├── core/
│   └── pmat_types.rs         # EXTEND: add GsdPhaseReport, PhaseInfo, PhaseStatus
├── analysis/
│   └── gsd_phase_adapter.rs  # NEW: parse_gsd_phases(), find_planning_dir()
├── app/
│   ├── state.rs              # EXTEND: add gsd_phase_report, gsd_phase_requested, gsd_phase_running fields
│   ├── channels.rs           # EXTEND: add ScanMsg::GsdPhaseReady, ScanMsg::GsdPhaseError
│   ├── draw_panels.rs        # EXTEND: add maybe_spawn_gsd_phase_thread(), draw_gsd_navigator_section()
│   └── toolbar.rs            # EXTEND: auto-trigger on GsdPhase mode switch
└── renderer/
    ├── rects.rs              # EXTEND: add color_by_gsd_phase() arm in file_color()
    ├── colors.rs             # EXTEND: add gsd_phase_color() for three states
    └── mod.rs                # EXTEND: RenderContext gains gsd_phase_report field
```

Also extend `layout/types.rs` ColorMode enum.

### Pattern 1: GsdPhaseReport Data Types

```rust
// sentrux-core/src/core/pmat_types.rs — add after GitDiffReport

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseStatus {
    Completed,
    InProgress,
    Planned,
}

#[derive(Debug, Clone)]
pub struct PhaseInfo {
    /// Phase number string as it appears in ROADMAP.md (e.g., "1", "2.1", "4")
    pub number: String,
    /// Phase name (e.g., "Cleanup", "PMAT Integration")
    pub name: String,
    /// Phase goal one-liner
    pub goal: String,
    pub status: PhaseStatus,
    /// Completion date if status == Completed, parsed from ROADMAP.md
    pub completed_date: Option<String>,
    /// All file paths associated with this phase (scan-root-relative, normalized)
    pub files: Vec<String>,
}

/// Per-file GSD phase assignment — the report stored on AppState.
#[derive(Debug, Clone)]
pub struct GsdPhaseReport {
    /// Ordered list of phases (numeric order: 1, 2, 2.1, 3, 4)
    pub phases: Vec<PhaseInfo>,
    /// scan-root-relative path → index into phases vec (most recent phase wins)
    pub by_file: HashMap<String, usize>,
}

impl GsdPhaseReport {
    /// Total phase count (for navigator panel).
    pub fn phase_count(&self) -> usize { self.phases.len() }

    /// Get phase info for a file path. Returns None if file is unassociated.
    pub fn phase_for_file(&self, path: &str) -> Option<&PhaseInfo> {
        self.by_file.get(path).map(|&idx| &self.phases[idx])
    }
}
```

### Pattern 2: parse_gsd_phases() Adapter

```rust
// sentrux-core/src/analysis/gsd_phase_adapter.rs

use std::collections::HashMap;
use std::path::Path;

pub fn parse_gsd_phases(scan_root: &str) -> Option<GsdPhaseReport> {
    // 1. Find .planning/ directory relative to scan root
    let planning_dir = find_planning_dir(scan_root)?;

    // 2. Parse ROADMAP.md for phase list and statuses
    let roadmap_path = planning_dir.join("ROADMAP.md");
    let roadmap = std::fs::read_to_string(&roadmap_path).ok()?;
    let mut phases = parse_roadmap_phases(&roadmap);

    if phases.is_empty() {
        return None;
    }

    // 3. For each phase, collect file lists from PLAN.md and SUMMARY.md
    let phases_dir = planning_dir.join("phases");
    if phases_dir.exists() {
        for phase in &mut phases {
            collect_phase_files(&phases_dir, phase, scan_root);
        }
    }

    // 4. Build by_file lookup (most recent phase wins — iterate in numeric order)
    let mut by_file: HashMap<String, usize> = HashMap::new();
    for (idx, phase) in phases.iter().enumerate() {
        for path in &phase.files {
            // Most recent phase wins — overwrite earlier assignments
            by_file.insert(path.clone(), idx);
        }
    }

    Some(GsdPhaseReport { phases, by_file })
}

/// Find .planning/ directory: check scan_root/.planning/ first, then walk up.
fn find_planning_dir(scan_root: &str) -> Option<std::path::PathBuf> {
    let direct = Path::new(scan_root).join(".planning");
    if direct.is_dir() { return Some(direct); }

    // Walk up to 3 parent directories
    let mut current = Path::new(scan_root).parent();
    for _ in 0..3 {
        if let Some(parent) = current {
            let candidate = parent.join(".planning");
            if candidate.is_dir() { return Some(candidate); }
            current = parent.parent();
        }
    }
    None
}
```

### Pattern 3: ROADMAP.md Phase Parsing

```rust
fn parse_roadmap_phases(content: &str) -> Vec<PhaseInfo> {
    use regex::Regex;

    // Match: - [x] **Phase 2.1: Name** - Goal text (completed 2026-03-15)
    // Or:    - [ ] **Phase 4: GSD Phase Overlay** - Goal text
    let phase_re = Regex::new(
        r"^- \[( |x)\] \*\*Phase\s+([\d.]+):\s*([^*]+?)\*\*\s*-\s*(.+?)(?:\s*\(completed (\d{4}-\d{2}-\d{2})\))?\s*$"
    ).unwrap();

    let mut phases = Vec::new();
    let mut first_incomplete_seen = false;

    for line in content.lines() {
        let line = line.trim();
        if let Some(caps) = phase_re.captures(line) {
            let checked = &caps[1] == "x";
            let number = caps[2].to_string();
            let name = caps[3].trim().to_string();
            let goal = caps[4].trim().to_string();
            let completed_date = caps.get(5).map(|m| m.as_str().to_string());

            let status = if checked {
                PhaseStatus::Completed
            } else if !first_incomplete_seen {
                first_incomplete_seen = true;
                PhaseStatus::InProgress
            } else {
                PhaseStatus::Planned
            };

            phases.push(PhaseInfo {
                number, name, goal, status, completed_date, files: Vec::new(),
            });
        }
    }
    phases
}
```

### Pattern 4: PLAN.md and SUMMARY.md File Collection

```rust
fn collect_phase_files(phases_dir: &Path, phase: &mut PhaseInfo, scan_root: &str) {
    // Phase directory slug: e.g., "01-cleanup", "02.1-rust-deep-analysis"
    // Find directories matching the phase number prefix
    let entries = match std::fs::read_dir(phases_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let dir_name = entry.file_name();
        let dir_str = dir_name.to_string_lossy();

        // Match phase number: dir starts with "01-", "02.1-", etc.
        // Normalize: phase "2.1" → dir prefix "02.1-" or "2.1-"
        if !dir_str.starts_with(&phase.number)
            && !dir_str.starts_with(&zero_pad_phase(&phase.number)) {
            continue;
        }

        let phase_dir = entry.path();
        // Read all *-PLAN.md files
        for plan_file in glob_plan_files(&phase_dir, "PLAN.md") {
            if let Some(files) = extract_files_modified(&plan_file) {
                for f in files {
                    let normalized = normalize_path(&f);
                    // Directory entries (ending with /) expand later at render time
                    if !phase.files.contains(&normalized) {
                        phase.files.push(normalized);
                    }
                }
            }
        }
        // Read all *-SUMMARY.md files
        for summary_file in glob_plan_files(&phase_dir, "SUMMARY.md") {
            if let Some(files) = extract_key_files(&summary_file) {
                for f in files {
                    let normalized = normalize_path(&f);
                    if !phase.files.contains(&normalized) {
                        phase.files.push(normalized);
                    }
                }
            }
        }
    }
}

/// Extract files_modified list from PLAN.md YAML frontmatter.
fn extract_files_modified(plan_path: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(plan_path).ok()?;
    parse_yaml_string_list(&content, "files_modified")
}

/// Extract key-files.created + key-files.modified from SUMMARY.md frontmatter.
fn extract_key_files(summary_path: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(summary_path).ok()?;
    let mut files = Vec::new();
    if let Some(mut created) = parse_yaml_nested_list(&content, "key-files", "created") {
        files.append(&mut created);
    }
    if let Some(mut modified) = parse_yaml_nested_list(&content, "key-files", "modified") {
        files.append(&mut modified);
    }
    if files.is_empty() { None } else { Some(files) }
}

/// Normalize a path entry: strip "./" prefix, strip trailing "/".
fn normalize_path(path: &str) -> String {
    path.trim_start_matches("./").trim_end_matches('/').to_string()
}
```

### Pattern 5: YAML Frontmatter Parsing (no serde_yaml needed)

The frontmatter is simple enough for manual regex parsing. The `toml` crate is already available but YAML is not. Use manual string parsing:

```rust
/// Parse a YAML sequence under a top-level key from between --- fences.
/// Handles:
///   key:
///     - value1
///     - value2
fn parse_yaml_string_list(content: &str, key: &str) -> Option<Vec<String>> {
    // Extract frontmatter block (between first --- and second ---)
    let fm = extract_frontmatter(content)?;

    let key_prefix = format!("{}:", key);
    let mut in_list = false;
    let mut results = Vec::new();

    for line in fm.lines() {
        if line.trim_start() == key_prefix || line.trim() == key_prefix {
            in_list = true;
            continue;
        }
        if in_list {
            let trimmed = line.trim();
            if trimmed.starts_with("- ") {
                let val = trimmed[2..].trim().trim_matches('"').trim_matches('\'');
                if !val.is_empty() {
                    results.push(val.to_string());
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                // Non-list, non-empty line ends the sequence
                in_list = false;
            }
        }
    }

    if results.is_empty() { None } else { Some(results) }
}

fn extract_frontmatter(content: &str) -> Option<&str> {
    let after_first = content.strip_prefix("---\n")?;
    let end = after_first.find("\n---")?;
    Some(&after_first[..end])
}
```

### Pattern 6: color_by_gsd_phase() in rects.rs

```rust
// sentrux-core/src/renderer/rects.rs — add to file_color() match arm
ColorMode::GsdPhase => color_by_gsd_phase(ctx, path),

fn color_by_gsd_phase(ctx: &RenderContext, path: &str) -> Color32 {
    let report = match ctx.gsd_phase_report {
        Some(r) => r,
        None => return ctx.theme_config.file_surface, // no report: monochrome
    };

    // Try exact path match
    // Also try directory prefix match for entries like "sentrux-core/src/app/mcp_server/"
    let phase_idx = report.by_file.get(path).copied()
        .or_else(|| find_directory_match(&report.by_file, path));

    match phase_idx {
        None => colors::NO_DATA_GRAY, // unassociated (GSDP-04)
        Some(idx) => {
            let phase = &report.phases[idx];
            colors::gsd_phase_color(phase.status)
        }
    }
}

fn find_directory_match(by_file: &HashMap<String, usize>, path: &str) -> Option<usize> {
    // Check if any key in by_file is a directory prefix of path
    // Keys like "sentrux-core/src/app/" match "sentrux-core/src/app/state.rs"
    for (key, &idx) in by_file {
        if path.starts_with(key.as_str()) {
            return Some(idx);
        }
    }
    None
}
```

### Pattern 7: gsd_phase_color() in colors.rs

```rust
// sentrux-core/src/renderer/colors.rs

/// Color a treemap node by its GSD phase status.
/// Completed: muted green (phase is done — past)
/// InProgress: amber/yellow (current active work)
/// Planned: steel blue (future — planned but not yet started)
pub fn gsd_phase_color(status: PhaseStatus) -> Color32 {
    match status {
        PhaseStatus::Completed  => Color32::from_rgb(76,  153,  76),  // muted green
        PhaseStatus::InProgress => Color32::from_rgb(220, 165,  32),  // amber/goldenrod
        PhaseStatus::Planned    => Color32::from_rgb(70,  130, 180),  // steel blue
    }
}
```

### Pattern 8: Phase Navigator Panel (draw_panels.rs)

Only shown when `ColorMode::GsdPhase` is active. Follows `draw_git_diff_controls()` pattern.

```rust
// sentrux-core/src/app/draw_panels.rs

/// Phase navigator row — only shown when ColorMode::GsdPhase is active.
/// Renders a horizontal timeline of phases as colored clickable segments.
/// Clicking a phase sets git_diff_window to that phase's commit range.
pub(crate) fn draw_gsd_phase_navigator(ui: &mut egui::Ui, state: &mut AppState) {
    if state.color_mode != ColorMode::GsdPhase {
        return;
    }
    let report = match &state.gsd_phase_report {
        Some(r) => r.clone(),
        None => {
            ui.label(egui::RichText::new("No .planning/ directory found").small().weak());
            return;
        }
    };

    ui.separator();
    ui.add_space(2.0);
    ui.label(egui::RichText::new("Phases:").small().weak());

    for (idx, phase) in report.phases.iter().enumerate() {
        let color = colors::gsd_phase_color(phase.status);
        let label = format!("{} {}", phase.number, phase.name);

        // Colored button for each phase
        let btn = egui::Button::new(
            egui::RichText::new(&label).small().color(egui::Color32::WHITE)
        ).fill(color);

        let resp = ui.add(btn)
            .on_hover_text(format!("Phase {}: {}\nGoal: {}\nStatus: {:?}",
                phase.number, phase.name, phase.goal, phase.status));

        if resp.clicked() {
            state.selected_phase_idx = Some(idx);
            // If commit range available, trigger GitDiff for this phase
            if let Some((start, end)) = &phase.commit_range {
                state.git_diff_window = DiffWindow::CommitRange {
                    from: start.clone(),
                    to: end.clone(),
                };
                state.git_diff_requested = true;
            }
        }
    }
}
```

### Pattern 9: ScanMsg and AppState Extensions

```rust
// channels.rs — add to ScanMsg enum
ScanMsg::GsdPhaseReady(crate::core::pmat_types::GsdPhaseReport),
ScanMsg::GsdPhaseError(String),

// state.rs — add fields in "Analysis reports" section
pub gsd_phase_report: Option<GsdPhaseReport>,
pub gsd_phase_running: bool,
pub gsd_phase_requested: bool,
pub selected_phase_idx: Option<usize>,   // for navigator highlight

// renderer/mod.rs — add to RenderContext
pub gsd_phase_report: Option<&'a GsdPhaseReport>,
```

### Pattern 10: DiffWindow::CommitRange Extension

The navigator's "click to see phase diff" requires a new `DiffWindow` variant:

```rust
// sentrux-core/src/metrics/evo/git_walker.rs
pub enum DiffWindow {
    TimeSecs(i64),
    CommitCount(u32),
    SinceLastTag,
    // NEW: explicit commit OID range for phase diff navigation
    CommitRange { from: String, to: String },  // SHA hex strings
}
```

All `DiffWindow` match arms in `git_walker.rs` must handle this new variant.

### Pattern 11: Phase Commit Range Detection

```rust
// sentrux-core/src/analysis/gsd_phase_adapter.rs

/// Detect commit range for each phase by parsing GSD-convention commit messages.
/// Convention: feat(03-01):, docs(phase-3):, feat(02):, etc.
pub fn detect_phase_commit_ranges(scan_root: &str, phases: &mut Vec<PhaseInfo>) {
    let repo = match git2::Repository::discover(scan_root) {
        Ok(r) => r,
        Err(_) => return,
    };

    let phase_scope_re = Regex::new(
        r"\((\d+)(?:[.-](\d+))?(?:-[a-z][^)]*)?[\s):]"
    ).unwrap();

    // Walk commits chronologically, assign to phases by scope match
    let mut revwalk = match repo.revwalk() {
        Ok(rw) => rw,
        Err(_) => return,
    };
    revwalk.set_sorting(git2::Sort::TIME | git2::Sort::REVERSE).ok();
    revwalk.push_head().ok();

    for oid_result in revwalk {
        let oid = match oid_result { Ok(o) => o, Err(_) => continue };
        let commit = match repo.find_commit(oid) { Ok(c) => c, Err(_) => continue };
        let msg = commit.summary().unwrap_or("");

        // Try to match a phase number from the commit scope
        if let Some(phase_num) = extract_phase_number_from_commit(msg, &phase_scope_re) {
            for phase in phases.iter_mut() {
                if phases_match_number(&phase.number, &phase_num) {
                    let sha = oid.to_string();
                    match &mut phase.commit_range {
                        None => phase.commit_range = Some((sha.clone(), sha)),
                        Some((ref start, ref mut end)) => {
                            // Extend end to this (later) commit
                            *end = sha;
                        }
                    }
                }
            }
        }
    }
}
```

### Anti-Patterns to Avoid

- **Using serde_yaml for frontmatter parsing:** `serde_yaml` is not in the dependency tree and adds a heavy transitive dependency. Manual regex parsing of the simple YAML used in plan files is sufficient and fast.
- **Blocking the UI thread with file system reads:** Even though parsing is fast (~5ms), it must run on a background thread via the established flag-spawn-ScanMsg pattern.
- **Exact-only path matching without directory prefix check:** PLAN.md `files_modified` often lists directory paths like `sentrux-core/src/app/mcp_server/`. Without prefix matching, all files in that directory are missed.
- **Adding GsdPhase ColorMode variant AFTER `#[serde(other)] Monochrome`:** The serde fallback variant must always be last. Insert GsdPhase before Monochrome. See Pattern from Phase 2.1 research.
- **Hardcoding `.planning/` at scan root:** Some projects have `.planning/` in a parent directory. `find_planning_dir()` should walk up a few levels.
- **Walking the full commit history for every scan:** Phase commit range detection can be expensive on large repos. Cap the revwalk at 2000 commits. If a phase has no matched commits, its `commit_range` stays `None` — the navigator button shows but clicking it does not change the GitDiff window.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| YAML parsing | Custom YAML parser | Manual frontmatter string parsing with `str` methods | Frontmatter is simple YAML sequences — no nesting beyond `key-files.created`/`key-files.modified`; full YAML parser is unnecessary overhead |
| Phase status detection | Date-comparison logic | ROADMAP.md `[x]`/`[ ]` checkbox + "first incomplete = in-progress" rule | Checkboxes are the canonical source of truth; don't infer from dates |
| Git commit attribution | Custom diff attribution | Parse GSD commit scopes (`feat(03-01):`) from existing git log | GSD convention already encodes phase info in commits |
| Background threading | New threading primitives | Existing `std::thread::Builder + crossbeam-channel` pattern (identical to `maybe_spawn_git_diff_thread`) | Established pattern across all Phase 2.1, 3 features |

---

## Common Pitfalls

### Pitfall 1: Directory Path Entries in files_modified

**What goes wrong:** PLAN.md `files_modified` lists `sentrux-core/src/app/mcp_server/` (trailing slash, directory). Exact `by_file.get(path)` lookup misses all individual files in that directory.

**Why it happens:** Plan files list directories when an entire subtree was deleted or created, not individual files.

**How to avoid:** In `color_by_gsd_phase()`, after the exact match fails, run `find_directory_match()` which checks if any entry in `by_file` is a prefix of the current path. Since directory entries have their trailing `/` stripped by `normalize_path()`, check `path.starts_with(key)` where `key` ends without `/`.

**Warning signs:** Phase 1 completion shows zero files colored (the phase deleted whole directories, not individual files).

### Pitfall 2: PLAN.md vs SUMMARY.md Path Discrepancies

**What goes wrong:** PLAN.md lists `files_modified` at planning time (what we intend to change). SUMMARY.md `key-files.modified` is the post-execution truth (what actually changed). They often differ.

**Why it happens:** Plans change during execution; auto-fixes add files not in the plan.

**How to avoid:** Collect from BOTH sources; union the two lists. Duplicates are deduplicated by `if !phase.files.contains()` check. SUMMARY.md takes precedence for completed phases (more accurate), but both are included to handle in-progress/planned phases that only have PLAN.md.

**Warning signs:** A completed phase shows fewer files than expected; checking the SUMMARY.md would show additional files that were actually touched.

### Pitfall 3: ColorMode Serde — GsdPhase Must Be Before Monochrome

**What goes wrong:** Adding `GsdPhase` variant to `ColorMode` after `#[serde(other)] Monochrome` means saved prefs with `"GsdPhase"` deserialize to `Monochrome` instead.

**Why it happens:** `#[serde(other)]` consumes all unrecognized variants — but it also consumes variants defined AFTER it.

**How to avoid:** Insert `GsdPhase` in the enum before `Monochrome`. The enum must be:
```rust
pub enum ColorMode {
    Language, Heat, Git, TdgGrade, Coverage, Risk, GitDiff,
    GsdPhase,   // NEW — BEFORE Monochrome
    #[serde(other)]
    Monochrome, // MUST remain last
}
```

**Warning signs:** Switching to GsdPhase mode, closing app, reopening — color mode has reverted to Monochrome.

### Pitfall 4: Phase Number Normalization for Directory Matching

**What goes wrong:** Phase number in ROADMAP.md is `"2.1"` but directory name is `"02.1-rust-deep-analysis"`. A simple `dir_name.starts_with("2.1")` fails.

**Why it happens:** GSD uses zero-padded two-digit phase numbers in directory names but unpadded numbers in ROADMAP.md text.

**How to avoid:** Implement `zero_pad_phase()`:
```rust
fn zero_pad_phase(number: &str) -> String {
    let (major, minor) = match number.split_once('.') {
        Some((maj, min)) => (maj, Some(min)),
        None => (number, None),
    };
    let padded = format!("{:0>2}", major);
    match minor {
        Some(min) => format!("{}.{}", padded, min),
        None => padded,
    }
}
// "1" → "01", "2.1" → "02.1", "4" → "04"
```

**Warning signs:** Phase 1 files populated, phases 2+ empty — the padded-number mismatch is the cause.

### Pitfall 5: .planning/ Not Found for Non-GSD Projects

**What goes wrong:** User opens a project without a `.planning/` directory. `parse_gsd_phases()` returns `None`. The `GsdPhaseReady` message carries `None`. The color mode should gracefully fall back rather than crashing.

**Why it happens:** GSD phase overlay only applies to GSD-planned projects.

**How to avoid:** When `GsdPhaseReady(None)` is received, store `None` in `state.gsd_phase_report`. In `color_by_gsd_phase()`, when `ctx.gsd_phase_report.is_none()`, return `ctx.theme_config.file_surface` (monochrome fallback). The navigator panel shows "No .planning/ directory found" message.

**Warning signs:** Treemap shows monochrome when GsdPhase mode selected — expected behavior for non-GSD projects. Should not panic.

### Pitfall 6: DiffWindow::CommitRange Must Be Handled in git_walker.rs

**What goes wrong:** Adding `CommitRange` to `DiffWindow` causes non-exhaustive match compiler errors in `walk_git_log_windowed()` and anywhere `DiffWindow` is matched.

**Why it happens:** Rust's exhaustive enum matching catches all new variants.

**How to avoid:** Search all `match diff_window` sites in `git_walker.rs` and `git_diff_adapter.rs`. For the `CommitRange` variant, implement the walk differently: use `git2::Repository::merge_base` to find the range, then filter commits by OID membership rather than by timestamp.

**Warning signs:** Compiler errors at `match window` in `git_walker.rs` after adding `CommitRange`.

---

## Code Examples

### Phase Legend in draw_color_legend()

Following the established pattern in `draw_panels.rs`:

```rust
// Source: existing draw_git_diff_legend pattern in draw_panels.rs
fn draw_gsd_phase_legend(ui: &mut egui::Ui) {
    use crate::renderer::colors::gsd_phase_color;
    use crate::core::pmat_types::PhaseStatus;

    draw_swatch(ui, gsd_phase_color(PhaseStatus::Completed));
    ui.add_space(2.0);
    ui.label(egui::RichText::new("completed").small().weak());
    ui.add_space(8.0);

    draw_swatch(ui, gsd_phase_color(PhaseStatus::InProgress));
    ui.add_space(2.0);
    ui.label(egui::RichText::new("in progress").small().weak());
    ui.add_space(8.0);

    draw_swatch(ui, gsd_phase_color(PhaseStatus::Planned));
    ui.add_space(2.0);
    ui.label(egui::RichText::new("planned").small().weak());
    ui.add_space(8.0);

    draw_swatch(ui, crate::renderer::colors::NO_DATA_GRAY);
    ui.add_space(2.0);
    ui.label(egui::RichText::new("unassociated").small().weak());
}
```

### Auto-trigger on GsdPhase Mode Switch (toolbar.rs)

```rust
// Mirrors the GitDiff auto-trigger pattern at toolbar.rs line 186
if state.color_mode == ColorMode::GsdPhase
    && prev_color_mode != ColorMode::GsdPhase
    && state.gsd_phase_report.is_none()
    && !state.gsd_phase_running
    && !state.scanning
{
    state.gsd_phase_requested = true;
}
```

### ScanMsg Handling in scanning.rs

```rust
// Mirrors GitDiffReady handling at scanning.rs line 73
ScanMsg::GsdPhaseReady(report) => {
    self.state.gsd_phase_report = Some(report);
    self.state.gsd_phase_running = false;
    ctx.request_repaint();
}
ScanMsg::GsdPhaseError(msg) => {
    eprintln!("[gsd-phase] {msg}");
    self.state.gsd_phase_running = false;
    ctx.request_repaint();
}
```

### Phase Tooltip Integration

The GSD phase overlay tooltip (GSDP-05) reuses `state.hovered_path`. In `status_bar.rs`, extend the hover display to show phase info when in GsdPhase mode:

```rust
// status_bar.rs — extend draw_left_info
if let Some(path) = &state.hovered_path {
    // ... existing file info display ...

    // Additional: phase info if in GsdPhase mode
    if state.color_mode == ColorMode::GsdPhase {
        if let Some(report) = &state.gsd_phase_report {
            if let Some(phase) = report.phase_for_file(path) {
                ui.label(egui::RichText::new(
                    format!("Phase {}: {}  |  {}", phase.number, phase.name, phase.goal)
                ).small().weak().color(gsd_phase_color(phase.status)));
            }
        }
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No plan visualization | GSD phase overlay colors files by planning phase | Phase 4 | Developer can see spatial distribution of work across the project |
| GitDiff only shows past | Phase overlay shows future (planned), present (in-progress), and past (completed) | Phase 4 | Completes the "past / present / future" triple overlay thesis |
| Fixed DiffWindow presets | `DiffWindow::CommitRange` allows exact phase commit ranges | Phase 4 | Phase navigator can link to specific phase diffs |

---

## Open Questions

1. **Phase number display: bare number vs padded vs slug**
   - What we know: ROADMAP.md uses bare numbers ("1", "2.1", "4"). Directory names are padded ("01-cleanup", "02.1-rust-deep-analysis"). Navigator buttons need a display label.
   - Recommendation: Use bare number + name in the navigator button label: "1 Cleanup", "2.1 Rust Deep Analysis", "4 GSD Phase Overlay". This matches what users see in ROADMAP.md.

2. **What if a project has many phases (10+)?**
   - What we know: The horizontal navigator has limited width. More than ~6 phases will overflow.
   - Recommendation: Use egui's `ScrollArea` wrapping the horizontal phase list. Each button is compact (number + abbreviated name). Tooltip shows full name/goal. This defers to Claude's discretion as specified in CONTEXT.md.

3. **Commit range detection: what if commits span multiple phases?**
   - What we know: Some commits (e.g., `refactor:` without phase scope) don't match any phase. The CONTEXT.md says "time-based fallback using completion dates."
   - Recommendation: If a commit has no phase scope, assign it to the phase whose `completed_date` is closest in time (after the commit date). If no completion dates are available, leave the `commit_range` as `None` for that phase — the navigator button still shows but does not trigger GitDiff.

4. **Performance of directory prefix matching**
   - What we know: `by_file` will have on the order of 100-300 entries for a medium-sized project. `find_directory_match()` is O(N) per file render call.
   - Recommendation: Pre-process directory entries into a separate `by_dir_prefix: Vec<(String, usize)>` sorted list, enabling binary search. At typical project sizes (<300 entries) the O(N) scan is acceptably fast (< 1 microsecond per file), so this optimization is optional for v1.

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
| GSDP-01 | `color_by_gsd_phase` returns phase color for associated file | unit | `cargo test -p sentrux-core color_by_gsd_phase_associated_file` | ❌ Wave 0 |
| GSDP-01 | `GsdPhaseReport::by_file` lookup returns correct phase index | unit | `cargo test -p sentrux-core gsd_phase_report_by_file_lookup` | ❌ Wave 0 |
| GSDP-02 | `gsd_phase_color(Completed)` returns greenish color | unit | `cargo test -p sentrux-core gsd_phase_color_completed_is_green` | ❌ Wave 0 |
| GSDP-02 | `gsd_phase_color(InProgress)` returns amber color | unit | `cargo test -p sentrux-core gsd_phase_color_in_progress_is_amber` | ❌ Wave 0 |
| GSDP-02 | `gsd_phase_color(Planned)` returns blue color | unit | `cargo test -p sentrux-core gsd_phase_color_planned_is_blue` | ❌ Wave 0 |
| GSDP-03 | `parse_roadmap_phases` detects completed/in-progress/planned from fixture | unit | `cargo test -p sentrux-core parse_roadmap_phases_status_detection` | ❌ Wave 0 |
| GSDP-03 | `extract_files_modified` returns list from fixture PLAN.md frontmatter | unit | `cargo test -p sentrux-core extract_files_modified_from_fixture` | ❌ Wave 0 |
| GSDP-03 | `extract_key_files` returns created+modified from fixture SUMMARY.md | unit | `cargo test -p sentrux-core extract_key_files_from_fixture` | ❌ Wave 0 |
| GSDP-03 | `normalize_path` strips `./` and trailing `/` | unit | `cargo test -p sentrux-core normalize_path_strips_prefix` | ❌ Wave 0 |
| GSDP-03 | `zero_pad_phase("2.1")` returns `"02.1"` | unit | `cargo test -p sentrux-core zero_pad_phase_decimal` | ❌ Wave 0 |
| GSDP-04 | `color_by_gsd_phase` returns NO_DATA_GRAY for unassociated file | unit | `cargo test -p sentrux-core color_by_gsd_phase_unassociated_is_gray` | ❌ Wave 0 |
| GSDP-04 | `color_by_gsd_phase` returns monochrome when no report present | unit | `cargo test -p sentrux-core color_by_gsd_phase_no_report_is_monochrome` | ❌ Wave 0 |
| GSDP-05 | `GsdPhaseReport::phase_for_file` returns correct PhaseInfo | unit | `cargo test -p sentrux-core gsd_phase_report_phase_for_file` | ❌ Wave 0 |
| GSDP-05 | Hover shows phase name in status bar (visual verification) | manual | Run sentrux on this project in GsdPhase mode, hover a colored file | N/A |
| GSDP-01–05 | Full integration: `parse_gsd_phases` on this repo's `.planning/` dir returns non-empty report | integration | `cargo test -p sentrux-core parse_gsd_phases_on_sentrux_planning` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p sentrux-core --lib 2>&1 | tail -20`
- **Per wave merge:** `cargo test --workspace 2>&1 | tail -30`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `sentrux-core/src/core/pmat_types.rs` — add `PhaseStatus`, `PhaseInfo` (with `commit_range` field), `GsdPhaseReport` types
- [ ] `sentrux-core/src/analysis/gsd_phase_adapter.rs` — new file with `parse_gsd_phases()`, `find_planning_dir()`, `parse_roadmap_phases()`, `collect_phase_files()`, `extract_files_modified()`, `extract_key_files()`, `normalize_path()`, `zero_pad_phase()`, `detect_phase_commit_ranges()`, test fixtures
- [ ] `sentrux-core/src/renderer/colors.rs` — add `gsd_phase_color()`
- [ ] `sentrux-core/src/layout/types.rs` — add `ColorMode::GsdPhase` variant (before `Monochrome`)
- [ ] `sentrux-core/src/metrics/evo/git_walker.rs` — add `DiffWindow::CommitRange` variant and handle in all match arms

---

## Sources

### Primary (HIGH confidence)
- `.planning/phases/01-cleanup/01-01-PLAN.md` — `files_modified` frontmatter format verified (exact YAML structure)
- `.planning/phases/01-cleanup/01-01-SUMMARY.md` — `key-files:` section format verified (created/modified sub-lists)
- `.planning/phases/03-git-diff-overlay/03-03-SUMMARY.md` — `key-files:` format cross-verified
- `.planning/ROADMAP.md` — `- [x]` / `- [ ]` checkbox format, `(completed YYYY-MM-DD)` date format verified
- `git log --oneline` output — GSD commit message convention verified (`feat(03-01):`, `docs(04):` patterns)
- `sentrux-core/Cargo.toml` — `regex = "1"` and `toml = "0.8"` confirmed present; no serde_yaml
- `sentrux-core/src/app/channels.rs` — ScanMsg enum, ScanReports struct verified for extension pattern
- `sentrux-core/src/app/draw_panels.rs` — `maybe_spawn_git_diff_thread()` pattern for replication
- `sentrux-core/src/app/scanning.rs` — `GitDiffReady`/`CoverageReady` handler pattern
- `sentrux-core/src/renderer/rects.rs` — `file_color()` dispatch, `color_by_git_diff()` pattern
- `sentrux-core/src/renderer/mod.rs` — `RenderContext` struct for extension
- `sentrux-core/src/renderer/colors.rs` — `NO_DATA_GRAY`, `git_diff_intensity_color()` patterns
- `sentrux-core/src/layout/types.rs` — `ColorMode` enum with `#[serde(other)] Monochrome` constraint
- `sentrux-core/src/app/toolbar.rs` — auto-trigger pattern, Coverage gating pattern

### Secondary (MEDIUM confidence)
- Phase 3 RESEARCH.md — DiffWindow types, git2 walk patterns, established background thread architecture
- Phase 2.1 RESEARCH.md — ColorMode serde ordering constraint (verified in layout/types.rs)
- STATE.md accumulated decisions — path normalization concern flagged

---

## Metadata

**Confidence breakdown:**
- GSD file formats: HIGH — verified by reading actual .planning/ files in this repo
- Phase status detection logic: HIGH — ROADMAP.md format is simple and consistent
- Code wiring patterns: HIGH — directly modeled on existing GitDiff and Coverage implementations in the codebase
- Commit range detection: MEDIUM — GSD commit convention confirmed from git log; edge cases (commits spanning phases) require runtime validation
- Phase navigator UI layout: MEDIUM — egui horizontal layout with ScrollArea is standard; exact sizing needs empirical tuning during implementation

**Research date:** 2026-03-15
**Valid until:** 2026-04-15 (30 days; GSD file formats are stable; egui API is stable)
