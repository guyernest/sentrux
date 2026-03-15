//! Integration tests verifying that unsupported file types are silently skipped.
//!
//! Satisfies Wave 0 requirement from VALIDATION.md:
//! "scan directory with .py/.go files without panic."

use sentrux_core::analysis::scanner;
use sentrux_core::analysis::scanner::common::ScanLimits;
use std::fs;
use std::path::PathBuf;

/// Create a temporary directory with Python and Go files.
/// Returns the path; caller is responsible for cleaning up.
fn make_temp_dir_with_unknown_files() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sentrux_test_unknown_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    fs::create_dir_all(&dir).expect("Failed to create temp dir");
    fs::write(dir.join("hello.py"), "def hello():\n    print('hello')\n")
        .expect("Failed to write .py file");
    fs::write(dir.join("main.go"), "package main\n\nfunc main() {}\n")
        .expect("Failed to write .go file");
    dir
}

#[test]
fn unknown_extensions_do_not_panic() {
    let dir = make_temp_dir_with_unknown_files();
    let limits = ScanLimits {
        max_file_size_kb: 1024,
        max_parse_size_kb: 512,
        max_call_targets: 10,
    };
    // Must not panic — this is the primary assertion.
    let result = scanner::scan_directory(
        dir.to_str().unwrap(),
        None,
        None,
        &limits,
    );
    let _ = fs::remove_dir_all(&dir);
    assert!(result.is_ok(), "scan_directory must not return an error for unknown extensions");
}

#[test]
fn unknown_extensions_not_parsed_structurally() {
    let dir = make_temp_dir_with_unknown_files();
    let limits = ScanLimits {
        max_file_size_kb: 1024,
        max_parse_size_kb: 512,
        max_call_targets: 10,
    };
    let result = scanner::scan_directory(
        dir.to_str().unwrap(),
        None,
        None,
        &limits,
    ).expect("scan_directory must not fail");

    let _ = fs::remove_dir_all(&dir);

    // Flatten file nodes from the snapshot tree
    let flat = sentrux_core::core::snapshot::flatten_files_ref(&result.snapshot.root);

    for file in &flat {
        // Python and Go must not be parsed (sa must be None and funcs must be 0)
        if file.name.ends_with(".py") || file.name.ends_with(".go") {
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
            assert_eq!(
                file.lang, "unknown",
                "File {:?} lang should be 'unknown', got '{}'",
                file.name,
                file.lang
            );
        }
    }
}
