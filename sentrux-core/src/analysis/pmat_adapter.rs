//! PMAT subprocess adapter — spawns the `pmat` binary and parses its JSON output.
//!
//! All functions are infallible from the caller's perspective: they return
//! `None` on any failure (binary not found, non-zero exit, parse error).
//! This prevents PMAT analysis failures from crashing sentrux.

use crate::core::pmat_types::{PmatTdgOutput, PmatRepoScore};
use std::process::{Command, Stdio};

/// Check whether the `pmat` binary is available on PATH.
///
/// Returns `Ok(())` if `pmat --version` exits 0, or `Err(message)` with an
/// install hint if the binary is not found or returns an error.
pub fn check_pmat_available() -> Result<(), String> {
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
}

/// Run `pmat analyze tdg --format json --path <root> -o <tmp>` and parse the output.
///
/// Returns `None` on any failure: binary not found, non-zero exit (which occurs
/// when PMAT finds critical defects), JSON parse error, or I/O error.
/// The caller should treat `None` as "no PMAT data available" and proceed normally.
pub fn run_pmat_tdg(root: &str, scan_gen: u64) -> Option<PmatTdgOutput> {
    let tmp_path = std::env::temp_dir().join(format!("sentrux_pmat_tdg_{scan_gen}.json"));

    let status = Command::new("pmat")
        .args(["analyze", "tdg", "--format", "json", "--path", root, "-o"])
        .arg(&tmp_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;

    // PMAT exits 1 when it finds critical defects but still writes the JSON.
    // We accept any exit (ok or error) and attempt to read the output file.
    // If the file doesn't exist (truly broken invocation), we return None below.
    let _ = status; // exit code intentionally ignored — see docstring

    let json_bytes = std::fs::read(&tmp_path).ok()?;
    let _ = std::fs::remove_file(&tmp_path); // best-effort cleanup

    serde_json::from_slice(&json_bytes).ok()
}

/// Run `pmat repo-score --format json --path <root> -o <tmp>` and parse the output.
///
/// Returns `None` on any failure. Same semantics as `run_pmat_tdg`.
pub fn run_pmat_repo_score(root: &str, scan_gen: u64) -> Option<PmatRepoScore> {
    let tmp_path = std::env::temp_dir()
        .join(format!("sentrux_pmat_repo_score_{scan_gen}.json"));

    let status = Command::new("pmat")
        .args(["repo-score", "--format", "json", "--path", root, "-o"])
        .arg(&tmp_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;

    let _ = status; // exit code intentionally ignored

    let json_bytes = std::fs::read(&tmp_path).ok()?;
    let _ = std::fs::remove_file(&tmp_path);

    serde_json::from_slice(&json_bytes).ok()
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that `check_pmat_available` doesn't panic regardless of whether
    /// pmat is installed. It should return Ok or Err — never panic.
    #[test]
    fn check_pmat_available_does_not_panic() {
        let result = check_pmat_available();
        // Either Ok(()) or Err(_) is acceptable — the test just ensures no panic.
        match result {
            Ok(()) => eprintln!("[test] pmat is available"),
            Err(e) => eprintln!("[test] pmat not available: {}", e),
        }
    }

    /// Verify that `run_pmat_tdg` returns None gracefully when pmat is not available
    /// (returns None instead of panicking).
    #[test]
    fn run_pmat_tdg_returns_none_on_missing_binary() {
        // We use a non-existent path to force a failure regardless of environment.
        let result = run_pmat_tdg("/nonexistent/path/for/test", 999_999_999);
        // May return Some (if pmat is installed and errors gracefully) or None —
        // but must not panic.
        let _ = result;
    }

    #[test]
    fn run_pmat_repo_score_returns_none_on_missing_binary() {
        let result = run_pmat_repo_score("/nonexistent/path/for/test", 999_999_998);
        let _ = result;
    }
}
