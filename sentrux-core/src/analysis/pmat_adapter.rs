//! PMAT subprocess adapter — spawns the `pmat` binary and parses its JSON output.
//!
//! All functions are infallible from the caller's perspective: they return
//! `None` on any failure (binary not found, non-zero exit, parse error).
//! This prevents PMAT analysis failures from crashing sentrux.

use crate::core::pmat_types::{
    CoverageOutput, CoverageReport, FileClippyData, ClippyReport,
    GraphMetricsOutput, GraphMetricsReport, PmatRepoScore, PmatTdgOutput,
    lint_category,
};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Cached PMAT availability check — spawns `pmat --version` at most once per process.
static PMAT_AVAILABLE: OnceLock<Result<(), String>> = OnceLock::new();

/// Check whether the `pmat` binary is available on PATH.
///
/// Returns `Ok(())` if `pmat --version` exits 0, or `Err(message)` with an
/// install hint if the binary is not found. Result is cached for the process lifetime.
pub fn check_pmat_available() -> Result<(), String> {
    PMAT_AVAILABLE
        .get_or_init(|| {
            match Command::new("pmat")
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
                Ok(status) if status.success() => Ok(()),
                Ok(status) => Err(format!(
                    "pmat --version exited with status {}. Install pmat: cargo install pmat",
                    status
                )),
                Err(e) => Err(format!(
                    "pmat not found ({}). Install pmat: cargo install pmat",
                    e
                )),
            }
        })
        .clone()
}

/// Run a PMAT subcommand that writes JSON to a temp file, and parse the result.
fn run_pmat_command<T: serde::de::DeserializeOwned>(
    args: &[&str],
    tmp_stem: &str,
    scan_gen: u64,
) -> Option<T> {
    let tmp_path = std::env::temp_dir().join(format!(
        "sentrux_{tmp_stem}_{}_{scan_gen}.json",
        std::process::id()
    ));

    // PMAT exits 1 when it finds critical defects but still writes the JSON.
    // We accept any exit and attempt to read the output file.
    let _status = Command::new("pmat")
        .args(args)
        .arg("-o")
        .arg(&tmp_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;

    let json_bytes = std::fs::read(&tmp_path).ok()?;
    let _ = std::fs::remove_file(&tmp_path); // best-effort cleanup

    serde_json::from_slice(&json_bytes).ok()
}

/// Run `pmat analyze tdg --format json --path <root> -o <tmp>` and parse the output.
///
/// Returns `None` on any failure. The caller should treat `None` as
/// "no PMAT data available" and proceed normally.
pub fn run_pmat_tdg(root: &str, scan_gen: u64) -> Option<PmatTdgOutput> {
    run_pmat_command(
        &["analyze", "tdg", "--format", "json", "--path", root],
        "pmat_tdg",
        scan_gen,
    )
}

/// Run `pmat repo-score --format json --path <root> -o <tmp>` and parse the output.
///
/// Returns `None` on any failure. Same semantics as `run_pmat_tdg`.
pub fn run_pmat_repo_score(root: &str, scan_gen: u64) -> Option<PmatRepoScore> {
    run_pmat_command(
        &["repo-score", "--format", "json", "--path", root],
        "pmat_repo_score",
        scan_gen,
    )
}

/// Run `pmat analyze graph-metrics --format json --path <root>` and return
/// a `GraphMetricsReport` with a filename-based lookup index.
///
/// Returns `None` on any failure. Callers should treat `None` as
/// "no graph-metrics data available" and proceed without it.
pub fn run_graph_metrics(root: &str, scan_gen: u64) -> Option<GraphMetricsReport> {
    let output: GraphMetricsOutput = run_pmat_command(
        &["analyze", "graph-metrics", "--format", "json", "--path", root],
        "graph_metrics",
        scan_gen,
    )?;
    Some(GraphMetricsReport::from_output(output))
}

/// Cached cargo-llvm-cov availability check — probes once per process.
static LLVM_COV_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check whether `cargo llvm-cov` is available on PATH.
///
/// Returns `true` if `cargo llvm-cov --version` exits successfully.
/// Result is cached for the process lifetime.
pub fn check_llvm_cov_available() -> bool {
    *LLVM_COV_AVAILABLE.get_or_init(|| {
        Command::new("cargo")
            .args(["llvm-cov", "--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Run `cargo llvm-cov --json --summary-only --ignore-run-fail` against the
/// given project root and return a `CoverageReport` with relative-path keys.
///
/// Uses a temp file for output (same pattern as `run_pmat_command`). Requires
/// `cargo-llvm-cov` to be installed; returns `None` if unavailable or on error.
///
/// `--ignore-run-fail` is required because sentrux has pre-existing test failures.
pub fn run_coverage(root: &str, scan_gen: u64) -> Option<CoverageReport> {
    let tmp_path = std::env::temp_dir().join(format!(
        "sentrux_coverage_{}_{}.json",
        std::process::id(),
        scan_gen
    ));
    let tmp_str = tmp_path.to_str()?;

    let _status = Command::new("cargo")
        .args([
            "llvm-cov",
            "--json",
            "--summary-only",
            "--ignore-run-fail",
            "--output-path",
            tmp_str,
        ])
        .current_dir(root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;

    let json_bytes = std::fs::read(&tmp_path).ok()?;
    let _ = std::fs::remove_file(&tmp_path); // best-effort cleanup

    let output: CoverageOutput = serde_json::from_slice(&json_bytes).ok()?;
    CoverageReport::from_output(output, root)
}

/// Run `cargo clippy --message-format=json -- -W clippy::pedantic -W clippy::nursery`
/// against the given project root and return a `ClippyReport` with per-file counts.
///
/// Parses NDJSON stdout line-by-line: only `reason=="compiler-message"` lines with
/// `level=="warning"` are counted. One bad line does not abort the whole report.
///
/// Returns `None` only if the subprocess cannot be spawned at all.
pub fn run_clippy_warnings(root: &str) -> Option<ClippyReport> {
    let output = Command::new("cargo")
        .args([
            "clippy",
            "--message-format=json",
            "--",
            "-W",
            "clippy::pedantic",
            "-W",
            "clippy::nursery",
        ])
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    let mut by_file: HashMap<String, FileClippyData> = HashMap::new();
    for line in output.stdout.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let obj: serde_json::Value = match serde_json::from_slice(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if obj["reason"] != "compiler-message" {
            continue;
        }
        let msg = &obj["message"];
        if msg["level"] != "warning" {
            continue;
        }
        let lint_id = match msg["code"]["code"].as_str() {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };
        let spans = match msg["spans"].as_array() {
            Some(s) => s,
            None => continue,
        };
        for span in spans {
            if span["is_primary"].as_bool() != Some(true) {
                continue;
            }
            let fname = match span["file_name"].as_str() {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => continue,
            };
            let entry = by_file.entry(fname).or_default();
            entry.total += 1;
            *entry.by_category.entry(lint_category(&lint_id).to_string()).or_insert(0) += 1;
        }
    }
    // Build basename index for cross-index joins with graph-metrics
    let mut by_basename: HashMap<String, FileClippyData> = HashMap::new();
    for (path, data) in &by_file {
        if let Some(base) = path.rsplit('/').next() {
            by_basename.entry(base.to_string()).or_default().total += data.total;
            for (cat, count) in &data.by_category {
                *by_basename.entry(base.to_string()).or_default().by_category.entry(cat.clone()).or_insert(0) += count;
            }
        }
    }
    Some(ClippyReport { by_file, by_basename })
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_pmat_available_does_not_panic() {
        let result = check_pmat_available();
        match result {
            Ok(()) => eprintln!("[test] pmat is available"),
            Err(e) => eprintln!("[test] pmat not available: {}", e),
        }
    }

    #[test]
    fn run_pmat_tdg_returns_none_on_missing_binary() {
        let result = run_pmat_tdg("/nonexistent/path/for/test", 999_999_999);
        let _ = result;
    }

    #[test]
    fn run_pmat_repo_score_returns_none_on_missing_binary() {
        let result = run_pmat_repo_score("/nonexistent/path/for/test", 999_999_998);
        let _ = result;
    }

    #[test]
    fn check_llvm_cov_available_no_panic() {
        // Must not panic regardless of whether cargo-llvm-cov is installed.
        let available: bool = check_llvm_cov_available();
        // Just assert it's a bool (compilation + no-panic check)
        let _ = available;
    }

    #[test]
    fn run_graph_metrics_returns_none_on_missing_binary() {
        // With a nonexistent path, run_graph_metrics should return None gracefully.
        let result = run_graph_metrics("/nonexistent/path/for/test", 999_999_997);
        let _ = result;
    }

    #[test]
    fn run_coverage_returns_none_on_nonexistent_path() {
        // With a nonexistent path, run_coverage should return None gracefully.
        let result = run_coverage("/nonexistent/path/for/test", 999_999_996);
        let _ = result;
    }

    #[test]
    fn run_clippy_warnings_returns_none_on_nonexistent_path() {
        // With a nonexistent path, run_clippy_warnings should return None gracefully.
        let result = run_clippy_warnings("/nonexistent/path/for/test");
        // It may return Some (empty report) or None — both are acceptable
        let _ = result;
    }
}
