//! GSD phase overlay adapter — parses `.planning/` directory structure into a `GsdPhaseReport`.
//!
//! Reads ROADMAP.md for phase status, collects files from PLAN.md `files_modified`
//! and SUMMARY.md `key-files`, and detects phase commit ranges from git commit messages.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::core::pmat_types::{CommitSummary, GsdPhaseReport, PhaseInfo, PhaseStatus};

/// Compiled once — matches commit scopes like (02), (02-01), (03.1), (4).
static SCOPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\((\d+)(?:[.\-](\d+))?\)").expect("valid regex")
});

// ── Public API ───────────────────────────────────────────────────────────

/// Parse GSD planning phases from the `.planning/` directory near `scan_root`.
///
/// Returns `None` if no `.planning/` directory is found.
pub fn parse_gsd_phases(scan_root: &str) -> Option<GsdPhaseReport> {
    let planning_dir = find_planning_dir(scan_root)?;
    let roadmap_path = planning_dir.join("ROADMAP.md");
    let roadmap_content = std::fs::read_to_string(&roadmap_path).ok()?;

    let mut phases = parse_roadmap_phases(&roadmap_content);
    if phases.is_empty() {
        return None;
    }

    let phases_dir = planning_dir.join("phases");
    if phases_dir.exists() {
        for phase in &mut phases {
            collect_phase_files(&phases_dir, phase, scan_root);
        }
    }

    detect_phase_commit_ranges(scan_root, &mut phases);

    // Build by_file index: last/most-recent phase wins for a given file
    let mut by_file: HashMap<String, usize> = HashMap::new();
    for (idx, phase) in phases.iter().enumerate() {
        for file in &phase.files {
            by_file.insert(file.clone(), idx);
        }
    }

    // Collect commit summaries from git history, annotated with phase_idx
    let commits = collect_commit_summaries(scan_root, &phases);

    Some(GsdPhaseReport { phases, by_file, commits })
}

// ── Internal helpers ──────────────────────────────────────────────────────

/// Search for `.planning/` directory starting at `scan_root`, walking up to 3 parents.
pub(crate) fn find_planning_dir(scan_root: &str) -> Option<PathBuf> {
    let start = Path::new(scan_root);
    let mut current = start.to_path_buf();
    for _ in 0..4 {
        let candidate = current.join(".planning");
        if candidate.is_dir() {
            return Some(candidate);
        }
        current = match current.parent() {
            Some(p) => p.to_path_buf(),
            None => break,
        };
    }
    None
}

/// Parse ROADMAP.md content into a list of PhaseInfo.
///
/// Recognizes lines like:
/// - `- [x] **Phase 1: Name** - Goal (completed 2026-03-01)`
/// - `- [ ] **Phase 2: Name** - Goal`
///
/// Status: [x]=Completed, first [ ]=InProgress, subsequent [ ]=Planned.
pub(crate) fn parse_roadmap_phases(content: &str) -> Vec<PhaseInfo> {
    use regex::Regex;

    // Pattern: `- [x/space] **Phase N[.M]: Name** - Goal (completed DATE)?`
    let re = Regex::new(
        r"(?m)^\s*-\s*\[([ xX])\]\s*\*\*Phase\s+([\d.]+):\s*([^*]+)\*\*\s*[-–]\s*([^\n(]+?)(?:\s*\(completed\s+([^)]+)\))?\s*$"
    ).expect("valid regex");

    let mut phases = Vec::new();
    let mut found_incomplete = false;

    for cap in re.captures_iter(content) {
        let checkbox = cap.get(1).map(|m| m.as_str()).unwrap_or(" ");
        let number_raw = cap.get(2).map(|m| m.as_str()).unwrap_or("0");
        let name = cap.get(3).map(|m| m.as_str().trim()).unwrap_or("").to_string();
        let goal = cap.get(4).map(|m| m.as_str().trim()).unwrap_or("").to_string();
        let completed_date = cap.get(5).map(|m| m.as_str().trim().to_string());

        let number = zero_pad_phase(number_raw);
        let status = if checkbox.trim() == "x" || checkbox.trim() == "X" {
            PhaseStatus::Completed
        } else if !found_incomplete {
            found_incomplete = true;
            PhaseStatus::InProgress
        } else {
            PhaseStatus::Planned
        };

        phases.push(PhaseInfo {
            number,
            name,
            goal,
            status,
            completed_date,
            files: Vec::new(),
            commit_range: None,
        });
    }

    phases
}

/// Collect files for a phase from PLAN.md files_modified and SUMMARY.md key-files.
///
/// Searches for the phase directory by number (with zero-padding variants),
/// then reads all *-PLAN.md and *-SUMMARY.md files.
fn collect_phase_files(phases_dir: &Path, phase: &mut PhaseInfo, scan_root: &str) {
    // Find the phase subdirectory — it should start with the padded number
    let phase_num = &phase.number;
    let phase_dir = match find_phase_dir(phases_dir, phase_num) {
        Some(d) => d,
        None => return,
    };

    let mut files: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Read all *-PLAN.md files in the phase directory
    if let Ok(entries) = std::fs::read_dir(&phase_dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            if fname.ends_with("-PLAN.md") {
                if let Some(plan_files) = extract_files_modified(&path) {
                    for f in plan_files {
                        let norm = normalize_path(&f);
                        if !norm.is_empty() {
                            files.insert(make_relative_to_scan_root(&norm, scan_root));
                        }
                    }
                }
            } else if fname.ends_with("-SUMMARY.md") {
                if let Some(key_files) = extract_key_files(&path) {
                    for f in key_files {
                        let norm = normalize_path(&f);
                        if !norm.is_empty() {
                            files.insert(make_relative_to_scan_root(&norm, scan_root));
                        }
                    }
                }
            }
        }
    }

    phase.files = files.into_iter().collect();
    phase.files.sort();
}

/// Find the phase subdirectory under `phases_dir` that matches the given number prefix.
fn find_phase_dir(phases_dir: &Path, phase_num: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(phases_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        // Directory name should start with "NN-" where NN matches phase_num
        if name.starts_with(&format!("{}-", phase_num)) || name.starts_with(phase_num) {
            if entry.path().is_dir() {
                return Some(entry.path());
            }
        }
    }
    None
}

/// Make a path relative to the scan root if it happens to be absolute.
/// If already relative, return as-is.
fn make_relative_to_scan_root(path: &str, scan_root: &str) -> String {
    let root_with_sep = if scan_root.ends_with('/') {
        scan_root.to_string()
    } else {
        format!("{}/", scan_root)
    };
    if path.starts_with(&root_with_sep) {
        path[root_with_sep.len()..].to_string()
    } else if path.starts_with(scan_root) {
        path[scan_root.len()..].trim_start_matches('/').to_string()
    } else {
        path.to_string()
    }
}

/// Detect commit ranges for phases by parsing commit messages.
///
/// Looks for GSD-style commit scopes like `feat(02-01):`, `docs(phase-3):`.
/// Caps the walk at 2000 commits.
fn detect_phase_commit_ranges(scan_root: &str, phases: &mut Vec<PhaseInfo>) {
    use git2::{Repository, Sort};

    let repo = match Repository::discover(scan_root) {
        Ok(r) => r,
        Err(_) => return,
    };
    let mut revwalk = match repo.revwalk() {
        Ok(rw) => rw,
        Err(_) => return,
    };
    if revwalk.set_sorting(Sort::REVERSE).is_err() {
        return;
    }
    if revwalk.push_head().is_err() {
        return;
    }

    // Phase number → index in phases
    let phase_index: HashMap<String, usize> = phases
        .iter()
        .enumerate()
        .map(|(i, p)| (p.number.clone(), i))
        .collect();

    let scope_re = &*SCOPE_RE;

    // phase_idx → (first_oid, last_oid)
    let mut ranges: HashMap<usize, (String, String)> = HashMap::new();
    let mut count = 0usize;

    for oid_result in revwalk {
        if count >= 2000 {
            break;
        }
        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => continue,
        };
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };
        count += 1;

        let msg = match commit.message() {
            Some(m) => m,
            None => continue,
        };
        // Extract phase number from commit scope
        if let Some(cap) = scope_re.captures(msg) {
            let phase_num_raw = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let phase_num = zero_pad_phase(phase_num_raw);
            if let Some(&idx) = phase_index.get(&phase_num) {
                let oid_str = oid.to_string();
                let entry = ranges.entry(idx).or_insert_with(|| (oid_str.clone(), oid_str.clone()));
                // REVERSE walk: first seen = earliest commit, last seen = most recent
                entry.1 = oid_str;
            }
        }
    }

    for (idx, (from, to)) in ranges {
        if idx < phases.len() {
            phases[idx].commit_range = Some((from, to));
        }
    }
}

/// Collect per-commit metadata for the timeline bar.
///
/// Walks the git history (newest first, up to 2000 commits), annotates each
/// commit with which phase it belongs to (if any), and returns them sorted by
/// epoch ascending (oldest first) for display in the timeline navigator.
fn collect_commit_summaries(scan_root: &str, phases: &[PhaseInfo]) -> Vec<CommitSummary> {
    use git2::{Repository, Sort};

    let repo = match Repository::discover(scan_root) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut revwalk = match repo.revwalk() {
        Ok(rw) => rw,
        Err(_) => return Vec::new(),
    };
    // Walk newest-to-oldest (default order) for efficiency
    if revwalk.push_head().is_err() {
        return Vec::new();
    }
    if revwalk.set_sorting(Sort::TIME).is_err() {
        return Vec::new();
    }

    // Build phase_index: phase number → index (same logic as detect_phase_commit_ranges)
    let phase_index: HashMap<String, usize> = phases
        .iter()
        .enumerate()
        .map(|(i, p)| (p.number.clone(), i))
        .collect();

    // Also build commit-sha → phase_idx from existing commit_range data
    // (phases that already have ranges from detect_phase_commit_ranges)
    // We re-walk independently here, using the scope regex for per-commit annotation.
    let scope_re = Regex::new(r"\((\d+)(?:[.\-](\d+))?\)").expect("valid regex");

    let mut commits = Vec::new();
    let mut count = 0usize;

    for oid_result in revwalk {
        if count >= 2000 {
            break;
        }
        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => continue,
        };
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };
        count += 1;

        let msg = commit.message().unwrap_or("").to_string();
        let first_line = msg.lines().next().unwrap_or("").to_string();
        let author = commit.author().name().unwrap_or("").to_string();
        let epoch = commit.time().seconds();
        let sha = oid.to_string();
        let short_sha = sha[..7.min(sha.len())].to_string();

        // Determine file count via diff against first parent
        let file_count = {
            let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
            let cur_tree = commit.tree().ok();
            match (parent_tree, cur_tree) {
                (Some(pt), Some(ct)) => {
                    repo.diff_tree_to_tree(Some(&pt), Some(&ct), None)
                        .map(|d| d.deltas().count())
                        .unwrap_or(0)
                }
                (None, Some(ct)) => {
                    // First commit in repo: count all files in tree
                    ct.iter().count()
                }
                _ => 0,
            }
        };

        // Annotate with phase_idx from commit scope
        let phase_idx = if let Some(cap) = scope_re.captures(&msg) {
            let phase_num_raw = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let phase_num = zero_pad_phase(phase_num_raw);
            phase_index.get(&phase_num).copied()
        } else {
            None
        };

        commits.push(CommitSummary {
            sha,
            short_sha,
            message: first_line,
            author,
            epoch,
            file_count,
            phase_idx,
        });
    }

    // Sort oldest-first for timeline display
    commits.sort_by_key(|c| c.epoch);
    commits
}

/// Parse YAML frontmatter `files_modified` list from a PLAN.md file.
pub(crate) fn extract_files_modified(plan_path: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(plan_path).ok()?;
    let fm = extract_frontmatter(&content)?;
    parse_yaml_string_list(fm, "files_modified")
}

/// Parse `key-files.created` and `key-files.modified` from a SUMMARY.md file.
pub(crate) fn extract_key_files(summary_path: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(summary_path).ok()?;
    let fm = extract_frontmatter(&content)?;
    let mut files = Vec::new();
    if let Some(created) = parse_yaml_nested_list(fm, "key-files", "created") {
        files.extend(created);
    }
    if let Some(modified) = parse_yaml_nested_list(fm, "key-files", "modified") {
        files.extend(modified);
    }
    if files.is_empty() { None } else { Some(files) }
}

/// Extract YAML frontmatter from document content (between first `---\n` and next `\n---`).
pub(crate) fn extract_frontmatter(content: &str) -> Option<&str> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return None;
    }
    let after_first = &content[3..];
    // skip to end of first `---` line
    let start = after_first.find('\n').map(|i| i + 1)?;
    let body = &after_first[start..];
    // Find closing `---`
    let end = body.find("\n---").unwrap_or(body.find("---\n").unwrap_or(body.len()));
    Some(&body[..end])
}

/// Parse a YAML sequence under `key:` from frontmatter content.
///
/// Handles both flow style (`key: [a, b]`) and block style:
/// ```yaml
/// key:
///   - a
///   - b
/// ```
pub(crate) fn parse_yaml_string_list(content: &str, key: &str) -> Option<Vec<String>> {
    // Try block sequence first
    let key_prefix = format!("{}:", key);
    let mut in_block = false;
    let mut result = Vec::new();
    let mut found_key = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if !found_key {
            if trimmed.starts_with(&key_prefix) {
                found_key = true;
                // Check for inline list: `key: [a, b, c]`
                let rest = trimmed[key_prefix.len()..].trim();
                if rest.starts_with('[') {
                    return parse_inline_list(rest);
                }
                in_block = true;
                continue;
            }
        } else if in_block {
            if trimmed.starts_with("- ") || trimmed == "-" {
                let item = trimmed.trim_start_matches('-').trim().trim_matches('"').trim_matches('\'').to_string();
                if !item.is_empty() {
                    result.push(item);
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                // New key — end of block
                break;
            }
        }
    }

    if result.is_empty() { None } else { Some(result) }
}

/// Parse a nested YAML sequence: `parent_key:\n  child_key:\n    - item`.
pub(crate) fn parse_yaml_nested_list(content: &str, parent_key: &str, child_key: &str) -> Option<Vec<String>> {
    let parent_prefix = format!("{}:", parent_key);
    let child_prefix = format!("{}:", child_key);
    let mut in_parent = false;
    let mut in_child = false;
    let mut result = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if !in_parent {
            if trimmed == parent_prefix || trimmed.starts_with(&parent_prefix) {
                in_parent = true;
                continue;
            }
        } else if !in_child {
            if trimmed == child_prefix || trimmed.starts_with(&child_prefix) {
                in_child = true;
                let rest = trimmed[child_prefix.len()..].trim();
                if rest.starts_with('[') {
                    return parse_inline_list(rest);
                }
                continue;
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with('-') {
                // Another top-level key — we've left the parent block
                let indent = line.chars().take_while(|c| c.is_whitespace()).count();
                if indent == 0 {
                    break;
                }
            }
        } else {
            // in child block
            if trimmed.starts_with("- ") || trimmed == "-" {
                let item = trimmed.trim_start_matches('-').trim().trim_matches('"').trim_matches('\'').to_string();
                if !item.is_empty() {
                    result.push(item);
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                // New key — end of child block
                let indent = line.chars().take_while(|c| c.is_whitespace()).count();
                if indent <= 2 {
                    break;
                }
            }
        }
    }

    if result.is_empty() { None } else { Some(result) }
}

/// Parse an inline YAML list like `[a, b, c]`.
fn parse_inline_list(s: &str) -> Option<Vec<String>> {
    let inner = s.trim().trim_start_matches('[').trim_end_matches(']');
    let items: Vec<String> = inner
        .split(',')
        .map(|item| item.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if items.is_empty() { None } else { Some(items) }
}

/// Normalize a file path: strip `./` prefix and trailing `/`.
pub(crate) fn normalize_path(path: &str) -> String {
    let p = path.trim_start_matches("./").trim_end_matches('/');
    p.to_string()
}

/// Zero-pad a phase number: "1" → "01", "2.1" → "02.1", "4" → "04".
pub(crate) fn zero_pad_phase(number: &str) -> String {
    if number.contains('.') {
        // Split at first dot, pad the integer part
        let dot_pos = number.find('.').unwrap();
        let int_part = &number[..dot_pos];
        let rest = &number[dot_pos..];
        format!("{:0>2}{}", int_part, rest)
    } else {
        format!("{:0>2}", number)
    }
}

/// Find a phase index by directory-prefix match on `path`.
///
/// A directory entry like `"src/app/"` matches `"src/app/state.rs"`.
pub(crate) fn find_directory_match(by_file: &HashMap<String, usize>, path: &str) -> Option<usize> {
    // Try progressively shorter prefixes
    let mut current = path;
    loop {
        let parent = match current.rfind('/') {
            Some(pos) => &current[..pos + 1], // include trailing slash
            None => break,
        };
        if let Some(&idx) = by_file.get(parent) {
            return Some(idx);
        }
        // Also try without trailing slash
        let parent_no_slash = parent.trim_end_matches('/');
        if let Some(&idx) = by_file.get(parent_no_slash) {
            return Some(idx);
        }
        current = parent.trim_end_matches('/');
        if current.is_empty() {
            break;
        }
    }
    None
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod gsd_phase_tests {
    use super::*;

    const ROADMAP_FIXTURE: &str = r#"
# Roadmap

## Phases

- [x] **Phase 1: Cleanup** - Remove plugins and narrow languages (completed 2026-01-15)
- [x] **Phase 2: PMAT Integration** - Integrate PMAT TDG grades (completed 2026-02-10)
- [ ] **Phase 3: Git Diff Overlay** - Color files by git diff intensity
- [ ] **Phase 4: GSD Phase Overlay** - Color files by GSD phase
- [ ] **Phase 5: Release** - Publish v1.0
"#;

    #[test]
    fn parse_roadmap_phases_extracts_correct_count() {
        let phases = parse_roadmap_phases(ROADMAP_FIXTURE);
        assert_eq!(phases.len(), 5, "should extract 5 phases");
    }

    #[test]
    fn parse_roadmap_phases_correct_names() {
        let phases = parse_roadmap_phases(ROADMAP_FIXTURE);
        assert_eq!(phases[0].name, "Cleanup");
        assert_eq!(phases[1].name, "PMAT Integration");
        assert_eq!(phases[2].name, "Git Diff Overlay");
    }

    #[test]
    fn parse_roadmap_phases_correct_goals() {
        let phases = parse_roadmap_phases(ROADMAP_FIXTURE);
        assert_eq!(phases[0].goal, "Remove plugins and narrow languages");
        assert_eq!(phases[2].goal, "Color files by git diff intensity");
    }

    #[test]
    fn parse_roadmap_phases_status_completed_for_checked() {
        let phases = parse_roadmap_phases(ROADMAP_FIXTURE);
        assert_eq!(phases[0].status, PhaseStatus::Completed);
        assert_eq!(phases[1].status, PhaseStatus::Completed);
    }

    #[test]
    fn parse_roadmap_phases_first_incomplete_is_in_progress() {
        let phases = parse_roadmap_phases(ROADMAP_FIXTURE);
        assert_eq!(phases[2].status, PhaseStatus::InProgress,
            "first incomplete phase should be InProgress");
    }

    #[test]
    fn parse_roadmap_phases_subsequent_incompletes_are_planned() {
        let phases = parse_roadmap_phases(ROADMAP_FIXTURE);
        assert_eq!(phases[3].status, PhaseStatus::Planned);
        assert_eq!(phases[4].status, PhaseStatus::Planned);
    }

    #[test]
    fn parse_roadmap_phases_completion_dates() {
        let phases = parse_roadmap_phases(ROADMAP_FIXTURE);
        assert_eq!(phases[0].completed_date, Some("2026-01-15".to_string()));
        assert_eq!(phases[1].completed_date, Some("2026-02-10".to_string()));
        assert_eq!(phases[2].completed_date, None);
    }

    #[test]
    fn parse_roadmap_phases_zero_padded_numbers() {
        let phases = parse_roadmap_phases(ROADMAP_FIXTURE);
        assert_eq!(phases[0].number, "01");
        assert_eq!(phases[1].number, "02");
        assert_eq!(phases[4].number, "05");
    }

    #[test]
    fn parse_roadmap_phases_empty_content_returns_empty() {
        let phases = parse_roadmap_phases("# No phases here");
        assert!(phases.is_empty());
    }

    // ── extract_files_modified ──

    fn make_temp_file(name: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "sentrux-gsd-test-{}-{}.md",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::write(&path, content).expect("write temp file");
        path
    }

    #[test]
    fn extract_files_modified_parses_block_list() {
        let content = r#"---
phase: 02-pmat
plan: 01
files_modified:
  - sentrux-core/src/app/channels.rs
  - sentrux-core/src/app/state.rs
  - sentrux-core/src/renderer/rects.rs
---

# Plan content
"#;
        let path = make_temp_file("plan", content);
        let files = extract_files_modified(&path).expect("should extract files");
        let _ = std::fs::remove_file(&path);
        assert_eq!(files.len(), 3);
        assert!(files.contains(&"sentrux-core/src/app/channels.rs".to_string()));
        assert!(files.contains(&"sentrux-core/src/renderer/rects.rs".to_string()));
    }

    #[test]
    fn extract_files_modified_returns_none_for_missing_key() {
        let content = r#"---
phase: test
plan: 01
---
"#;
        let path = make_temp_file("plan-no-files", content);
        let files = extract_files_modified(&path);
        let _ = std::fs::remove_file(&path);
        assert!(files.is_none(), "should return None when files_modified key is absent");
    }

    // ── extract_key_files ──

    #[test]
    fn extract_key_files_parses_nested_yaml() {
        let content = r#"---
phase: 02
plan: 01
key-files:
  created:
    - sentrux-core/src/analysis/pmat_adapter.rs
  modified:
    - sentrux-core/src/app/channels.rs
    - sentrux-core/src/app/state.rs
---
"#;
        let path = make_temp_file("summary", content);
        let files = extract_key_files(&path).expect("should extract key files");
        let _ = std::fs::remove_file(&path);
        assert_eq!(files.len(), 3);
        assert!(files.contains(&"sentrux-core/src/analysis/pmat_adapter.rs".to_string()));
        assert!(files.contains(&"sentrux-core/src/app/channels.rs".to_string()));
    }

    // ── normalize_path ──

    #[test]
    fn normalize_path_strips_dot_slash() {
        assert_eq!(normalize_path("./src/main.rs"), "src/main.rs");
    }

    #[test]
    fn normalize_path_strips_trailing_slash() {
        assert_eq!(normalize_path("src/app/"), "src/app");
    }

    #[test]
    fn normalize_path_strips_both() {
        assert_eq!(normalize_path("./src/app/"), "src/app");
    }

    #[test]
    fn normalize_path_leaves_bare_path_unchanged() {
        assert_eq!(normalize_path("src/main.rs"), "src/main.rs");
    }

    // ── zero_pad_phase ──

    #[test]
    fn zero_pad_phase_pads_single_digit() {
        assert_eq!(zero_pad_phase("1"), "01");
        assert_eq!(zero_pad_phase("4"), "04");
        assert_eq!(zero_pad_phase("9"), "09");
    }

    #[test]
    fn zero_pad_phase_leaves_two_digit_alone() {
        assert_eq!(zero_pad_phase("10"), "10");
        assert_eq!(zero_pad_phase("03"), "03");
    }

    #[test]
    fn zero_pad_phase_pads_dotted() {
        assert_eq!(zero_pad_phase("2.1"), "02.1");
        assert_eq!(zero_pad_phase("3.2"), "03.2");
    }

    // ── find_directory_match ──

    #[test]
    fn find_directory_match_matches_prefix() {
        let mut by_file: HashMap<String, usize> = HashMap::new();
        by_file.insert("src/app/".to_string(), 3);
        let result = find_directory_match(&by_file, "src/app/state.rs");
        assert_eq!(result, Some(3));
    }

    #[test]
    fn find_directory_match_exact_hit_not_needed() {
        let mut by_file: HashMap<String, usize> = HashMap::new();
        by_file.insert("sentrux-core/src/".to_string(), 1);
        let result = find_directory_match(&by_file, "sentrux-core/src/main.rs");
        assert_eq!(result, Some(1));
    }

    #[test]
    fn find_directory_match_no_match_returns_none() {
        let by_file: HashMap<String, usize> = HashMap::new();
        let result = find_directory_match(&by_file, "src/app/state.rs");
        assert!(result.is_none());
    }

    // ── GsdPhaseReport ──

    #[test]
    fn gsd_phase_report_phase_for_file_exact_match() {
        let phases = vec![
            PhaseInfo {
                number: "01".to_string(),
                name: "Cleanup".to_string(),
                goal: "Clean".to_string(),
                status: PhaseStatus::Completed,
                completed_date: None,
                files: vec!["src/main.rs".to_string()],
                commit_range: None,
            },
        ];
        let mut by_file = HashMap::new();
        by_file.insert("src/main.rs".to_string(), 0);
        let report = GsdPhaseReport { phases, by_file, commits: Vec::new() };
        let phase = report.phase_for_file("src/main.rs");
        assert!(phase.is_some());
        assert_eq!(phase.unwrap().name, "Cleanup");
    }

    #[test]
    fn gsd_phase_report_phase_for_file_unknown_returns_none() {
        let report = GsdPhaseReport {
            phases: vec![],
            by_file: HashMap::new(),
            commits: Vec::new(),
        };
        assert!(report.phase_for_file("unknown.rs").is_none());
    }

    #[test]
    fn gsd_phase_report_phase_count() {
        let phases = vec![
            PhaseInfo {
                number: "01".to_string(), name: "A".to_string(), goal: "g".to_string(),
                status: PhaseStatus::Completed, completed_date: None,
                files: vec![], commit_range: None,
            },
            PhaseInfo {
                number: "02".to_string(), name: "B".to_string(), goal: "g".to_string(),
                status: PhaseStatus::InProgress, completed_date: None,
                files: vec![], commit_range: None,
            },
        ];
        let report = GsdPhaseReport { phases, by_file: HashMap::new(), commits: Vec::new() };
        assert_eq!(report.phase_count(), 2);
    }

    // ── parse_gsd_phases returns None when no .planning dir ──

    #[test]
    fn parse_gsd_phases_returns_none_when_no_planning_dir() {
        // Use a path that definitely has no .planning directory
        let tmp = std::env::temp_dir().join(format!(
            "sentrux-test-no-planning-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).ok();
        let result = parse_gsd_phases(&tmp.to_string_lossy());
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(result.is_none(), "should return None when no .planning dir exists");
    }

    // ── extract_frontmatter ──

    #[test]
    fn extract_frontmatter_basic() {
        let content = "---\nkey: value\nother: foo\n---\n\n# Body";
        let fm = extract_frontmatter(content).expect("should extract frontmatter");
        assert!(fm.contains("key: value"), "frontmatter should contain key: value");
        assert!(!fm.contains("# Body"), "frontmatter should not include body");
    }

    #[test]
    fn extract_frontmatter_returns_none_if_no_dashes() {
        let content = "# Not YAML front matter\nSome content";
        assert!(extract_frontmatter(content).is_none());
    }
}
