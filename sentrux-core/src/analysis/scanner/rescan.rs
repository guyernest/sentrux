//! Incremental rescan: patches an existing snapshot with changes to specific files.
//!
//! Extracted from scanner.rs — handles file change detection, re-parsing, and
//! tree/graph rebuilding for changed files only.

use super::common::{
    ScanLimits, ScanResult, count_lines_batch, detect_lang,
    should_ignore_dir, should_ignore_file, MAX_FILES,
};
use super::tree::build_tree;
use crate::core::types::AppError;
use crate::core::snapshot::Snapshot;
use crate::core::types::FileNode;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

/// Incremental rescan: patch an existing snapshot with changes to specific files.
/// Re-parses only changed files, rebuilds tree + graphs.
/// Accepts `on_tree_ready` to emit partial snapshot before graph rebuild. [ref:7f9a39c8]
pub fn rescan_changed(
    root_path: &str,
    old_snap: &Snapshot,
    changed_rel_paths: &[String],
    on_tree_ready: Option<&dyn Fn(Snapshot)>,
    limits: &ScanLimits,
) -> Result<ScanResult, AppError> {
    let root = Path::new(root_path);
    let max_file_size_bytes = limits.max_file_size_kb * 1024;
    let max_parse_size = limits.max_parse_size_kb;
    let max_call_targets = limits.max_call_targets;

    // Flatten old snapshot into a mutable file list (clone cost ~ file count, not content)
    let mut files: Vec<FileNode> = crate::core::snapshot::flatten_files(&old_snap.root);

    // Expand directories and classify into reparse vs deleted
    let expanded = expand_directory_events(root, changed_rel_paths, max_file_size_bytes);
    let (to_reparse, deleted) = classify_changed_paths(root, &expanded, max_file_size_bytes);

    // Remove deleted files — exact match OR prefix match for deleted directories.
    // When a directory is deleted, macOS FSEvents may only report the directory
    // itself (not individual files within it), so we must also remove all files
    // whose path starts with "deleted_dir/".
    //
    // Collect directory prefixes once (with trailing '/') to avoid repeated
    // string building inside the hot retain loop.
    let deleted_dir_prefixes: Vec<String> = deleted.iter()
        .map(|d| format!("{}/", d))
        .collect();
    files.retain(|f| {
        if deleted.contains(&f.path) {
            return false;
        }
        // Check if any deleted path is a parent directory of this file
        deleted_dir_prefixes.iter().all(|prefix| !f.path.starts_with(prefix.as_str()))
    });

    // Batch line counts + structural analysis + git statuses
    let line_counts = batch_line_counts(&to_reparse);
    let sa_map = batch_parse_files(&to_reparse, max_parse_size);
    let git_statuses = crate::analysis::git::get_statuses(root_path);

    // Update or insert changed files into the file list
    upsert_changed_files(&mut files, &to_reparse, &line_counts, &sa_map, &git_statuses);

    // Enforce MAX_FILES limit (same as initial scan) [ref:93cf32d4]
    enforce_max_files(&mut files);

    // Build tree, emit partial snapshot, build graphs, return final result
    build_snapshot_with_graphs(root, files, on_tree_ready, max_call_targets)
}

/// Walk directories in `changed_rel_paths` to discover new files inside.
/// Non-directory paths are passed through. Applies same ignore/size filters
/// as collect_paths. [ref:93cf32d4]
fn expand_directory_events(
    root: &Path,
    changed_rel_paths: &[String],
    max_file_size_bytes: u64,
) -> Vec<String> {
    let mut expanded: Vec<String> = Vec::new();
    for rel in changed_rel_paths {
        let abs = root.join(rel);
        if abs.exists() && abs.is_dir() {
            expand_single_dir(root, &abs, max_file_size_bytes, &mut expanded);
        } else {
            expanded.push(rel.clone());
        }
        if expanded.len() >= MAX_FILES {
            break;
        }
    }
    expanded
}

/// Check if a walked entry is a valid file for expansion (not ignored, within size limit).
/// Returns Some(rel_path) if valid, None if the entry should be skipped.
fn validate_walk_entry(
    entry: &ignore::DirEntry,
    root: &Path,
    max_file_size_bytes: u64,
) -> Option<String> {
    if !entry.file_type().is_some_and(|ft| ft.is_file()) {
        return None;
    }
    let path = entry.path().to_path_buf();
    if should_ignore_file(&path) {
        return None;
    }
    if let Ok(meta) = fs::metadata(&path) {
        if meta.len() > max_file_size_bytes {
            return None;
        }
    }
    path.strip_prefix(root)
        .ok()
        .map(|rel| rel.to_string_lossy().to_string())
}

/// Walk a single directory and append discovered file rel-paths to `out`.
/// Same filters as collect_paths: ignore dirs, ignore files, size limit. [ref:93cf32d4]
fn expand_single_dir(
    root: &Path,
    dir_abs: &Path,
    max_file_size_bytes: u64,
    out: &mut Vec<String>,
) {
    for entry in ignore::WalkBuilder::new(dir_abs)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .max_depth(Some(20))
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                return !should_ignore_dir(&name);
            }
            true
        })
        .build()
    {
        if out.len() >= MAX_FILES {
            eprintln!("[rescan] expanded_paths hit MAX_FILES limit ({}), truncating", MAX_FILES);
            break;
        }
        if let Ok(e) = entry {
            if let Some(rel_path) = validate_walk_entry(&e, root, max_file_size_bytes) {
                out.push(rel_path);
            }
        }
    }
}

/// Separate expanded paths into files to reparse (exist on disk) and deleted files.
/// Applies same ignore/size filters as collect_paths for direct file paths. [ref:6c60c4ee]
fn classify_changed_paths(
    root: &Path,
    expanded: &[String],
    max_file_size_bytes: u64,
) -> (Vec<(String, PathBuf)>, HashSet<String>) {
    let mut to_reparse: Vec<(String, PathBuf)> = Vec::new();
    let mut deleted: HashSet<String> = HashSet::new();
    for rel in expanded {
        let abs = root.join(rel);
        if abs.exists() && abs.is_file() {
            if should_ignore_file(&abs) {
                continue;
            }
            if let Ok(meta) = fs::metadata(&abs) {
                if meta.len() > max_file_size_bytes {
                    continue;
                }
            }
            to_reparse.push((rel.clone(), abs));
        } else if !abs.exists() {
            deleted.insert(rel.clone());
        }
    }
    (to_reparse, deleted)
}

/// Batch tokei line counting for all reparse targets.
fn batch_line_counts(to_reparse: &[(String, PathBuf)]) -> HashMap<PathBuf, (u32, u32, u32, u32)> {
    let abs_paths: Vec<PathBuf> = to_reparse.iter().map(|(_, abs)| abs.clone()).collect();
    if abs_paths.is_empty() {
        HashMap::new()
    } else {
        count_lines_batch(&abs_paths)
    }
}

/// Batch structural analysis parsing in parallel for all reparse targets.
fn batch_parse_files(
    to_reparse: &[(String, PathBuf)],
    max_parse_size: usize,
) -> HashMap<String, crate::core::types::StructuralAnalysis> {
    let parse_inputs: Vec<(String, String, String)> = to_reparse
        .iter()
        .map(|(rel, abs)| {
            let lang = detect_lang(abs);
            (abs.to_string_lossy().to_string(), rel.clone(), lang)
        })
        .collect();
    crate::analysis::parser::parse_files_batch(&parse_inputs, max_parse_size)
        .into_iter()
        .collect()
}

/// Look up line counts for a file, with canonicalized-path fallback and read fallback.
fn lookup_line_counts(
    abs: &Path,
    line_counts: &HashMap<PathBuf, (u32, u32, u32, u32)>,
) -> (u32, u32, u32, u32) {
    line_counts
        .get(abs)
        .or_else(|| match abs.canonicalize() {
            Ok(cp) => line_counts.get(&cp),
            Err(_) => None,
        })
        .copied()
        .unwrap_or_else(|| {
            if let Ok(content) = fs::read_to_string(abs) {
                let total = content.lines().count() as u32;
                (total, 0, 0, 0)
            } else {
                (0, 0, 0, 0)
            }
        })
}

/// Build a FileNode from a changed file's metadata, line counts, and structural analysis.
fn build_file_node(
    rel: &str,
    abs: &Path,
    line_counts: &HashMap<PathBuf, (u32, u32, u32, u32)>,
    sa_map: &HashMap<String, crate::core::types::StructuralAnalysis>,
    git_statuses: &HashMap<String, String>,
) -> FileNode {
    let mtime = match fs::metadata(abs).and_then(|m| m.modified()) {
        Ok(t) => t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64(),
        Err(_) => 0.0,
    };
    let (lines, logic, comments, blanks) = lookup_line_counts(abs, line_counts);
    let lang = detect_lang(abs);
    let sa = sa_map.get(rel).cloned();
    let funcs = sa.as_ref().and_then(|s| s.functions.as_ref()).map_or(0, |v| v.len() as u32);
    let gs = git_statuses.get(rel).cloned().unwrap_or_default();
    let name = abs.file_name().unwrap_or_default().to_string_lossy().to_string();
    FileNode {
        path: rel.to_string(), name, is_dir: false,
        lines, logic, comments, blanks, funcs, mtime, gs, lang, sa,
        children: None,
    }
}

/// Update existing files or insert new ones from reparse results. O(1) per file via HashMap.
fn upsert_changed_files(
    files: &mut Vec<FileNode>,
    to_reparse: &[(String, PathBuf)],
    line_counts: &HashMap<PathBuf, (u32, u32, u32, u32)>,
    sa_map: &HashMap<String, crate::core::types::StructuralAnalysis>,
    git_statuses: &HashMap<String, String>,
) {
    let mut file_map: HashMap<String, usize> = files.iter().enumerate()
        .map(|(i, f)| (f.path.clone(), i)).collect();
    for (rel, abs) in to_reparse {
        let node = build_file_node(rel, abs, line_counts, sa_map, git_statuses);
        if let Some(&idx) = file_map.get(rel) {
            files[idx] = node;
        } else {
            file_map.insert(rel.clone(), files.len());
            files.push(node);
        }
    }
}

/// Enforce MAX_FILES limit: keep most recent files by mtime. [ref:93cf32d4]
fn enforce_max_files(files: &mut Vec<FileNode>) {
    if files.len() > MAX_FILES {
        files.sort_unstable_by(|a, b| b.mtime.total_cmp(&a.mtime));
        files.truncate(MAX_FILES);
    }
}

/// Build tree, emit partial snapshot via callback, then build graphs and return final result.
fn build_snapshot_with_graphs(
    root: &Path,
    files: Vec<FileNode>,
    on_tree_ready: Option<&dyn Fn(Snapshot)>,
    max_call_targets: usize,
) -> Result<ScanResult, AppError> {
    let total_files = files.len() as u32;
    let total_lines: u32 = files.iter().map(|f| f.lines as u64).sum::<u64>().min(u32::MAX as u64) as u32;
    let root_name = root.file_name().unwrap_or_default().to_string_lossy().to_string();

    let (tree, total_dirs) = build_tree(files, &root_name);
    let tree = Arc::new(tree);

    // Emit tree-ready with empty graphs — frontend renders rectangles immediately
    if let Some(cb) = on_tree_ready {
        cb(Snapshot {
            root: Arc::clone(&tree),
            total_files, total_lines, total_dirs,
            call_graph: Vec::new(), import_graph: Vec::new(),
            inherit_graph: Vec::new(), entry_points: Vec::new(),
            exec_depth: HashMap::new(),
        });
    }

    // Build graphs from flattened tree (zero-copy flatten)
    let flat_files = crate::core::snapshot::flatten_files_ref(&tree);
    let gr = crate::analysis::graph::build_graphs(&flat_files, Some(root), max_call_targets);

    Ok(ScanResult {
        snapshot: Snapshot {
            root: tree,
            total_files, total_lines, total_dirs,
            call_graph: gr.call_edges, import_graph: gr.import_edges,
            inherit_graph: gr.inherit_edges, entry_points: gr.entry_points,
            exec_depth: gr.exec_depth,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::FileNode;

    fn make_file(path: &str) -> FileNode {
        FileNode {
            path: path.to_string(),
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
            is_dir: false, lines: 10, logic: 8, comments: 1, blanks: 1,
            funcs: 1, mtime: 0.0, gs: String::new(), lang: "rust".into(),
            sa: None, children: None,
        }
    }

    fn make_file_with_mtime(path: &str, mtime: f64) -> FileNode {
        let mut f = make_file(path);
        f.mtime = mtime;
        f
    }

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    // ── Deletion tests (existing) ────────────────────────────────────────

    #[test]
    fn test_directory_deletion_removes_child_files() {
        let mut files = vec![
            make_file("src/foo/bar.rs"),
            make_file("src/foo/baz.rs"),
            make_file("src/main.rs"),
        ];
        let deleted: HashSet<String> = ["src/foo".to_string()].into_iter().collect();
        let deleted_dir_prefixes: Vec<String> = deleted.iter()
            .map(|d| format!("{}/", d)).collect();
        files.retain(|f| {
            if deleted.contains(&f.path) { return false; }
            deleted_dir_prefixes.iter().all(|prefix| !f.path.starts_with(prefix.as_str()))
        });
        assert_eq!(files.len(), 1, "Only src/main.rs should survive");
        assert_eq!(files[0].path, "src/main.rs");
    }

    #[test]
    fn test_individual_file_deletion() {
        let mut files = vec![make_file("src/foo.rs"), make_file("src/bar.rs")];
        let deleted: HashSet<String> = ["src/foo.rs".to_string()].into_iter().collect();
        let deleted_dir_prefixes: Vec<String> = deleted.iter()
            .map(|d| format!("{}/", d)).collect();
        files.retain(|f| {
            if deleted.contains(&f.path) { return false; }
            deleted_dir_prefixes.iter().all(|prefix| !f.path.starts_with(prefix.as_str()))
        });
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/bar.rs");
    }

    #[test]
    fn test_delete_all_files_produces_empty() {
        let mut files = vec![make_file("src/main.rs"), make_file("src/lib.rs")];
        let deleted: HashSet<String> = ["src".to_string()].into_iter().collect();
        let deleted_dir_prefixes: Vec<String> = deleted.iter()
            .map(|d| format!("{}/", d)).collect();
        files.retain(|f| {
            if deleted.contains(&f.path) { return false; }
            deleted_dir_prefixes.iter().all(|prefix| !f.path.starts_with(prefix.as_str()))
        });
        assert!(files.is_empty());
    }

    // ── enforce_max_files ────────────────────────────────────────────────

    #[test]
    fn enforce_max_files_no_op_when_under_limit() {
        let mut files = vec![make_file("a.rs"), make_file("b.rs")];
        enforce_max_files(&mut files);
        assert_eq!(files.len(), 2, "should not truncate when under MAX_FILES");
    }

    #[test]
    fn enforce_max_files_keeps_most_recent() {
        // Create MAX_FILES + 5 files with ascending mtimes
        let mut files: Vec<FileNode> = (0..MAX_FILES + 5)
            .map(|i| make_file_with_mtime(&format!("file_{i}.rs"), i as f64))
            .collect();
        enforce_max_files(&mut files);
        assert_eq!(files.len(), MAX_FILES);
        // All remaining files should have mtime >= 5.0 (the oldest 5 were trimmed)
        for f in &files {
            assert!(f.mtime >= 5.0, "file {} with mtime {} should have been pruned", f.path, f.mtime);
        }
    }

    // ── upsert_changed_files ─────────────────────────────────────────────

    #[test]
    fn upsert_updates_existing_file() {
        let tmp = temp_dir();
        let file_path = tmp.path().join("src").join("main.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "fn main() {}\n").unwrap();

        let mut files = vec![make_file("src/main.rs")];
        assert_eq!(files[0].lines, 10, "original has 10 lines");

        let to_reparse = vec![("src/main.rs".to_string(), file_path.clone())];
        let line_counts: HashMap<PathBuf, (u32, u32, u32, u32)> =
            [(file_path.clone(), (1, 1, 0, 0))].into_iter().collect();
        let sa_map = HashMap::new();
        let git_statuses = HashMap::new();

        upsert_changed_files(&mut files, &to_reparse, &line_counts, &sa_map, &git_statuses);
        assert_eq!(files.len(), 1, "should update in place, not add");
        assert_eq!(files[0].lines, 1, "lines should be updated to 1");
    }

    #[test]
    fn upsert_inserts_new_file() {
        let tmp = temp_dir();
        let file_path = tmp.path().join("src").join("new.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "fn new() {}\nfn other() {}\n").unwrap();

        let mut files = vec![make_file("src/main.rs")];
        let to_reparse = vec![("src/new.rs".to_string(), file_path.clone())];
        let line_counts: HashMap<PathBuf, (u32, u32, u32, u32)> =
            [(file_path.clone(), (2, 2, 0, 0))].into_iter().collect();
        let sa_map = HashMap::new();
        let git_statuses = HashMap::new();

        upsert_changed_files(&mut files, &to_reparse, &line_counts, &sa_map, &git_statuses);
        assert_eq!(files.len(), 2, "should add the new file");
        assert!(files.iter().any(|f| f.path == "src/new.rs"), "new.rs should exist");
    }

    // ── lookup_line_counts ───────────────────────────────────────────────

    #[test]
    fn lookup_line_counts_direct_hit() {
        let path = PathBuf::from("/tmp/test.rs");
        let counts: HashMap<PathBuf, (u32, u32, u32, u32)> =
            [(path.clone(), (42, 30, 5, 7))].into_iter().collect();
        assert_eq!(lookup_line_counts(&path, &counts), (42, 30, 5, 7));
    }

    #[test]
    fn lookup_line_counts_fallback_to_file_read() {
        let tmp = temp_dir();
        let file_path = tmp.path().join("fallback.rs");
        fs::write(&file_path, "line1\nline2\nline3\n").unwrap();

        let empty_counts: HashMap<PathBuf, (u32, u32, u32, u32)> = HashMap::new();
        let (total, logic, comments, blanks) = lookup_line_counts(&file_path, &empty_counts);
        assert_eq!(total, 3, "should count 3 lines from file read");
        assert_eq!(logic, 0);
        assert_eq!(comments, 0);
        assert_eq!(blanks, 0);
    }

    #[test]
    fn lookup_line_counts_missing_file_returns_zeros() {
        let empty_counts: HashMap<PathBuf, (u32, u32, u32, u32)> = HashMap::new();
        let result = lookup_line_counts(Path::new("/nonexistent/file.rs"), &empty_counts);
        assert_eq!(result, (0, 0, 0, 0));
    }

    // ── classify_changed_paths ───────────────────────────────────────────

    #[test]
    fn classify_existing_file_goes_to_reparse() {
        let tmp = temp_dir();
        let file_path = tmp.path().join("src").join("main.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "fn main() {}").unwrap();

        let expanded = vec!["src/main.rs".to_string()];
        let (to_reparse, deleted) = classify_changed_paths(tmp.path(), &expanded, 10_000_000);
        assert_eq!(to_reparse.len(), 1);
        assert_eq!(to_reparse[0].0, "src/main.rs");
        assert!(deleted.is_empty());
    }

    #[test]
    fn classify_missing_file_goes_to_deleted() {
        let tmp = temp_dir();
        let expanded = vec!["src/gone.rs".to_string()];
        let (to_reparse, deleted) = classify_changed_paths(tmp.path(), &expanded, 10_000_000);
        assert!(to_reparse.is_empty());
        assert!(deleted.contains("src/gone.rs"));
    }

    #[test]
    fn classify_skips_ignored_extensions() {
        let tmp = temp_dir();
        let file_path = tmp.path().join("data.png");
        fs::write(&file_path, "fake png data").unwrap();

        let expanded = vec!["data.png".to_string()];
        let (to_reparse, deleted) = classify_changed_paths(tmp.path(), &expanded, 10_000_000);
        assert!(to_reparse.is_empty(), "png should be ignored");
        assert!(deleted.is_empty(), "existing ignored file is not deleted");
    }

    #[test]
    fn classify_skips_oversized_files() {
        let tmp = temp_dir();
        let file_path = tmp.path().join("big.rs");
        fs::write(&file_path, "x".repeat(2000)).unwrap(); // 2000 bytes

        let expanded = vec!["big.rs".to_string()];
        let (to_reparse, _) = classify_changed_paths(tmp.path(), &expanded, 1000); // 1KB limit
        assert!(to_reparse.is_empty(), "oversized file should be skipped");
    }

    // ── expand_directory_events ──────────────────────────────────────────

    #[test]
    fn expand_directory_discovers_files_inside() {
        let tmp = temp_dir();
        let dir = tmp.path().join("src");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.rs"), "fn a() {}").unwrap();
        fs::write(dir.join("b.rs"), "fn b() {}").unwrap();

        let changed = vec!["src".to_string()];
        let expanded = expand_directory_events(tmp.path(), &changed, 10_000_000);
        assert!(expanded.len() >= 2, "should discover both .rs files: {:?}", expanded);
        assert!(expanded.iter().any(|p| p.contains("a.rs")));
        assert!(expanded.iter().any(|p| p.contains("b.rs")));
    }

    #[test]
    fn expand_directory_skips_nested_ignored_dirs() {
        // expand_single_dir starts the walker at the given dir, so top-level
        // ignored dirs (node_modules itself) are not filtered at this layer.
        // But NESTED ignored dirs (e.g., node_modules/.cache) ARE filtered.
        let tmp = temp_dir();
        let dir = tmp.path().join("vendor");
        fs::create_dir_all(dir.join("target")).unwrap(); // "target" is in IGNORED_DIRS
        fs::write(dir.join("target").join("debug.rs"), "// build output").unwrap();
        fs::write(dir.join("lib.rs"), "pub fn vendor() {}").unwrap();

        let changed = vec!["vendor".to_string()];
        let expanded = expand_directory_events(tmp.path(), &changed, 10_000_000);
        assert!(expanded.iter().any(|p| p.contains("lib.rs")), "vendor/lib.rs should be found");
        assert!(!expanded.iter().any(|p| p.contains("debug.rs")), "vendor/target/ should be filtered");
    }

    #[test]
    fn expand_non_directory_passes_through() {
        let tmp = temp_dir();
        fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();

        let changed = vec!["main.rs".to_string()];
        let expanded = expand_directory_events(tmp.path(), &changed, 10_000_000);
        assert_eq!(expanded, vec!["main.rs".to_string()]);
    }

    #[test]
    fn expand_missing_path_passes_through() {
        let tmp = temp_dir();
        let changed = vec!["gone.rs".to_string()];
        let expanded = expand_directory_events(tmp.path(), &changed, 10_000_000);
        assert_eq!(expanded, vec!["gone.rs".to_string()], "missing file still passed through for classify to handle");
    }

    // ── build_file_node ──────────────────────────────────────────────────

    #[test]
    fn build_file_node_basic() {
        let tmp = temp_dir();
        let file_path = tmp.path().join("src").join("lib.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "pub fn hello() {}\n").unwrap();

        let line_counts: HashMap<PathBuf, (u32, u32, u32, u32)> =
            [(file_path.clone(), (1, 1, 0, 0))].into_iter().collect();
        let sa_map = HashMap::new();
        let git_statuses: HashMap<String, String> =
            [("src/lib.rs".to_string(), "M".to_string())].into_iter().collect();

        let node = build_file_node("src/lib.rs", &file_path, &line_counts, &sa_map, &git_statuses);
        assert_eq!(node.path, "src/lib.rs");
        assert_eq!(node.name, "lib.rs");
        assert!(!node.is_dir);
        assert_eq!(node.lines, 1);
        assert_eq!(node.lang, "rust");
        assert_eq!(node.gs, "M");
        assert!(node.mtime > 0.0, "mtime should be set from filesystem");
    }

    #[test]
    fn build_file_node_unknown_lang() {
        let tmp = temp_dir();
        let file_path = tmp.path().join("README.txt");
        fs::write(&file_path, "Hello world\n").unwrap();

        let line_counts: HashMap<PathBuf, (u32, u32, u32, u32)> =
            [(file_path.clone(), (1, 0, 0, 0))].into_iter().collect();

        let node = build_file_node("README.txt", &file_path, &line_counts, &HashMap::new(), &HashMap::new());
        assert_eq!(node.path, "README.txt");
        assert_eq!(node.gs, "", "no git status = empty string");
    }

    // ── build_snapshot_with_graphs ───────────────────────────────────────

    #[test]
    fn build_snapshot_with_graphs_produces_valid_snapshot() {
        let tmp = temp_dir();
        let files = vec![
            make_file("src/main.rs"),
            make_file("src/lib.rs"),
            make_file("src/utils/helpers.rs"),
        ];
        let result = build_snapshot_with_graphs(tmp.path(), files, None, 50)
            .expect("should succeed");
        assert_eq!(result.snapshot.total_files, 3);
        assert_eq!(result.snapshot.total_lines, 30); // 3 * 10
    }

    #[test]
    fn build_snapshot_with_graphs_calls_on_tree_ready() {
        let tmp = temp_dir();
        let files = vec![make_file("src/main.rs")];
        let called = std::sync::atomic::AtomicBool::new(false);
        let cb = |_snap: Snapshot| {
            called.store(true, std::sync::atomic::Ordering::SeqCst);
        };
        let _ = build_snapshot_with_graphs(tmp.path(), files, Some(&cb), 50);
        assert!(called.load(std::sync::atomic::Ordering::SeqCst), "on_tree_ready should be called");
    }

    #[test]
    fn build_snapshot_with_graphs_empty_files() {
        let tmp = temp_dir();
        let result = build_snapshot_with_graphs(tmp.path(), vec![], None, 50)
            .expect("should succeed even with no files");
        assert_eq!(result.snapshot.total_files, 0);
        assert_eq!(result.snapshot.total_lines, 0);
    }

    // ── batch_line_counts ────────────────────────────────────────────────

    #[test]
    fn batch_line_counts_empty_input() {
        let result = batch_line_counts(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn batch_line_counts_real_file() {
        let tmp = temp_dir();
        let file_path = tmp.path().join("test.rs");
        fs::write(&file_path, "fn main() {\n    println!(\"hello\");\n}\n").unwrap();

        let to_reparse = vec![("test.rs".to_string(), file_path.clone())];
        let counts = batch_line_counts(&to_reparse);
        // tokei should find at least some lines
        if let Some(&(total, _, _, _)) = counts.get(&file_path) {
            assert!(total >= 2, "should count at least 2 code lines");
        }
        // tokei should find the file — if it doesn't, the test above is skipped but this catches it
        assert!(!counts.is_empty(), "tokei should find the .rs file in temp dir");
    }
}

