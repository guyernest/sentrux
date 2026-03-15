# Domain Pitfalls

**Domain:** Rust desktop app — replacing core analysis engine with external library, adding git history and GSD plan overlays to an egui treemap
**Researched:** 2026-03-14
**Confidence:** HIGH (grounded in the existing codebase; LOW where PMAT's internal API surface is not yet confirmed)

---

## Critical Pitfalls

Mistakes that cause rewrites or major rework.

---

### Pitfall 1: PMAT Is Primarily a Binary Crate With an Undocumented Library Surface

**What goes wrong:** PMAT's README and public documentation describe a CLI/MCP binary, not a Rust library. If the public API surface is minimal (`pub(crate)` internals, no stable trait exports), callers are forced to either vendor a fork, call the binary via subprocess, or wrap internal structs that break across PMAT releases.

**Why it happens:** Tools designed as CLI apps commonly expose no stable `pub` module surface because their integration path is "run the binary." The WebFetch of the PMAT repo returned no information about public struct visibility, feature flags, or library-mode docs.

**Consequences:** If `pmat` lacks a `[lib]` target or its analysis types are `pub(crate)`, sentrux cannot use it as a `[dependencies]` entry at all — requiring a subprocess spawn (adds latency, kills incremental scan UX) or a complete PMAT fork.

**Warning signs:**
- `cargo add pmat` succeeds but `use pmat::...` returns no public items
- PMAT's `Cargo.toml` has no `[lib]` section or `crate-type = ["lib"]`
- docs.rs shows only the binary entrypoint, no exported types

**Prevention:** Before writing any integration code, open PMAT's `Cargo.toml` and confirm: (a) a `[lib]` section exists, (b) the analysis types (`TdgGrade`, `HealthScore`, etc.) are `pub` and re-exported from the crate root. If absent, the integration design must change to subprocess or fork. This must be the first task of the integration milestone, not an assumption.

**Phase:** Integration milestone, Task 1 (blocker — nothing else can proceed until confirmed)

---

### Pitfall 2: Dependency Version Conflicts Between PMAT and Sentrux

**What goes wrong:** Sentrux uses `git2 = "0.20"`, `tree-sitter = "0.25"`, `rayon = "1"`, `egui = "0.31"`. PMAT uses its own dependency versions. If PMAT pins different semver-incompatible versions of shared crates (especially `tree-sitter`, which has a history of breaking changes between minor versions), Cargo resolves to two copies of the same crate — or fails to unify them at all, producing linker errors when the two copies interact across the boundary.

**Why it happens:** `tree-sitter` language grammars are compiled against a specific tree-sitter ABI. Having two copies of the `tree-sitter` crate in one binary means grammars compiled for one version will silently produce garbage or panic when called through the other version's API. `tokei` is another likely collision point. PMAT is described as using a "PAIML Sovereign Stack" of internal libraries (trueno, aprender) which may pull in their own versions of `tokio`, `serde`, etc.

**Consequences:** Linker errors on first `cargo build`; or, worse, a build that succeeds but crashes at runtime when a grammar pointer is passed across the ABI boundary.

**Warning signs:**
- `cargo tree -d` shows duplicate versions of `tree-sitter`, `tokio`, or `serde`
- Build warnings about "multiple crate versions" for any crate both projects use
- Runtime panics with messages like "called `Result::unwrap()` on an `Err` value: InvalidNodeType"

**Prevention:** Run `cargo tree -d` immediately after adding PMAT to `Cargo.toml`. Resolve every duplicate before writing integration code. If `tree-sitter` versions conflict, sentrux's own tree-sitter usage (for import resolution) must be migrated to the PMAT version or PMAT's tree-sitter usage must be feature-flagged off.

**Phase:** Integration milestone, Task 2 (run cargo tree before any code changes)

---

### Pitfall 3: ScanReports Channel Contract Breaks During Incremental Migration

**What goes wrong:** `ScanReports` in `channels.rs` bundles `health`, `arch`, `evolution`, `test_gaps`, and `rules` — all produced by the scanner thread and consumed by `apply_scan_reports` on the main thread. During migration, some fields will come from PMAT and some will still come from sentrux's own metrics. If the migration replaces fields one-at-a-time without a clear contract for which source is authoritative, the app will silently mix PMAT grades with sentrux grades in the same UI frame.

**Why it happens:** The current code has a fallback in `apply_scan_reports`: `reports.health.unwrap_or_else(|| crate::metrics::compute_health(&snap))`. This looks like a safe migration shim, but it means callers must set `None` for fields they haven't migrated, and any caller that forgets to `None`-out a field will serve stale sentrux data even after PMAT integration is "complete."

**Consequences:** UI shows mixed grades (e.g. PMAT TDG letter beside sentrux coupling score) with no indication they are from different systems. Debugging this is very hard because both paths produce plausible-looking output.

**Warning signs:**
- Panel shows "A+" TDG grade from PMAT but coupling score is computed from sentrux's `HealthReport`
- Tests pass because each system produces results independently, but integration tests show inconsistent composite grades
- `evolution` field is populated by both PMAT and `metrics::evo::compute_evolution`

**Prevention:** Before the migration milestone begins, explicitly document which `ScanReports` fields will be replaced by PMAT outputs and set all pending-migration fields to `None` on day one. Never keep both the old and new computation in flight for the same field. Add a debug assertion that verifies the source tag of each report field matches the expected source for the current milestone stage.

**Phase:** Integration milestone, throughout

---

### Pitfall 4: ColorMode Enum Is Serialized to Disk — Adding Overlay Variants Breaks Saved Prefs

**What goes wrong:** `ColorMode` derives `Serialize`/`Deserialize` and is stored in user preferences via `eframe` persistence. Adding new variants (`ColorMode::GitHistory`, `ColorMode::GsdPhase`) is not backward-compatible: users with saved prefs from before the change will get a deserialization error on first launch after upgrading, and eframe will silently fall back to defaults — losing all saved preferences, not just the color mode.

**Why it happens:** `serde` with `#[serde(rename_all = "lowercase")]` will fail to deserialize an unknown variant. This affects the entire `UserPrefs` blob, not just the color mode field.

**Consequences:** Silent preference reset on upgrade. Users who customized settings (zoom, theme, scan root) lose everything. No error is shown in the UI.

**Warning signs:**
- `eframe` storage returns `None` for prefs after a version upgrade that added enum variants
- Any `#[serde(rename_all = ...)]` enum that gains new variants without `#[serde(other)]`

**Prevention:** Before adding `GitHistory` and `GsdPhase` to `ColorMode`, add `#[serde(other)]` to an existing fallback variant (e.g. `Monochrome`) or use `#[serde(default)]` on the entire prefs struct so unknown variants fall back to a safe default without nuking all other saved state. Write a test that deserializes an old prefs blob (missing the new variants) and verifies the result is a valid `UserPrefs` rather than an error.

**Phase:** Git history overlay milestone, before adding the first new ColorMode variant

---

### Pitfall 5: Git History Walk Blocks the Scanner Thread on Large Repos

**What goes wrong:** The existing `walk_git_log` runs synchronously on the scanner thread. For the git history overlay, the time window is user-selectable (15min, 1h, 1d, etc.) and could span thousands of commits on an active repo. A 90-day lookback on a repo with 50k commits will take several seconds, blocking `ScanMsg::Complete` from being delivered — stalling the treemap during what looks like a scan.

**Why it happens:** `git2::Revwalk` is synchronous. The current code uses `MAX_FILES_PER_COMMIT = 50` as a noise cap, but there is no cap on commit count. The O(N²) pair generation in `count_file_pairs` compounds this — a 90-day walk with many commits accumulates millions of pairs.

**Consequences:** UI appears frozen while the history walk runs. The `poll_dead_scanner` watchdog may not trigger because the thread is alive (just slow), but the user sees no progress indicator for history computation. On spinning disks or network-mounted repos, this becomes worse.

**Warning signs:**
- Scanner thread takes more than 3 seconds on any repo with >5k commits
- `ScanMsg::Progress` stops emitting during the git walk phase (no progress feedback)
- Memory grows unbounded during history computation on repos with many merge commits

**Prevention:** Move git history computation to a separate channel/thread from the main scanner. The overlay data (file → recent-change intensity) should be delivered as an `OverlayMsg` independent of the scan pipeline, so treemap renders immediately with PMAT grades and the git overlay fades in when ready. Apply a hard commit-count cap (not just a per-commit file cap) for the history walk, and surface the cap to the user ("showing last 1000 commits").

**Phase:** Git history overlay milestone, architecture design

---

### Pitfall 6: GSD Plan File Path Matching Against Snapshot Paths Is Brittle

**What goes wrong:** The GSD phase overlay colors files by which GSD planning phase will/has touched them. This requires matching file paths in `.planning/` YAML/markdown against file paths in the `Snapshot`. The `Snapshot` stores paths relative to the scan root (`src/lib.rs`, not `/absolute/path/to/src/lib.rs`). GSD plan files may contain absolute paths, git-root-relative paths, or paths relative to the plan file's directory. Matching them is not a simple string comparison.

**Why it happens:** Sentrux already has this problem in `analysis/git.rs` at line 152 (`strip_prefix` to convert workdir-relative paths to scan-root-relative paths). The same path normalization failure will occur with GSD plan paths, but with an extra level of indirection (the plan file itself is at an arbitrary location).

**Consequences:** Files appear uncolored in the GSD overlay because path matching silently produces zero matches. Users see a blank overlay and assume the feature is broken.

**Warning signs:**
- GSD overlay colors zero files even when the plan references known files
- Paths in plan files use absolute paths or `./` prefix that doesn't match snapshot keys
- Different behavior on macOS (case-insensitive filesystem) vs Linux

**Prevention:** Normalize all paths to scan-root-relative before matching. Implement a path normalization function that strips scan root prefix, resolves `./`, and lowercases on case-insensitive filesystems. Test the overlay with at minimum: absolute path in plan, relative path in plan, path with `./` prefix, and path with wrong case on macOS.

**Phase:** GSD phase overlay milestone, path resolution design

---

## Moderate Pitfalls

---

### Pitfall 7: Removing the Plugin System Before Confirming PMAT Covers All Three Languages

**What goes wrong:** Sentrux currently uses tree-sitter grammars loaded as dynamic plugins for language support. The plan is to remove the plugin system because PMAT "handles analysis." But if PMAT supports Rust well and TypeScript partially, removing the plugin system before verifying PMAT's TS/JS support leaves sentrux worse than before for TypeScript AWS CDK projects.

**Prevention:** Before removing `analysis/plugin/`, verify PMAT produces correct import graphs and TDG grades for a representative TypeScript project (e.g. an AWS CDK project). Do this verification before, not after, writing the removal code.

**Phase:** Integration milestone, language verification step

---

### Pitfall 8: egui Overlay Rendering With Per-File Color Lookups Per Frame

**What goes wrong:** The current `ColorMode::Heat` and `ColorMode::Churn` color assignments happen inside the renderer's per-file loop. For overlays that require a database lookup (git history intensity, GSD phase assignment), doing a `HashMap::get` per file per frame at 60fps on a 5000-file repo is 300k hash lookups/frame. The state field `frame_now_secs` and `frame_instant` already exist specifically to amortize per-frame per-file syscall costs. The same problem will occur with overlay lookups unless they are pre-computed.

**Prevention:** Compute overlay color maps once (when the overlay data changes) into a `HashMap<String, Color32>` stored in `AppState`, not re-derived per render frame. Invalidate the color map only when the overlay data changes (new scan result, different time window selected).

**Phase:** Both overlay milestones, renderer integration

---

### Pitfall 9: Dead Module Aliases Persist Into the PMAT-Integrated Codebase

**What goes wrong:** The existing `pub use evo as evolution` alias in `metrics/mod.rs` was created for `app/mcp_server/handlers_evo.rs`. When the MCP server is removed (planned), the alias becomes orphaned. If the MCP removal and PMAT integration happen in the same milestone, the alias removal gets forgotten and dead code accumulates.

**Prevention:** The MCP removal and the `evo as evolution` alias removal should be a single commit. Add a `#[deprecated]` attribute to `pub use evo as evolution` during the transition to produce a compiler warning that forces cleanup.

**Phase:** Integration milestone, MCP server removal step

---

### Pitfall 10: tokei Conflict if PMAT Uses a Different Line-Counting Strategy

**What goes wrong:** Sentrux uses `tokei = "12"` for line counting with a `catch_unwind` guard around panics. If PMAT also counts lines internally (which is likely for its health scoring), the two systems may produce different total-line counts for the same file (different language detection, different blank-line treatment). This makes comparisons between sentrux's treemap size metric (sized by LOC) and PMAT's health scoring inconsistent — a file may appear large by sentrux's count but small by PMAT's.

**Prevention:** After integration, pick one source of truth for line counts. If PMAT exposes LOC per file, prefer PMAT's counts and remove the `tokei` integration. If PMAT does not expose per-file LOC, keep `tokei` but document the discrepancy explicitly in the UI ("file size by tokei LOC; health grade by PMAT analysis").

**Phase:** Integration milestone, metric consolidation

---

## Minor Pitfalls

---

### Pitfall 11: `eprintln!` Logging Becomes Confusing With Two Analysis Systems

**What goes wrong:** Sentrux already emits verbose timing `eprintln!` for every scan phase. Adding PMAT (which likely has its own diagnostic output) doubles the noise. Users see interleaved stderr from both systems with no way to separate them.

**Prevention:** The existing concern about `eprintln!` noise is documented in CONCERNS.md. Introduce `tracing` during the integration milestone (not as a separate effort) so that both sentrux and any PMAT diagnostic output can be gated by log level.

**Phase:** Integration milestone, logging cleanup (parallel with API integration)

---

### Pitfall 12: `MAX_FILES_PER_COMMIT` Defined in Two Places — Git History Overlay Adds a Third

**What goes wrong:** `MAX_FILES_PER_COMMIT = 50` is already defined in both `metrics/evo/mod.rs` and `metrics/evo/git_walker.rs`. If the git history overlay introduces its own git walk (for the selectable time window), it will be tempted to define its own copy of this constant rather than reusing the existing one.

**Prevention:** Consolidate to one definition before building the git history overlay. Define `MAX_FILES_PER_COMMIT` once in `git_walker.rs` with `pub(crate)` visibility. The overlay's git walk should import it rather than redefining.

**Phase:** Git history overlay milestone, before writing the new git walk

---

### Pitfall 13: Silent Truncation at `MAX_FILES = 100_000` Is Not Surfaced in Overlay Mode

**What goes wrong:** The scanner silently truncates at 100,000 files. In overlay mode (git history, GSD phases), a truncated snapshot means some files have overlay data in the plan but no corresponding node in the treemap. The overlay will appear to have gaps (files referenced in git history with no color in the treemap) that are not explained to the user.

**Prevention:** When an overlay is active, check if `snapshot.total_files` equals `MAX_FILES` and show a warning in the status bar: "Repository truncated to 100,000 files — overlay may be incomplete."

**Phase:** Both overlay milestones, status bar integration

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| PMAT library integration | PMAT has no stable `[lib]` target | Verify `Cargo.toml` and `pub` surface before writing any code |
| Cargo dependency unification | `tree-sitter` version conflict | Run `cargo tree -d` before any integration code |
| ScanReports migration | Mixed PMAT + sentrux grades in same frame | Explicit source tagging; `None`-out replaced fields on day one |
| ColorMode enum extension | Saved prefs deserialization breaks | Add `#[serde(other)]` fallback before adding new variants |
| Git history walk | Blocks scanner thread on large repos | Separate thread + independent `OverlayMsg` channel |
| GSD plan path matching | Zero matches due to path normalization | Path normalization utility with explicit test cases |
| Language support removal | Plugin removal before TS/JS verified | Verify PMAT output on TypeScript project first |
| Overlay renderer integration | Per-frame per-file hash lookups at 60fps | Pre-compute overlay color map into `AppState`, invalidate on data change |
| MCP server removal | `evo as evolution` alias left as dead code | Remove alias in same commit as MCP server code |

---

## Sources

- Codebase analysis: `sentrux-core/src/app/channels.rs` — ScanReports structure and fallback logic
- Codebase analysis: `sentrux-core/src/layout/types.rs` — ColorMode serialization with `serde(rename_all)`
- Codebase analysis: `sentrux-core/src/metrics/evo/git_walker.rs` — synchronous git2 revwalk
- Codebase analysis: `sentrux-core/src/analysis/git.rs` — path normalization via `strip_prefix`
- Codebase analysis: `sentrux-core/src/app/state.rs` — `frame_now_secs` / `frame_instant` amortization pattern
- Codebase analysis: `.planning/codebase/CONCERNS.md` — eprintln noise, MAX_FILES_PER_COMMIT duplication, tokei panic
- PMAT repository fetch (rate-limited): Primary design is CLI/binary; library surface unconfirmed — **LOW confidence on PMAT API claims**
- Project context: `.planning/PROJECT.md` — milestone structure and integration constraints
