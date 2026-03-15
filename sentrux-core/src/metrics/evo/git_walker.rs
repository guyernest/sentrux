//! Git log walking and commit parsing for evolution metrics.
//!
//! Extracts per-commit records from a repository using `git2` —
//! no shell-out to `git`. Designed for efficient sequential walking
//! with early cutoff by date.

use git2::{Repository, Sort};
use std::collections::HashSet;
use std::path::Path;

/// Maximum files per commit to consider (skip mega-merges that add noise).
const MAX_FILES_PER_COMMIT: usize = 50;

// ── Public types ──

/// Selectable time window for the git diff overlay.
///
/// Controls how far back `walk_git_log_windowed` looks when computing
/// per-file change intensity.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DiffWindow {
    /// Walk commits up to this many seconds before now
    TimeSecs(i64),
    /// Walk exactly this many commits (most recent first)
    CommitCount(u32),
    /// Walk commits since the most recent annotated or lightweight tag
    SinceLastTag,
    /// Walk commits between two OID hex strings (inclusive range for phase-based navigation)
    CommitRange {
        /// Starting commit OID (older end of range)
        from: String,
        /// Ending commit OID (newer end, typically HEAD of a phase)
        to: String,
    },
}

impl Default for DiffWindow {
    fn default() -> Self {
        DiffWindow::TimeSecs(86400) // 1 day
    }
}

impl DiffWindow {
    /// Preset time windows for the toolbar picker.
    /// Each entry is (variant, short label).
    /// Returns a static slice via OnceLock. DiffWindow is not Copy due to CommitRange.
    pub fn preset_slice() -> &'static [(DiffWindow, &'static str)] {
        use std::sync::OnceLock;
        static PRESETS: OnceLock<Vec<(DiffWindow, &'static str)>> = OnceLock::new();
        PRESETS.get_or_init(|| vec![
            (DiffWindow::TimeSecs(900),    "15m"),
            (DiffWindow::TimeSecs(3600),   "1h"),
            (DiffWindow::TimeSecs(86400),  "1d"),
            (DiffWindow::TimeSecs(604800), "1w"),
            (DiffWindow::SinceLastTag,     "tag"),
            (DiffWindow::CommitCount(1),   "1c"),
            (DiffWindow::CommitCount(5),   "5c"),
        ])
    }
}

/// Result of `walk_git_log_windowed`: commit records + set of new file paths.
pub struct DiffWalkResult {
    /// Commit records within the window
    pub records: Vec<CommitRecord>,
    /// Paths of files first added (created) within the window
    pub new_file_paths: HashSet<String>,
}

/// Per-commit record: which files changed, who authored it, when.
pub struct CommitRecord {
    pub author: String,
    pub epoch: i64,
    pub files: Vec<CommitFile>,
}

pub struct CommitFile {
    pub path: String,
    pub added: u32,
    pub removed: u32,
}

// ── Public API ──

/// Walk the git log from HEAD back `lookback_days` days, collecting per-commit records.
///
/// Skips merge commits and mega-commits (> 50 files). Returns records in
/// reverse chronological order.
pub(crate) fn walk_git_log(root: &Path, lookback_days: u32) -> Result<Vec<CommitRecord>, String> {
    let repo = Repository::discover(root).map_err(|e| format!("Git discover failed: {e}"))?;
    let workdir = repo
        .workdir()
        .ok_or("Bare repository — no working directory")?;

    let cutoff = epoch_now() - (lookback_days as i64 * 86400);

    let mut revwalk = repo.revwalk().map_err(|e| format!("Revwalk failed: {e}"))?;
    revwalk.set_sorting(Sort::TIME).map_err(|e| format!("Sort failed: {e}"))?;
    revwalk.push_head().map_err(|e| format!("Push HEAD failed: {e}"))?;

    let prefix = scan_root_prefix(root, workdir);
    let (records, skip_counts) = collect_commits(&repo, revwalk, cutoff, &prefix);

    let total_skipped = skip_counts.oid + skip_counts.commit + skip_counts.parse;
    if total_skipped > 0 {
        eprintln!(
            "[evolution] walked {} commits, skipped {} (oid_err={}, commit_err={}, unparseable={})",
            records.len() + total_skipped as usize, total_skipped,
            skip_counts.oid, skip_counts.commit, skip_counts.parse
        );
    }

    Ok(records)
}

/// Walk the git log using a `DiffWindow`, collecting per-commit records and new-file paths.
///
/// Supports four window modes:
/// - `TimeSecs(n)`: walk commits within the last `n` seconds
/// - `CommitCount(n)`: walk the most recent `n` non-merge non-mega commits
/// - `SinceLastTag`: walk commits since the most recent tag (falls back to full history if no tags)
/// - `CommitRange { from, to }`: walk commits between two OID hex strings
pub fn walk_git_log_windowed(root: &Path, window: DiffWindow) -> Result<DiffWalkResult, String> {
    let repo = Repository::discover(root).map_err(|e| format!("Git discover failed: {e}"))?;
    let workdir = repo
        .workdir()
        .ok_or("Bare repository — no working directory")?;

    let prefix = scan_root_prefix(root, workdir);

    // CommitRange uses a different revwalk setup — handle it separately
    if let DiffWindow::CommitRange { ref from, ref to } = window {
        return walk_commit_range(&repo, workdir, from, to, &prefix);
    }

    let mut revwalk = repo.revwalk().map_err(|e| format!("Revwalk failed: {e}"))?;
    revwalk.set_sorting(Sort::TIME).map_err(|e| format!("Sort failed: {e}"))?;
    revwalk.push_head().map_err(|e| format!("Push HEAD failed: {e}"))?;

    let cutoff: Option<i64> = match window {
        DiffWindow::TimeSecs(secs) => Some(epoch_now() - secs),
        DiffWindow::CommitCount(_) => None,
        DiffWindow::SinceLastTag => {
            match find_last_tag_epoch(&repo) {
                Ok(epoch) => Some(epoch),
                Err(_) => {
                    // No tags found — return empty result instead of walking entire history
                    return Ok(DiffWalkResult {
                        records: Vec::new(),
                        new_file_paths: std::collections::HashSet::new(),
                    });
                }
            }
        }
        DiffWindow::CommitRange { .. } => unreachable!("handled above"),
    };
    let max_count: Option<u32> = match window {
        DiffWindow::CommitCount(n) => Some(n),
        _ => None,
    };

    let mut records = Vec::new();
    let mut new_file_paths = HashSet::new();
    let mut commit_count = 0u32;
    let prefix_sep = if prefix.is_empty() { String::new() } else { format!("{prefix}/") };

    'outer: for oid_result in revwalk {
        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => continue,
        };
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // CommitCount mode: stop after N commits processed
        if let Some(max) = max_count {
            if commit_count >= max {
                break 'outer;
            }
        }

        // TimeSecs / SinceLastTag mode: stop when we reach the cutoff
        if let Some(cutoff_epoch) = cutoff {
            if commit.time().seconds() < cutoff_epoch {
                break 'outer;
            }
        }

        // Skip merge commits
        if commit.parent_count() > 1 {
            continue;
        }

        let author = commit.author().name().unwrap_or("unknown").to_string();
        let tree = match commit.tree() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
        let diff = match repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let num_deltas = diff.deltas().len();
        if num_deltas > MAX_FILES_PER_COMMIT {
            continue;
        }

        // Collect files and track new files (Added status)
        let mut files = Vec::new();
        for (i, delta) in diff.deltas().enumerate() {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string());
            let path = match path {
                Some(p) => p,
                None => continue,
            };
            let rel_path = if prefix_sep.is_empty() {
                path
            } else if let Some(stripped) = path.strip_prefix(&prefix_sep) {
                stripped.to_string()
            } else {
                continue
            };

            // Track new files via Added delta status
            if delta.status() == git2::Delta::Added {
                new_file_paths.insert(rel_path.clone());
            }

            let (added, removed) = get_patch_stats(&diff, i);
            files.push(CommitFile { path: rel_path, added, removed });
        }

        if files.is_empty() {
            continue;
        }

        records.push(CommitRecord {
            author,
            epoch: commit.time().seconds(),
            files,
        });
        commit_count += 1;
    }

    Ok(DiffWalkResult { records, new_file_paths })
}

/// Find the Unix epoch of the most recent tag in the repository.
///
/// Handles both lightweight tags (pointing directly to commits) and annotated
/// tags (pointing to tag objects) via `peel_to_commit()`.
///
/// Returns `Err` if the repository has no tags.
pub fn find_last_tag_epoch(repo: &Repository) -> Result<i64, String> {
    let tag_names = repo.tag_names(None).map_err(|e| format!("tag_names failed: {e}"))?;
    if tag_names.is_empty() {
        return Err("No tags found in repository".to_string());
    }

    let mut max_epoch: Option<i64> = None;
    for name in tag_names.iter().flatten() {
        let ref_name = format!("refs/tags/{name}");
        let reference = match repo.find_reference(&ref_name) {
            Ok(r) => r,
            Err(_) => continue,
        };
        // peel_to_commit handles both lightweight and annotated tags
        let commit = match reference.peel_to_commit() {
            Ok(c) => c,
            Err(_) => continue,
        };
        let epoch = commit.time().seconds();
        if max_epoch.map_or(true, |m| epoch > m) {
            max_epoch = Some(epoch);
        }
    }

    max_epoch.ok_or_else(|| "No tags found in repository".to_string())
}

/// Counts of skipped commits by reason.
struct SkipCounts {
    oid: u32,
    commit: u32,
    parse: u32,
}

/// Walk the revwalk iterator, collecting commit records and counting skips.
fn collect_commits(
    repo: &Repository,
    revwalk: git2::Revwalk<'_>,
    cutoff: i64,
    prefix: &str,
) -> (Vec<CommitRecord>, SkipCounts) {
    let mut records = Vec::new();
    let mut skips = SkipCounts { oid: 0, commit: 0, parse: 0 };

    for oid_result in revwalk {
        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => { skips.oid += 1; continue; }
        };
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => { skips.commit += 1; continue; }
        };
        if commit.time().seconds() < cutoff {
            break;
        }
        match parse_commit(repo, &commit, prefix) {
            Some(record) => records.push(record),
            None => { skips.parse += 1; }
        }
    }

    (records, skips)
}

// ── Internal helpers ──

pub(crate) fn epoch_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Determine the scan root relative to the workdir, for path filtering.
fn scan_root_prefix(root: &Path, workdir: &Path) -> String {
    let root_canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let workdir_canonical = workdir.canonicalize().unwrap_or_else(|_| workdir.to_path_buf());
    root_canonical
        .strip_prefix(&workdir_canonical)
        .unwrap_or(Path::new(""))
        .to_string_lossy()
        .to_string()
}

/// Parse a single commit into a CommitRecord, returning None for merge commits,
/// mega-commits, or commits that produce no relevant files.
fn parse_commit(
    repo: &Repository,
    commit: &git2::Commit<'_>,
    prefix: &str,
) -> Option<CommitRecord> {
    // Skip merge commits — they double-count changes.
    if commit.parent_count() > 1 {
        return None;
    }

    let author = commit.author().name().unwrap_or("unknown").to_string();
    let tree = match commit.tree() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[evolution] commit {}: tree() failed: {}", commit.id(), e);
            return None;
        }
    };
    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
    let diff = match repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[evolution] commit {}: diff failed: {}", commit.id(), e);
            return None;
        }
    };

    if let Err(e) = diff.stats() {
        eprintln!("[evolution] commit {}: diff.stats() failed: {}", commit.id(), e);
        return None;
    }

    let num_deltas = diff.deltas().len();
    if num_deltas > MAX_FILES_PER_COMMIT {
        return None;
    }

    let files = collect_diff_files(&diff, prefix);
    if files.is_empty() {
        return None;
    }

    Some(CommitRecord {
        author,
        epoch: commit.time().seconds(),
        files,
    })
}

/// Collect changed files from a diff, filtering to the scan root prefix.
fn collect_diff_files(diff: &git2::Diff<'_>, prefix: &str) -> Vec<CommitFile> {
    let mut files = Vec::new();
    for (i, delta) in diff.deltas().enumerate() {
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.to_string_lossy().to_string());
        let path = match path {
            Some(p) => p,
            None => continue,
        };
        let rel_path = if prefix.is_empty() {
            path
        } else if let Some(stripped) = path.strip_prefix(&format!("{prefix}/")) {
            stripped.to_string()
        } else {
            continue;
        };
        let (added, removed) = get_patch_stats(diff, i);
        files.push(CommitFile { path: rel_path, added, removed });
    }
    files
}

/// Extract added/removed line counts from a diff patch for a specific delta index.
fn get_patch_stats(diff: &git2::Diff, delta_idx: usize) -> (u32, u32) {
    let mut added = 0u32;
    let mut removed = 0u32;

    if let Ok(Some(patch)) = git2::Patch::from_diff(diff, delta_idx) {
        let (_, a, r) = patch.line_stats().unwrap_or((0, 0, 0));
        added = a as u32;
        removed = r as u32;
    }

    (added, removed)
}

/// Walk commits between two OID hex strings (inclusive range).
///
/// Resolves `from` and `to` as OIDs, then collects all commits reachable
/// from `to` that are not ancestors of `from`. Used for phase-based git
/// diff navigation via `DiffWindow::CommitRange`.
fn walk_commit_range(
    repo: &Repository,
    _workdir: &std::path::Path,
    from: &str,
    to: &str,
    prefix: &str,
) -> Result<DiffWalkResult, String> {
    let from_oid = repo.revparse_single(from)
        .map_err(|e| format!("CommitRange: could not resolve 'from' OID '{}': {e}", from))?
        .peel_to_commit()
        .map_err(|e| format!("CommitRange: 'from' is not a commit: {e}"))?
        .id();
    let to_oid = repo.revparse_single(to)
        .map_err(|e| format!("CommitRange: could not resolve 'to' OID '{}': {e}", to))?
        .peel_to_commit()
        .map_err(|e| format!("CommitRange: 'to' is not a commit: {e}"))?
        .id();

    let mut revwalk = repo.revwalk().map_err(|e| format!("Revwalk failed: {e}"))?;
    revwalk.set_sorting(Sort::TIME).map_err(|e| format!("Sort failed: {e}"))?;
    revwalk.push(to_oid).map_err(|e| format!("CommitRange: push 'to' failed: {e}"))?;
    revwalk.hide(from_oid).map_err(|e| format!("CommitRange: hide 'from' failed: {e}"))?;

    let prefix_sep = if prefix.is_empty() { String::new() } else { format!("{prefix}/") };
    let mut records = Vec::new();
    let mut new_file_paths = HashSet::new();

    for oid_result in revwalk {
        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => continue,
        };
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if commit.parent_count() > 1 {
            continue; // skip merges
        }

        let author = commit.author().name().unwrap_or("unknown").to_string();
        let tree = match commit.tree() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
        let diff = match repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None) {
            Ok(d) => d,
            Err(_) => continue,
        };
        if diff.deltas().len() > MAX_FILES_PER_COMMIT {
            continue;
        }

        let mut files = Vec::new();
        for (i, delta) in diff.deltas().enumerate() {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string());
            let path = match path {
                Some(p) => p,
                None => continue,
            };
            let rel_path = if prefix_sep.is_empty() {
                path
            } else if let Some(stripped) = path.strip_prefix(&prefix_sep) {
                stripped.to_string()
            } else {
                continue;
            };
            if delta.status() == git2::Delta::Added {
                new_file_paths.insert(rel_path.clone());
            }
            let (added, removed) = get_patch_stats(&diff, i);
            files.push(CommitFile { path: rel_path, added, removed });
        }

        if !files.is_empty() {
            records.push(CommitRecord { author, epoch: commit.time().seconds(), files });
        }
    }

    Ok(DiffWalkResult { records, new_file_paths })
}

// ── Unit tests ──

#[cfg(test)]
mod tests {
    use super::*;

    // ── DiffWindow tests ──

    #[test]
    fn diff_window_default_is_one_day() {
        assert_eq!(DiffWindow::default(), DiffWindow::TimeSecs(86400));
    }

    #[test]
    fn diff_window_presets_has_7_entries() {
        assert_eq!(DiffWindow::preset_slice().len(), 7);
    }

    #[test]
    fn diff_window_presets_contains_15m() {
        assert!(DiffWindow::preset_slice().iter().any(|(w, label)| {
            matches!(w, DiffWindow::TimeSecs(900)) && *label == "15m"
        }), "PRESETS should contain 15m entry");
    }

    #[test]
    fn diff_window_presets_contains_1h() {
        assert!(DiffWindow::preset_slice().iter().any(|(w, label)| {
            matches!(w, DiffWindow::TimeSecs(3600)) && *label == "1h"
        }), "PRESETS should contain 1h entry");
    }

    #[test]
    fn diff_window_presets_contains_1d() {
        assert!(DiffWindow::preset_slice().iter().any(|(w, label)| {
            matches!(w, DiffWindow::TimeSecs(86400)) && *label == "1d"
        }), "PRESETS should contain 1d entry");
    }

    #[test]
    fn diff_window_presets_contains_1w() {
        assert!(DiffWindow::preset_slice().iter().any(|(w, label)| {
            matches!(w, DiffWindow::TimeSecs(604800)) && *label == "1w"
        }), "PRESETS should contain 1w entry");
    }

    #[test]
    fn diff_window_presets_contains_tag() {
        assert!(DiffWindow::preset_slice().iter().any(|(w, label)| {
            matches!(w, DiffWindow::SinceLastTag) && *label == "tag"
        }), "PRESETS should contain tag entry");
    }

    #[test]
    fn diff_window_presets_contains_1c() {
        assert!(DiffWindow::preset_slice().iter().any(|(w, label)| {
            matches!(w, DiffWindow::CommitCount(1)) && *label == "1c"
        }), "PRESETS should contain 1c entry");
    }

    #[test]
    fn diff_window_presets_contains_5c() {
        assert!(DiffWindow::preset_slice().iter().any(|(w, label)| {
            matches!(w, DiffWindow::CommitCount(5)) && *label == "5c"
        }), "PRESETS should contain 5c entry");
    }

    #[test]
    fn diff_window_serde_roundtrip() {
        let w = DiffWindow::TimeSecs(3600);
        let json = serde_json::to_string(&w).expect("serialize");
        let decoded: DiffWindow = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(w, decoded);

        let w2 = DiffWindow::CommitCount(5);
        let json2 = serde_json::to_string(&w2).expect("serialize");
        let decoded2: DiffWindow = serde_json::from_str(&json2).expect("deserialize");
        assert_eq!(w2, decoded2);

        let w3 = DiffWindow::SinceLastTag;
        let json3 = serde_json::to_string(&w3).expect("serialize");
        let decoded3: DiffWindow = serde_json::from_str(&json3).expect("deserialize");
        assert_eq!(w3, decoded3);
    }

    #[test]
    fn diff_window_commit_range_serde_roundtrip() {
        let w = DiffWindow::CommitRange {
            from: "abc1234".to_string(),
            to: "def5678".to_string(),
        };
        let json = serde_json::to_string(&w).expect("serialize CommitRange");
        let decoded: DiffWindow = serde_json::from_str(&json).expect("deserialize CommitRange");
        assert_eq!(w, decoded);
    }

    // ── find_last_tag_epoch on a tagless temp repo ──

    #[test]
    fn find_last_tag_epoch_no_tags_returns_err() {
        // Create a temp repo with no tags
        let tmp_path = std::env::temp_dir().join(format!("sentrux-test-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));
        std::fs::create_dir_all(&tmp_path).expect("create temp dir");
        let repo = git2::Repository::init(&tmp_path).expect("init repo");
        let result = find_last_tag_epoch(&repo);
        let _ = std::fs::remove_dir_all(&tmp_path);
        assert!(result.is_err(), "find_last_tag_epoch should return Err on repo with no tags");
        let err = result.unwrap_err();
        assert!(err.contains("No tags"), "Error should mention 'No tags', got: {}", err);
    }
}
