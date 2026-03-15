//! Integration tests verifying that unsupported file types are silently skipped.
//!
//! Satisfies Wave 0 requirement from VALIDATION.md:
//! "scan directory with .py/.go files without panic."

use sentrux_core::analysis::lang_registry::LANG_UNKNOWN;
use sentrux_core::analysis::scanner;
use sentrux_core::analysis::scanner::common::ScanLimits;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Create a uniquely-named temporary directory with Python and Go files.
fn make_temp_dir_with_unknown_files() -> PathBuf {
    let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("sentrux_test_unknown_{}", n));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("Failed to create temp dir");
    fs::write(dir.join("hello.py"), "def hello():\n    print('hello')\n")
        .expect("Failed to write .py file");
    fs::write(dir.join("main.go"), "package main\n\nfunc main() {}\n")
        .expect("Failed to write .go file");
    dir
}

/// RAII guard that removes the temp dir on drop (even on panic).
struct TempDirGuard(PathBuf);
impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn unknown_extensions_silently_skipped() {
    let dir = make_temp_dir_with_unknown_files();
    let _guard = TempDirGuard(dir.clone());

    let limits = ScanLimits {
        max_file_size_kb: 1024,
        max_parse_size_kb: 512,
        max_call_targets: 10,
    };

    // Must not panic or error
    let result = scanner::scan_directory(dir.to_str().unwrap(), None, None, &limits)
        .expect("scan_directory must not fail for unknown extensions");

    // Verify unknown files are present but not structurally parsed
    let flat = sentrux_core::core::snapshot::flatten_files_ref(&result.snapshot.root);

    for file in &flat {
        if file.lang == LANG_UNKNOWN {
            assert!(
                file.sa.is_none(),
                "File {:?} should not have structural analysis",
                file.name
            );
            assert_eq!(
                file.funcs, 0,
                "File {:?} should have funcs=0 since it was not parsed",
                file.name
            );
        }
    }
}
