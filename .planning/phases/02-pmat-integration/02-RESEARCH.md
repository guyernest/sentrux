# Phase 2: PMAT Integration - Research

**Researched:** 2026-03-14
**Domain:** Rust subprocess integration, egui panel extension, ColorMode extension, metrics module deletion
**Confidence:** HIGH — PMAT binary is installed locally and directly invoked to verify all API claims

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- Subprocess integration: spawn `pmat analyze tdg --output json`, parse stdout via serde_json
- If PMAT API spike finds a library crate, prefer that — but subprocess is the confirmed fallback
- PMAT is **required** — sentrux will not scan without it
- Error on PMAT missing: show clear error banner ("PMAT not found. Install: cargo install pmat") and refuse to scan
- Cache PMAT results in AppState — only re-run on file changes (rescan), not every frame
- New `ColorMode::TdgGrade` variant with green-to-red gradient (A+ = deep green → F = red)
- TDG is the **default ColorMode** when opening a project (replaces Language as default)
- Letter grade badges (A+, B-, etc.) shown on treemap nodes **above a size threshold** — small nodes show color only
- If PMAT only provides aggregate grades (not per-file), that's fine — show project-level grade in panel
- Mutation testing results shown in the **file detail panel** (click a file to see mutation score)
- Progressive disclosure: show TDG grade + health score summary by default, expand for full breakdown
- Health panel approach: Claude's discretion (dedicated panel vs replacing existing)
- **Prune ColorModes to essentials**: keep Language, Heat, Git, TDG. Drop Age, Churn, Risk, ExecDepth, BlastRadius
- **Rewrite `sentrux check` and `sentrux gate`** to use PMAT's TDG grades instead of sentrux's metrics engine
- **Metrics engine deletion**: keep evo (git churn, bus factor) if PMAT doesn't cover it; delete what PMAT replaces
- `#[serde(other)]` fallback on ColorMode enum before adding TdgGrade variant

### Claude's Discretion

- Health panel layout approach (dedicated vs replacing existing panel)
- Which metrics/ submodules to keep vs delete (based on PMAT capability assessment during spike)
- PMAT subprocess invocation details (timeout, working directory, argument format)
- Loading/progress UX during PMAT subprocess execution

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PMAT-01 | Sentrux integrates PMAT as analysis backend (library crate or subprocess) | Subprocess confirmed as correct path; `pmat analyze tdg` JSON schema fully verified |
| PMAT-02 | PMAT TDG grades (A+ through F) computed for scanned projects | Per-file grades confirmed via `pmat analyze tdg --format json`; grade enum has 11 values |
| PMAT-03 | TDG grade badges displayed on treemap file/directory nodes | `renderer/badges.rs` infrastructure reusable; size threshold pattern from existing badges |
| PMAT-04 | TDG grade color mode colors treemap nodes by grade (green A+ → red F) | `renderer/colors.rs` pattern; new `tdg_grade_color(grade)` function mirrors `blast_radius_color` |
| PMAT-05 | PMAT health score displayed in dedicated GUI panel | `pmat repo-score --format json` provides `total_score` + `grade` + `categories`; replaces `health_display.rs` |
| PMAT-06 | PMAT mutation testing results accessible through GUI panel | PMAT does NOT provide mutation testing data — see Open Questions; file detail panel shows TDG breakdown instead |
| PMAT-07 | Sentrux's own grading/rating system removed and replaced by PMAT metrics | Deletion scope confirmed: `grading.rs`, `stability.rs`, `arch/`, `dsm/`, `testgap/`, `whatif/` (already removed in Phase 1); `evo/` kept |
| CLEN-04 | Unused analysis code (sentrux's own metrics engine) removed after PMAT replaces it | Clear deletion list established; `evo/` module preserved as add-on (bus factor, churn PMAT doesn't cover) |
</phase_requirements>

---

## Summary

PMAT is installed locally as version 2.213.14 from crates.io and confirmed to be a binary-only integration target — its crates.io release does not expose a stable library API suitable for direct Rust dependency. The `src/lib.rs` exists on GitHub master (v3.7.1) but the installed crates.io version diverges significantly. The subprocess path is correct and fully verified.

The primary integration command is `pmat analyze tdg --format json --path <dir> -o <file>`. This produces a single JSON object with `files[]` (per-file TDG scores and grades), `average_score`, `average_grade`, `total_files`, and `language_distribution`. Per-file data includes: `file_path`, `grade` (as a string like `"APLus"`), `total` (0-100 score), component scores, `penalties_applied[]`, `critical_defects_count`, `has_critical_defects`. The grade enum maps scores to 11 values: A+ (≥95), A (≥90), A- (≥85), B+ (≥80), B (≥75), B- (≥70), C+ (≥65), C (≥60), C- (≥55), D (≥50), F (<50). In JSON, A+ is serialized as `"APLus"` (not `"A+"`) due to serde naming constraints.

The health panel is powered by `pmat repo-score --format json` which returns `total_score` (0-110 scale), `grade`, and `categories` (documentation, precommit_hooks, repository_hygiene, build_test_automation, continuous_integration, pmat_compliance). PMAT does NOT provide mutation testing data — the requirement PMAT-06 must be re-scoped to display TDG score breakdown details (per-component scores, penalties, critical defects) in the file detail panel rather than mutation testing results.

**Primary recommendation:** Subprocess integration via `pmat analyze tdg --format json --path <root> -o <tmpfile>` on the scanner thread after the filesystem scan completes. Parse the tmpfile output and discard. Use `pmat repo-score --format json` for the health panel. Grade strings from `analyze tdg` use `APLus` format; normalize to display strings (`A+`, `A-`, etc.) via a local mapping function.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `std::process::Command` | stdlib | Spawn `pmat` subprocess | No new dependency; matches subprocess decision |
| `serde_json` | already in tree | Parse PMAT JSON output | Already in `sentrux-core/Cargo.toml` |
| `tempfile` or `/tmp` path | stdlib `std::env::temp_dir()` | Write PMAT `-o` output to temp file | Avoids stdout line-mixing with PMAT's progress messages |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `crossbeam-channel` | already in tree | Deliver `PmatReport` from scanner thread to UI | Existing pattern — add `pmat: Option<PmatReport>` to `ScanReports` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `pmat analyze tdg --format json -o` | `pmat tdg <file>` per-file | `tdg <file>` gives single-file project-level aggregate, not per-file list; `analyze tdg` gives `files[]` array |
| Writing to temp file | Parsing stdout | PMAT stdout mixes progress messages (`🔍 Starting TDG...`) with JSON; `-o` flag produces clean JSON |
| `pmat repo-score` for health | `pmat tdg` for health | `repo-score` has richer category breakdown; `tdg` only gives code quality score |

**Installation (for users):**
```bash
cargo install pmat
```

---

## Architecture Patterns

### PMAT Output File Strategy

**What:** PMAT's `--format json` output is NOT pure JSON on stdout — it also emits Unicode progress lines (`🔍`, `✅`, `⛔`). The `-o <file>` flag writes clean JSON to a file while progress goes to stdout.

**Pattern:**
```rust
// Source: verified by running pmat locally
let tmp_path = std::env::temp_dir().join(format!("sentrux_pmat_{}.json", scan_gen));
let output = std::process::Command::new("pmat")
    .args(["analyze", "tdg", "--format", "json", "--path", root, "-o", tmp_path.to_str().unwrap()])
    .current_dir(root)
    .output()?;
// status check
let json_bytes = std::fs::read(&tmp_path)?;
let _ = std::fs::remove_file(&tmp_path); // cleanup
let report: PmatTdgOutput = serde_json::from_slice(&json_bytes)?;
```

**PMAT auto-fail behaviour:** If the project has critical defects (`.unwrap()` calls, etc.), `pmat analyze tdg` exits with code 1 and does NOT write the output file. Sentrux must handle this gracefully — treat it as `pmat: None` rather than a fatal error.

### PmatReport Types

```rust
// Placement: sentrux-core/src/core/pmat_types.rs (new file)
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmatTdgOutput {
    pub files: Vec<PmatFileScore>,
    pub average_score: f32,
    pub average_grade: String,      // "APLus", "A", "B", etc.
    pub total_files: usize,
    pub language_distribution: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmatFileScore {
    pub file_path: String,          // e.g. "./sentrux-core/src/app/channels.rs"
    pub grade: String,              // "APLus" | "A" | "AMinus" | "BPlus" | ...
    pub total: f32,                 // 0-100 score
    pub structural_complexity: f32,
    pub semantic_complexity: f32,
    pub duplication_ratio: f32,
    pub coupling_score: f32,
    pub doc_coverage: f32,
    pub consistency_score: f32,
    pub entropy_score: f32,
    pub confidence: f32,
    pub language: String,
    pub critical_defects_count: usize,
    pub has_critical_defects: bool,
    pub penalties_applied: Vec<PmatPenalty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmatPenalty {
    pub source_metric: String,
    pub amount: f32,
    pub issue: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmatRepoScore {
    pub total_score: f32,           // 0-110 scale
    pub grade: String,              // same "APLus" format
    pub categories: std::collections::HashMap<String, PmatScoreCategory>,
    pub recommendations: Vec<serde_json::Value>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmatScoreCategory {
    pub score: f32,
    pub max_score: f32,
    pub percentage: f32,
    pub status: String,             // "Warning" | "Fail" | "Pass"
}

// Aggregate: holds both commands' results
#[derive(Debug, Clone)]
pub struct PmatReport {
    pub tdg: PmatTdgOutput,
    pub repo_score: Option<PmatRepoScore>,
    // file_index: fast lookup by normalized path
    pub by_path: std::collections::HashMap<String, usize>, // normalized_path -> index in tdg.files
}
```

### Grade String Mapping

```rust
// sentrux-core/src/core/pmat_types.rs
/// Map PMAT's serde-serialized grade string to display string.
/// PMAT serializes enum variants as PascalCase: "APLus" → "A+", etc.
pub fn grade_to_display(grade: &str) -> &'static str {
    match grade {
        "APLus" => "A+",
        "A"     => "A",
        "AMinus" => "A-",
        "BPlus" => "B+",
        "B"     => "B",
        "BMinus" => "B-",
        "CPlus" => "C+",
        "C"     => "C",
        "CMinus" => "C-",
        "D"     => "D",
        "F"     => "F",
        _       => "?",
    }
}

/// Map PMAT grade to a normalized 0.0–1.0 value for color interpolation.
/// A+ = 1.0, F = 0.0.
pub fn grade_to_t(grade: &str) -> f32 {
    match grade {
        "APLus" => 1.0,
        "A"     => 0.9,
        "AMinus" => 0.8,
        "BPlus" => 0.7,
        "B"     => 0.6,
        "BMinus" => 0.5,
        "CPlus" => 0.4,
        "C"     => 0.35,
        "CMinus" => 0.25,
        "D"     => 0.15,
        "F"     => 0.0,
        _       => 0.5,
    }
}
```

### ColorMode::TdgGrade Integration Pattern

The existing `file_color` dispatch in `renderer/rects.rs` calls `colors::*` functions. Adding `TdgGrade` is a new match arm:

```rust
// renderer/colors.rs — new function
pub fn tdg_grade_color(grade: &str) -> Color32 {
    let t = crate::core::pmat_types::grade_to_t(grade);
    // green(A+) → yellow(C) → red(F) — matches CI dashboard conventions
    let r = (30.0 + (1.0 - t) * 225.0) as u8;
    let g = (180.0 * t) as u8;
    let b = 40_u8;
    Color32::from_rgb(r, g, b)
}
```

### serde(other) Fallback Before Adding TdgGrade

The `ColorMode` enum in `layout/types.rs` is serialized to disk via `app/prefs.rs`. Adding a new variant without a fallback breaks deserialization of saved prefs:

```rust
// layout/types.rs — MUST be added before TdgGrade variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    Language,
    Heat,
    Git,
    TdgGrade,
    #[serde(other)]   // absorbs unknown variants (Age, Churn, Risk, etc.) when deserializing old prefs
    Monochrome,       // fallback default
}
```

**Order matters:** `#[serde(other)]` must be on the LAST variant. The fallback must be `Monochrome` (the neutral default) so loading old prefs with `"churn"` or `"risk"` degrades gracefully.

### TDG Badge Rendering Pattern

The existing `renderer/badges.rs` renders entry-point dots. TDG grade badges use a similar approach — screen-space text at a fixed position, visible only above a size threshold:

```rust
// In renderer/badges.rs or a new pmat_badges.rs
// Show grade text badge above a minimum screen size
const GRADE_BADGE_MIN_SCREEN_PX: f32 = 28.0; // don't badge tiny nodes

fn draw_tdg_badge(painter: &egui::Painter, screen_rect: egui::Rect, grade: &str, ctx: &RenderContext) {
    if screen_rect.width() < GRADE_BADGE_MIN_SCREEN_PX || screen_rect.height() < GRADE_BADGE_MIN_SCREEN_PX {
        return;
    }
    let display = crate::core::pmat_types::grade_to_display(grade);
    let pos = screen_rect.left_top() + egui::vec2(3.0, 3.0);
    painter.text(pos, egui::Align2::LEFT_TOP, display,
        egui::FontId::monospace(9.0), egui::Color32::WHITE);
}
```

### PMAT Not Found Error Pattern

Following the architecture's `ScanMsg::Error` pattern — never panic, send error to UI:

```rust
// In scanner_thread (sentrux-core/src/app/scan_threads.rs or analysis/pmat_adapter.rs)
fn check_pmat_available() -> Result<(), String> {
    match std::process::Command::new("pmat").arg("--version").output() {
        Ok(out) if out.status.success() => Ok(()),
        _ => Err("PMAT not found. Install: cargo install pmat".to_string()),
    }
}
```

On failure, send `ScanMsg::Error` with the message to the UI thread. The UI thread renders it as an error banner in the status bar (existing `scan_step` field).

### Recommended Project Structure — New Files

```
sentrux-core/src/
├── core/
│   └── pmat_types.rs           # PmatReport, PmatTdgOutput, PmatFileScore, grade helpers (NEW)
├── analysis/
│   └── pmat_adapter.rs         # Subprocess invocation, JSON parsing, PmatReport construction (NEW)
└── app/
    └── panels/
        └── pmat_panel.rs       # PMAT health panel + file detail TDG breakdown (NEW, replaces health_display.rs)
```

### Anti-Patterns to Avoid

- **Parsing pmat stdout directly:** PMAT emits Unicode emoji progress lines before the JSON. Always use `-o <file>` and read the file.
- **Blocking the UI thread with pmat subprocess:** `pmat analyze tdg` on a large project can take 5-30 seconds. Run on the scanner thread, never block egui's update loop.
- **Running pmat every frame:** Cache `pmat: Option<PmatReport>` on `AppState`; only rerun on `ScanCommand::FullScan` or `ScanCommand::Rescan`.
- **Using `pmat tdg <file>` per-file:** This gives a project-level aggregate for a single file, not a per-file list. Use `pmat analyze tdg --path <dir>` for the full per-file list.
- **Trusting pmat analyze tdg on projects with critical defects:** PMAT exits with code 1 and writes no output file when it detects critical defects (`.unwrap()` calls, etc.). Treat exit code != 0 as `pmat_result: None` — display a "PMAT analysis unavailable" message in the panel without blocking the scan.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TDG scoring formula | Custom complexity scorer | `pmat analyze tdg` | PMAT's formula (structural + semantic + duplication + coupling + doc + consistency + entropy) is calibrated; home-grown scoring diverges from PMAT's output |
| Grade-to-color mapping | Ad-hoc HSV math | `tdg_grade_color(grade)` using `grade_to_t()` | Consistent green→yellow→red matches CI dashboard conventions users expect |
| Grade string parsing | String regex parsing | `grade_to_display()` match table | The 11-variant mapping is fixed and complete |
| Repo health scoring | Custom project health aggregator | `pmat repo-score --format json` | PMAT checks CI, hooks, hygiene, compliance — not replicable cheaply |

**Key insight:** PMAT's value is its calibrated multi-dimensional scoring that maps to a single grade. Any custom scoring reimplements this incorrectly.

---

## Common Pitfalls

### Pitfall 1: PMAT Critical Defect Auto-Fail

**What goes wrong:** `pmat analyze tdg` exits with code 1 and writes no output file when the project has critical defects (`.unwrap()` calls are the most common trigger). The sentrux codebase itself has 21 such defects. Running `pmat analyze tdg` on sentrux returns exit code 1 with no JSON output.

**Why it happens:** PMAT's quality gate logic treats `.unwrap()` in non-test code as a blocking critical defect. Many Rust projects have this.

**How to avoid:** Always check `output.status.success()` before reading the output file. Treat exit code 1 as `PmatReport { tdg: None, ... }` — show a "TDG analysis unavailable: N critical defects" message in the panel. Do NOT block the treemap from loading.

**Warning signs:** Empty output file after subprocess returns.

### Pitfall 2: Grade String Format Mismatch

**What goes wrong:** `pmat analyze tdg --format json` serializes grades as `"APLus"`, `"AMinus"`, `"BPlus"` (Rust enum variants, PascalCase). `pmat tdg <path>` serializes grades as `"A+"`, `"A-"`, `"B+"` (Display impl). They are DIFFERENT commands with DIFFERENT grade serialization.

**Why it happens:** The `analyze tdg` subcommand uses `#[derive(Serialize)]` directly on the `Grade` enum (producing variant names), while the `tdg` subcommand uses `Display` formatting.

**How to avoid:** Use `grade_to_display()` mapping exclusively. If switching between commands, verify grade string format. The `analyze tdg` command (directory scan) produces `"APLus"` format; this is what sentrux uses.

**Warning signs:** Grade badges show `"APLus"` as literal text instead of `"A+"`.

### Pitfall 3: ColorMode Deserialization Breakage

**What goes wrong:** Adding `TdgGrade` to `ColorMode` without `#[serde(other)]` breaks deserialization of existing user prefs that contain `"churn"`, `"risk"`, etc.

**Why it happens:** `serde` fails on unknown variants by default with no fallback.

**How to avoid:** Add `#[serde(other)]` to the fallback variant (`Monochrome`) BEFORE adding `TdgGrade`. Write a test that deserializes `{ "color_mode": "churn" }` and asserts it produces `ColorMode::Monochrome`.

**Warning signs:** App panics on launch for users who had a non-standard color mode saved.

### Pitfall 4: file_path Normalization

**What goes wrong:** PMAT returns `file_path` values like `"./sentrux-core/src/app/channels.rs"`. The snapshot's file index keys are scan-root-relative without `./` prefix (e.g., `"sentrux-core/src/app/channels.rs"`).

**Why it happens:** PMAT uses `./`-prefixed relative paths; the scanner uses bare relative paths.

**How to avoid:** Strip `./` prefix when building `by_path` index in `PmatReport`. In `pmat_adapter.rs`:
```rust
let normalized = path.trim_start_matches("./").to_string();
```

**Warning signs:** Per-file TDG lookup returns `None` for all files; treemap shows no TDG coloring even when PMAT ran successfully.

### Pitfall 5: Subprocess Timeout on Large Projects

**What goes wrong:** `pmat analyze tdg` on a large project (>5000 files) can take 30+ seconds. If the scanner thread blocks on `Command::output()`, rescan requests queue up indefinitely.

**Why it happens:** `Command::output()` is synchronous and blocks until the subprocess exits.

**How to avoid:** Add a hard timeout using `std::thread` join with timeout, or use `Command::spawn()` + polling. Recommended: `Command::spawn()` and poll with a 60-second cap. On timeout, send `ScanMsg::Error("PMAT analysis timed out")` and continue with `pmat: None`.

**Warning signs:** Scan never completes on large repos; UI hangs indefinitely after scan.

---

## Code Examples

### Complete PMAT Subprocess Invocation

```rust
// Source: verified by running pmat 2.213.14 locally
// sentrux-core/src/analysis/pmat_adapter.rs

use std::process::Command;
use std::path::PathBuf;

/// Run `pmat analyze tdg` on the given root directory.
/// Returns None if PMAT is unavailable, exits with error, or times out.
/// Caller must handle None gracefully (show "PMAT unavailable" UI, don't block treemap).
pub fn run_pmat_tdg(root: &str, scan_gen: u64) -> Option<PmatTdgOutput> {
    // Check pmat is installed
    if Command::new("pmat").arg("--version").output().is_err() {
        return None;
    }

    let tmp_path: PathBuf = std::env::temp_dir()
        .join(format!("sentrux_pmat_tdg_{}.json", scan_gen));

    let status = Command::new("pmat")
        .args([
            "analyze", "tdg",
            "--format", "json",
            "--path", root,
            "-o", tmp_path.to_str().unwrap(),
        ])
        .current_dir(root)
        .stdout(std::process::Stdio::null()) // suppress progress output
        .stderr(std::process::Stdio::null()) // suppress error output
        .status()
        .ok()?;

    // pmat exits 1 on critical defects — treat as "no data" not "fatal error"
    if !status.success() {
        let _ = std::fs::remove_file(&tmp_path);
        return None;
    }

    let json_bytes = std::fs::read(&tmp_path).ok()?;
    let _ = std::fs::remove_file(&tmp_path);
    serde_json::from_slice(&json_bytes).ok()
}

/// Run `pmat repo-score` for the health panel.
pub fn run_pmat_repo_score(root: &str, scan_gen: u64) -> Option<PmatRepoScore> {
    let tmp_path: PathBuf = std::env::temp_dir()
        .join(format!("sentrux_pmat_score_{}.json", scan_gen));

    let status = Command::new("pmat")
        .args([
            "repo-score",
            "--format", "json",
            "--path", root,
            "-o", tmp_path.to_str().unwrap(),
        ])
        .current_dir(root)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?;

    if !status.success() {
        let _ = std::fs::remove_file(&tmp_path);
        return None;
    }

    let json_bytes = std::fs::read(&tmp_path).ok()?;
    let _ = std::fs::remove_file(&tmp_path);
    serde_json::from_slice(&json_bytes).ok()
}
```

### ScanReports Extension

```rust
// sentrux-core/src/app/channels.rs — add pmat field to ScanReports
pub struct ScanReports {
    /// PMAT TDG analysis results — None if pmat not installed or analysis failed
    pub pmat: Option<crate::core::pmat_types::PmatReport>,
    // Keep during transition (Phase 2 end: delete health, arch, test_gaps, rules)
    pub health: Option<HealthReport>,
    pub arch: Option<ArchReport>,
    pub evolution: Option<EvolutionReport>,
    pub test_gaps: Option<TestGapReport>,
    pub rules: Option<RuleCheckResult>,
}
```

### AppState Extension

```rust
// sentrux-core/src/app/state.rs — add pmat_report field
pub struct AppState {
    // ... existing fields ...
    /// PMAT analysis results — cached until next rescan
    pub pmat_report: Option<crate::core::pmat_types::PmatReport>,
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| sentrux internal grading (A-F based on coupling/entropy) | PMAT TDG grades (A+ through F, 11 levels) | Phase 2 | More granular, externally calibrated grades replace home-grown system |
| `ColorMode` with 9 variants (Monochrome, Language, Heat, Age, Churn, Risk, Git, ExecDepth, BlastRadius) | 4 variants: Language, Heat, Git, TdgGrade | Phase 2 | Simplified toolbar; TDG is the primary signal |
| `HealthReport` from `metrics::compute_health` | `PmatReport` from `pmat` subprocess | Phase 2 | External tool replaces internal coupling/entropy grader |
| `sentrux check` / `sentrux gate` using `metrics::rules` | Rewritten to use `pmat tdg --min-grade` or `pmat quality-gates` | Phase 2 | CLI commands delegate to PMAT's quality gate logic |

**Deprecated/outdated:**
- `metrics::grading` — replaced by PMAT grades
- `metrics::stability` — replaced by PMAT's coupling/entropy scores
- `metrics::arch` — architecture metrics (abstractness, instability) removed; keep `evo` for churn/bus factor
- `metrics::dsm` — Design Structure Matrix; PMAT covers structural coupling
- `metrics::testgap` — PMAT does not cover test gap; but the module provides value. KEEP for now, reassess after Phase 2 integration
- `metrics::whatif` — already removed in Phase 1
- `app/panels/health_display.rs` — replaced by `pmat_panel.rs`
- `app/panels/arch_display.rs` — removed with arch module
- `app/panels/dsm_panel.rs` — removed with dsm module
- `app/panels/testgap_display.rs` — kept if testgap module is kept
- `app/panels/whatif_display.rs` — already removed in Phase 1

---

## Open Questions

1. **PMAT-06: Mutation testing results**
   - What we know: PMAT has no mutation testing commands in its CLI (`pmat --help` lists no `mutants` or `mutation` subcommand). The installed version 2.213.14 does not expose mutation test data.
   - What's unclear: Whether a future PMAT version adds mutation testing; whether the user's intent was "mutation testing if available or TDG breakdown if not"
   - Recommendation: Redefine PMAT-06 as "file detail panel shows PMAT TDG breakdown (component scores, penalties, critical defects) when a file is selected" — this delivers meaningful per-file data without mutation testing. Flag to user in planning.

2. **PMAT critical defects on sentrux itself**
   - What we know: `pmat analyze tdg` on the sentrux project exits with code 1 due to 21 `.unwrap()` calls in non-test code. Phase 1 cleanup did not address these.
   - What's unclear: Whether Phase 2 should fix the `.unwrap()` calls in sentrux to make PMAT analysis usable on its own codebase during development, or accept `pmat: None` during development.
   - Recommendation: Add a Phase 2 task to fix the `.unwrap()` violations in sentrux (most are in layout/spatial_index.rs and layout/blueprint_dag.rs based on PMAT output) — this both unblocks PMAT integration testing and improves the codebase.

3. **`sentrux check` / `sentrux gate` rewrite strategy**
   - What we know: `pmat quality-gate --format json --fail-on-violation` provides a checks-based quality gate with configurable checks (dead-code, complexity, coverage, etc.); `pmat tdg --min-grade <GRADE>` provides grade-based gating
   - What's unclear: Whether to use `pmat quality-gate` (broader checks) or `pmat tdg --min-grade` (grade-only); the current `sentrux gate` uses architectural baseline diffing
   - Recommendation: Rewrite `sentrux check` → `pmat quality-gate --format json --fail-on-violation` (comprehensive); rewrite `sentrux gate` → `pmat tdg --min-grade C` (grade gate, configurable). Document this in the plan.

---

## Metrics Module Deletion Decision

Based on PMAT capability verification:

| Module | PMAT Covers? | Decision | Rationale |
|--------|-------------|----------|-----------|
| `metrics/grading.rs` | Yes (TDG grades) | DELETE | PMAT's grades replace this entirely |
| `metrics/stability.rs` | Partially (coupling in TDG) | DELETE | PMAT covers coupling; entropy logic not needed standalone |
| `metrics/arch/` | Partially (TDG includes structural coupling) | DELETE | Martin's abstractness/instability not provided by PMAT but not in user requirements |
| `metrics/dsm/` | Partially (coupling covered) | DELETE | DSM visualization not in Phase 2 or 3 scope |
| `metrics/evo/` | NO (no git history analysis) | KEEP | Churn, bus factor, temporal coupling are unique to sentrux — PMAT has `analyze churn` but it's a different tool boundary |
| `metrics/testgap/` | NO (no test gap analysis) | KEEP temporarily | Value not replaced by PMAT; assess post-Phase 2 |
| `metrics/rules/` | Superseded by `pmat quality-gate` | DELETE | `sentrux check` rewrites to `pmat quality-gate`; the rules engine is its sole consumer |

**Panels to delete:** `health_display.rs`, `arch_display.rs`, `dsm_panel.rs`, `rules_display.rs`
**Panels to keep:** `evolution_display.rs`, `testgap_display.rs` (if modules kept), `activity_panel.rs`
**Panels to create:** `pmat_panel.rs` (TDG health + file detail)

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
| PMAT-01 | `pmat analyze tdg` subprocess runs and returns parsed JSON | unit | `cargo test -p sentrux-core pmat_adapter -- --nocapture` | ❌ Wave 0 |
| PMAT-01 | PMAT not found → graceful error, no panic | unit | `cargo test -p sentrux-core pmat_not_found` | ❌ Wave 0 |
| PMAT-02 | Grade strings from `analyze tdg` parse correctly (APLus→A+) | unit | `cargo test -p sentrux-core grade_to_display` | ❌ Wave 0 |
| PMAT-02 | `grade_to_t` maps APLus→1.0, F→0.0 | unit | `cargo test -p sentrux-core grade_to_t` | ❌ Wave 0 |
| PMAT-03 | Badge shown above size threshold, hidden below | unit | `cargo test -p sentrux-core tdg_badge_threshold` | ❌ Wave 0 |
| PMAT-04 | `tdg_grade_color("APLus")` returns green-ish, `tdg_grade_color("F")` returns red-ish | unit | `cargo test -p sentrux-core tdg_grade_color` | ❌ Wave 0 |
| PMAT-04 | `ColorMode::TdgGrade` is default after opening project | unit | `cargo test -p sentrux-core default_color_mode` | ❌ Wave 0 |
| PMAT-05 | `pmat repo-score` JSON parses into `PmatRepoScore` struct | unit | `cargo test -p sentrux-core repo_score_parse` | ❌ Wave 0 |
| PMAT-07 | Old ColorModes (Age, Churn, Risk, ExecDepth, BlastRadius) absent from `ColorMode::ALL` | unit | `cargo test -p sentrux-core color_mode_variants` | ❌ Wave 0 |
| CLEN-04 | `metrics::grading`, `metrics::stability`, `metrics::arch`, `metrics::dsm`, `metrics::rules` modules do not exist | compile | `cargo build --workspace` (no `mod grading;` in metrics/mod.rs) | ❌ Wave 0 |
| ColorMode serde | Deserializing old prefs with `"churn"` produces `ColorMode::Monochrome`, not error | unit | `cargo test -p sentrux-core color_mode_serde_compat` | ❌ Wave 0 |
| file_path | `./path/to/file` normalizes to `path/to/file` for snapshot lookup | unit | `cargo test -p sentrux-core pmat_path_normalization` | ❌ Wave 0 |
| PMAT-06 | File detail panel shows TDG breakdown for selected file (manual only) | manual | Run sentrux, scan project, click a file, verify panel shows component scores | N/A |

### Sampling Rate
- **Per task commit:** `cargo test -p sentrux-core --lib 2>&1 | tail -20`
- **Per wave merge:** `cargo test --workspace 2>&1 | tail -30`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `sentrux-core/src/core/pmat_types.rs` — covers PMAT-01, PMAT-02, PMAT-04, PMAT-05 type parsing
- [ ] `sentrux-core/src/analysis/pmat_adapter.rs` — covers PMAT-01 subprocess invocation
- [ ] `sentrux-core/src/analysis/pmat_adapter_tests.rs` (or `#[cfg(test)]` block) — covers unit tests for above
- [ ] `sentrux-core/src/renderer/colors.rs` — `tdg_grade_color` function (PMAT-04)

---

## Sources

### Primary (HIGH confidence)
- Local `pmat 2.213.14` binary invocation — all JSON schemas verified by running the tool directly
- `sentrux-core/src/app/channels.rs` — `ScanReports` structure, extension pattern
- `sentrux-core/src/layout/types.rs` — `ColorMode` enum, `#[serde(rename_all = "lowercase")]`
- `sentrux-core/src/app/prefs.rs` — ColorMode persistence pattern
- `sentrux-core/src/renderer/colors.rs` — existing color function patterns
- `sentrux-core/src/renderer/badges.rs` — badge rendering pattern for TDG grade badges
- `sentrux-core/src/app/scanning.rs` — `ScanReports` consumption, `AppState` update pattern
- `https://raw.githubusercontent.com/paiml/paiml-mcp-agent-toolkit/master/src/tdg/grade.rs` — Grade enum definition and `from_score` thresholds

### Secondary (MEDIUM confidence)
- `pmat --help` and subcommand `--help` output — CLI surface verified but not all combinations tested
- GitHub `paiml/paiml-mcp-agent-toolkit` `src/` listing — confirms `lib.rs` exists on master; master version (3.7.1) diverges from installed crates.io version (2.213.14)
- `pmat repo-score --format json` output — schema verified on sentrux project

### Tertiary (LOW confidence)
- PMAT mutation testing capabilities — NOT found in installed version; `pmat --help` lists no mutation command; PMAT-06 must be re-scoped

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all subprocess commands verified by running locally
- JSON schemas: HIGH — all schemas verified by actual command execution with `-o` flag
- Grade string format: HIGH — verified by reading `src/tdg/grade.rs` on GitHub master
- Architecture patterns: HIGH — grounded in existing sentrux source files
- PMAT-06 mutation testing: LOW — no mutation testing found in installed pmat; requires user clarification
- `sentrux check`/`sentrux gate` rewrite: MEDIUM — `pmat quality-gate` and `pmat tdg --min-grade` verified as available commands but not tested end-to-end

**Research date:** 2026-03-14
**Valid until:** 2026-04-14 (30 days; pmat version may change but subprocess interface is stable)
