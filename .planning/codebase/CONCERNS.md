# Codebase Concerns

**Analysis Date:** 2026-03-14

## Tech Debt

**Unimplemented Plugin Checksum Verification:**
- Issue: `verify_checksum()` in the plugin loader reads the manifest's SHA256 hash but performs no actual verification. The function unconditionally returns `Ok(())` with a TODO comment.
- Files: `sentrux-core/src/analysis/plugin/loader.rs:145-161`
- Impact: Downloaded grammar plugins are loaded into the process (via `unsafe` FFI) without any integrity check. A corrupted or tampered `.dylib`/`.so` would execute without warning.
- Fix approach: Add `sha2` to `[dependencies]` in `sentrux-core/Cargo.toml` and implement the SHA256 comparison against `manifest.checksums`.

**`evo` Module Re-exported as `evolution` for Backward Compatibility:**
- Issue: `metrics/mod.rs` re-exports `pub use evo as evolution` because an external caller (`app/mcp_handlers_evo.rs`) still imports the old name. This creates a permanent dual-name surface for the same module.
- Files: `sentrux-core/src/metrics/mod.rs:28`
- Impact: New callers may use either name, fragmenting usage patterns. The comment calls this "backward compatibility" but the old import path is internal.
- Fix approach: Migrate `app/mcp_handlers_evo.rs` to use `crate::metrics::evo` and remove the alias.

**`eprintln!` Used as the Sole Logging Strategy:**
- Issue: The entire codebase uses raw `eprintln!` for diagnostics — there is no structured logging framework (e.g., `tracing`, `log`). This produces noisy stderr during normal operation (timing breakdowns for every scan, every resolve pass, every file tree build) that cannot be filtered by level or module.
- Files: `sentrux-core/src/analysis/scanner/mod.rs` (10+ eprintln calls), `sentrux-core/src/analysis/resolver/suffix.rs`, `sentrux-core/src/analysis/resolver/oxc.rs`, `sentrux-core/src/analysis/graph/mod.rs`, `sentrux-core/src/app/scanning.rs`, and ~15 other files.
- Impact: Timing and debug output is always emitted to stderr regardless of context (GUI mode, MCP mode, CI). No ability to suppress noise or enable verbose mode.
- Fix approach: Introduce `tracing` or `log`/`env_logger` and replace `eprintln!` with `tracing::debug!`, `tracing::warn!`, etc. Guard timing logs behind `debug` level.

**`tests2` Parallel Test Files Pattern:**
- Issue: Several modules have both `tests.rs` and `tests2.rs` side by side (e.g., `metrics/mod_tests.rs` + `metrics/mod_tests2.rs`, `layout/tests.rs` + `layout/tests2.rs`, `analysis/graph/tests.rs` + `analysis/graph/tests2.rs`, `analysis/parser/tests.rs` + `analysis/parser/tests2.rs`). The `tests2` files appear to be overflow from initial test files.
- Files: `sentrux-core/src/metrics/mod_tests2.rs`, `sentrux-core/src/layout/tests2.rs`, `sentrux-core/src/analysis/graph/tests2.rs`, `sentrux-core/src/analysis/parser/tests2.rs`, `sentrux-core/src/metrics/arch/tests2.rs`
- Impact: Test organization is inconsistent and harder to navigate. It signals tests were added without a clear placement strategy.
- Fix approach: Merge `tests2` content back into the primary `tests.rs` or extract into focused sub-modules with descriptive names.

**No `justfile` for Project Scripts:**
- Issue: There is no `justfile` defining standard project tasks (build, test, install, release). The only automation is in `.github/workflows/`. Developers must know raw `cargo` invocations.
- Files: None (missing)
- Impact: No consistent local developer workflow. Common tasks like installing plugins before tests (`cargo run -- plugin add-standard`) are undocumented at the project root.
- Fix approach: Add a `justfile` at the repository root with recipes for `build`, `test`, `release`, and `install-plugins`.

**`MAX_FILES_PER_COMMIT` Defined Twice:**
- Issue: The constant `MAX_FILES_PER_COMMIT = 50` is defined independently in both `metrics/evo/mod.rs:340` and `metrics/evo/git_walker.rs:11`. They happen to have the same value but are not shared.
- Files: `sentrux-core/src/metrics/evo/mod.rs:340`, `sentrux-core/src/metrics/evo/git_walker.rs:11`
- Impact: If one is changed, the other silently diverges. The limit affects commit processing in both the walker and the pair aggregator.
- Fix approach: Define once in `git_walker.rs` (where commits are sourced) and re-export or `pub(super)` expose it for use in `mod.rs`.

## Security Considerations

**Unsafe Dynamic Library Loading with Memory Leak:**
- Risk: Plugin grammars are loaded via `libloading` in an `unsafe` block. After loading, the `Library` is intentionally leaked via `std::mem::forget(lib)` to keep the grammar pointer alive. There is no unloading path.
- Files: `sentrux-core/src/analysis/plugin/loader.rs:167-192`
- Current mitigation: The comment explains this follows the same pattern as nvim-treesitter and Helix. The grammar function pointer is checked by name before calling.
- Recommendations: Since checksum verification is not implemented (see above), a malicious or corrupted plugin `.dylib` will execute arbitrary code in the process. Completing checksum verification is the critical missing safety layer.

**Anonymous Telemetry Sent Without Explicit Consent Prompt:**
- Risk: Usage statistics (scan counts, MCP call counts, gate runs, health grade, file count, tier, platform) are sent to `https://api.sentrux.dev/version` daily via `curl`. This happens on first run without presenting a consent prompt to the user.
- Files: `sentrux-core/src/app/update_check.rs:153-203`
- Current mitigation: `SENTRUX_NO_UPDATE_CHECK=1` env var disables it. The module doc comment compares it to VS Code / npm telemetry.
- Recommendations: Add an opt-out notice on first run (the `is_new_user()` check already exists but only sets a `new=1` flag, not a consent gate). Document the data collection in README.

**JSON Response Parsed with String Splitting Instead of a JSON Parser:**
- Risk: The update-check response is parsed by splitting on `"latest"` and then on `"` characters. Any change to the server response format (whitespace, key ordering, extra fields) silently fails or extracts garbage.
- Files: `sentrux-core/src/app/update_check.rs:185-189`
- Current mitigation: Failure is silent (the version notification is just skipped).
- Recommendations: Parse with `serde_json` — the dependency is already present. This also eliminates any risk of version string injection if the splitting logic is ever reused.

## Performance Bottlenecks

**Verbose Timing `eprintln!` on Every Scan:**
- Problem: Multiple `eprintln!` timing statements execute on every scan, even in release builds: collect_paths, count_lines (tokei), scan_files, git_status, parse_files, tree_ready, build_graphs, resolve_imports (oxc + suffix). Each statement formats and writes to stderr unconditionally.
- Files: `sentrux-core/src/analysis/scanner/mod.rs:260-410`, `sentrux-core/src/analysis/resolver/suffix.rs:123-130`, `sentrux-core/src/analysis/resolver/oxc.rs:268-278`, `sentrux-core/src/analysis/graph/mod.rs:168-178`
- Cause: No log-level gating; timing instrumentation is always-on.
- Improvement path: Gate behind `tracing::debug!` or a compile-time `cfg!(debug_assertions)` check.

**O(N²) Co-Change Pair Generation Partially Capped:**
- Problem: `count_file_pairs` in `metrics/evo/mod.rs` is O(N²) in the number of files per commit. It is capped at `MAX_FILES_PER_COMMIT = 50`, which limits worst-case to 1225 pairs per commit. But a large repo with many commits multiplies this across the entire git walk.
- Files: `sentrux-core/src/metrics/evo/mod.rs:339-356`, `sentrux-core/src/metrics/evo/git_walker.rs:11`
- Cause: Pair-counting requires checking all combinations. The 50-file cap prevents worst-case but megarepos with many medium-sized commits (10-30 files each) will still accumulate high pair-map memory.
- Improvement path: Consider a probabilistic approach (MinHash / locality-sensitive hashing) for very large repos, or add a total-pairs limit in addition to the per-commit cap.

## Fragile Areas

**`tokei` Panic Caught with `catch_unwind`:**
- Files: `sentrux-core/src/analysis/scanner/common.rs:118-126`
- Why fragile: `tokei::Languages::get_statistics()` can panic on certain inputs (directories with no recognizable source). The code wraps this in `std::panic::catch_unwind(AssertUnwindSafe(...))` and returns an empty map on panic.
- Safe modification: Any change to the `tokei` call site must preserve the `catch_unwind` wrapper. Upgrading `tokei` should be tested against edge-case directories (empty dirs, binary-only dirs).
- Test coverage: The panic path is exercised incidentally but there is no dedicated test for the `tokei` panic recovery.

**Orphaned Files in Tree Build Logged but Not Recovered:**
- Files: `sentrux-core/src/analysis/scanner/tree.rs:141-150`
- Why fragile: If `build_tree` produces orphaned files (files that did not get placed into any directory node), the code logs a `[tree] BUG:` warning but does not recover the orphaned files — they are silently dropped from the snapshot.
- Safe modification: Any change to path normalization or tree assembly logic must verify the orphaned-count check does not fire. The bug surface is the path prefix logic in `assemble_dir_node`.
- Test coverage: No test specifically exercises the orphan path.

**Dead Thread Recovery via `poll_dead_scanner` / `poll_dead_layout`:**
- Files: `sentrux-core/src/app/scanning.rs:454-482`
- Why fragile: Scanner and layout threads are respawned automatically if they die. The respawn logic drops the old channel, spawns a detached join thread, then creates new channels. If the layout thread fails to respawn, the app enters a degraded state where the error is logged but scanning never completes. No user-visible recovery UI is shown.
- Safe modification: Changes to the channel setup or thread spawning must account for the respawn paths in both `respawn_scanner_thread` and `respawn_layout_thread`.
- Test coverage: `scanning_tests.rs` covers channel draining and message ordering but does not test the respawn code path.

**Plugin Dynamic Library Lifetime Tied to Process:**
- Files: `sentrux-core/src/analysis/plugin/loader.rs:186-188`
- Why fragile: `std::mem::forget(lib)` leaks every loaded grammar library for the lifetime of the process. There is no mechanism to reload plugins without restarting. If a plugin is replaced on disk while the process is running, the old (leaked) version continues to be used.
- Safe modification: Do not attempt hot-reload of grammar plugins without redesigning the lifetime model. Attempting to unload a `libloading::Library` after `forget` was called on it is undefined behavior.
- Test coverage: None for the memory-leak or hot-reload scenario.

## Test Coverage Gaps

**Checksum Verification Code Path:**
- What's not tested: The `verify_checksum` function does nothing (TODO), so the success path is trivially tested but the actual SHA256 logic does not exist.
- Files: `sentrux-core/src/analysis/plugin/loader.rs:145-161`
- Risk: When checksum verification is eventually implemented, there will be no regression coverage for the check/fail paths.
- Priority: High (security-adjacent)

**Respawn Code Path for Scanner and Layout Threads:**
- What's not tested: `respawn_scanner_thread` and `respawn_layout_thread` in the GUI app are not covered by any test. The `scanning_tests.rs` tests cover channel message ordering but not thread death/recovery.
- Files: `sentrux-core/src/app/scanning.rs:354-482`
- Risk: A regression in the respawn logic would silently leave the app in a hung state after a thread crash.
- Priority: Medium

**Tokei Panic Recovery:**
- What's not tested: No test asserts that `count_lines_batch` returns an empty map when tokei panics.
- Files: `sentrux-core/src/analysis/scanner/common.rs:118-126`
- Risk: A tokei upgrade that changes panic behavior could break the recovery path silently.
- Priority: Medium

**Tree Orphan Path in `build_tree`:**
- What's not tested: No test verifies behavior when `file_children` has leftover entries after tree assembly.
- Files: `sentrux-core/src/analysis/scanner/tree.rs:141-150`
- Risk: Path normalization changes could silently drop files from snapshots.
- Priority: Medium

## Scaling Limits

**MAX_FILES Hard Limit Truncates Silently:**
- Current capacity: Up to 100,000 files (`MAX_FILES = 100_000` in `sentrux-core/src/analysis/scanner/common.rs:11`)
- Limit: Repositories exceeding 100,000 tracked files are silently truncated. The truncation is logged to stderr but not surfaced to the user in the UI or MCP response.
- Scaling path: Expose the truncation as a warning in the scan result / health report so callers know the analysis is partial.

**DSM Matrix Grows O(N²) in File Count:**
- Current capacity: DSM is a full NxN adjacency matrix. For large repos, this materializes the complete dependency matrix in memory.
- Limit: At 10,000 files, the matrix is 10^8 cells. At the default `bool` representation, that is ~100MB.
- Files: `sentrux-core/src/metrics/dsm/mod.rs`
- Scaling path: Switch to a sparse representation (e.g., `HashMap<(usize, usize), bool>`) for the internal matrix, or cap DSM computation at a configurable file threshold.

## Dependencies at Risk

**Windows Support Disabled:**
- Risk: The CI matrix explicitly excludes Windows with the comment "tree-sitter-scss build script incompatible with MSVC — will re-enable when upstream fixes".
- Files: `.github/workflows/ci.yml:15-16`
- Impact: Windows users are unsupported. There is no Windows CI validation, so Windows-specific breakage accumulates silently.
- Migration plan: Track the upstream `tree-sitter-scss` issue and re-enable the Windows matrix entry once fixed.

---

*Concerns audit: 2026-03-14*
