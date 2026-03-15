---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: planning
stopped_at: Completed 04-gsd-phase-overlay 04-01-PLAN.md
last_updated: "2026-03-15T20:05:29.798Z"
last_activity: 2026-03-14 — Roadmap created; ready to begin Phase 1 planning
progress:
  total_phases: 5
  completed_phases: 4
  total_plans: 15
  completed_plans: 13
  percent: 50
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-14)

**Core value:** Show developers the health and evolution of their codebase through an interactive treemap powered by PMAT's analysis — past changes via git diff overlays, present state via TDG grades, and future direction via GSD phase overlays.
**Current focus:** Phase 1 — Cleanup

## Current Position

Phase: 1 of 4 (Cleanup)
Plan: 0 of TBD in current phase
Status: Ready to plan
Last activity: 2026-03-14 — Roadmap created; ready to begin Phase 1 planning

Progress: [█████░░░░░] 50%

## Performance Metrics

**Velocity:**
- Total plans completed: 0
- Average duration: —
- Total execution time: 0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

**Recent Trend:**
- Last 5 plans: —
- Trend: —

*Updated after each plan completion*
| Phase 01-cleanup P01 | 5 | 2 tasks | 11 files |
| Phase 01-cleanup P02 | 3 | 2 tasks | 3 files |
| Phase 02-pmat-integration P01 | 5 | 2 tasks | 11 files |
| Phase 02-pmat-integration P02 | 5 | 2 tasks | 10 files |
| Phase 02-pmat-integration P03 | 12 | 2 tasks | 8 files |
| Phase 02-pmat-integration P04 | 18 | 2 tasks | 35 files |
| Phase 02.1-rust-deep-analysis P01 | 4 | 2 tasks | 5 files |
| Phase 02.1-rust-deep-analysis P02 | 3 | 2 tasks | 8 files |
| Phase 02.1-rust-deep-analysis P03 | 90 | 2 tasks | 12 files |
| Phase 03-git-diff-overlay P01 | 7 | 2 tasks | 10 files |
| Phase 03-git-diff-overlay P02 | 4 | 2 tasks | 6 files |
| Phase 03-git-diff-overlay P03 | 50 | 2 tasks | 8 files |
| Phase 04-gsd-phase-overlay P01 | 35 | 2 tasks | 16 files |

## Accumulated Context

### Roadmap Evolution

- Phase 02.1 inserted after Phase 2: Rust Deep Analysis (INSERTED) — integrate PMAT code rank, test coverage (cargo-llvm-cov), and clippy advanced linting into treemap visualization

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Pre-Phase 1]: PMAT may be binary-only (no lib.rs confirmed) — Phase 2 must start with a PMAT API spike before any integration code is written
- [Pre-Phase 1]: Cleanup (MCP server, plugins, language narrowing) can land first as safe subtractive changes
- [Pre-Phase 1]: Git diff overlay uses existing git2 dependency — relatively bounded scope
- [Pre-Phase 1]: GSD overlay has highest complexity — needs GSD plan reader + file path normalization
- [Phase 01-cleanup]: lang_registry.rs rewritten as empty static registry to break plugin::load_all_plugins() dependency; Plan 02 will populate with compiled-in Rust/TS/JS grammars
- [Phase 01-cleanup]: dirs crate removed by replacing dirs::home_dir() in update_check.rs with HOME/USERPROFILE env var lookup
- [Phase 01-cleanup]: Language::new(LanguageFn) pattern used for tree-sitter 0.25 grammar compilation; tsx maps to LANGUAGE_TYPESCRIPT
- [Phase 01-cleanup]: PluginLangConfig renamed to LangConfig to remove plugin terminology from the static registry
- [Phase 01-cleanup]: Pre-existing oracle failures (27) for removed languages are expected; 3-language binary is CLEN-03 complete
- [Phase 02-pmat-integration]: PmatReport by_path uses index into tdg.files (zero-copy lookup) not clone
- [Phase 02-pmat-integration]: run_pmat_tdg accepts non-zero exit: PMAT exits 1 on critical defects but writes valid JSON
- [Phase 02-pmat-integration]: Tarjan SCC .unwrap() replaced with .expect(invariant msg) to satisfy PMAT heuristic
- [Phase 02-pmat-integration]: TdgGrade is default ColorMode: primary free mode, replaces Monochrome/Language as the first thing users see
- [Phase 02-pmat-integration]: ColorMode serde(other) on Monochrome: old prefs with churn/risk/age/execdepth/blastradius deserialize safely to Monochrome
- [Phase 02-pmat-integration]: PMAT is required for scan — check_pmat_available() at scan start, ScanMsg::Error if missing
- [Phase 02-pmat-integration]: draw_pmat_panel wired into metrics_panel.rs (consistent with all other panels), not draw_panels.rs
- [Phase 02-pmat-integration]: sentrux check delegates to pmat quality-gate subprocess; sentrux gate delegates to pmat tdg --min-grade C
- [Phase 02-pmat-integration]: 91 tests deleted with deleted modules (grading/arch/dsm/rules/stability) — pre-existing 27 oracle failures unchanged
- [Phase 02.1-rust-deep-analysis]: risk_color() takes max_raw parameter for project-level normalization (not fixed constant)
- [Phase 02.1-rust-deep-analysis]: ColorMode::Coverage and Risk fall back to monochrome in file_color() until 02.1-02 wires real data
- [Phase 02.1-rust-deep-analysis]: New ColorMode variants must be inserted BEFORE serde(other) Monochrome in enum definition
- [Phase 02.1-rust-deep-analysis]: color_by_coverage() returns muted gray for uninstrumented files to distinguish from no-data (monochrome) and 0% coverage (red)
- [Phase 02.1-rust-deep-analysis]: color_by_risk() uses max_raw=1.0 placeholder; Plan 03 can refine with project-level PageRank max normalization
- [Phase 02.1-rust-deep-analysis]: coverage_requested flag routes coverage spawn through draw_panels.rs (owns scan_msg_tx) — toolbar stays stateless
- [Phase 02.1-rust-deep-analysis]: risk normalization: max_raw computed per-frame from max PageRank; propagated via RenderContext so color_by_risk() stays pure
- [Phase 03-git-diff-overlay]: GitDiff serializes to 'GitDiff' (PascalCase) via serde(rename) to distinguish from lowercase rename_all convention
- [Phase 03-git-diff-overlay]: raw_intensity = sqrt(lines * commits): geometric mean prevents either volume or frequency from dominating
- [Phase 03-git-diff-overlay]: muted gray (50,52,55) for files not in git diff report: distinguishes unchanged-in-window from no-data (GDIT-04)
- [Phase 03-git-diff-overlay]: git_diff_window and git_diff_custom_n not reset on new scan — user selections survive project changes
- [Phase 03-git-diff-overlay]: draw_git_diff_section only shown when color_mode == ColorMode::GitDiff to avoid noise in other modes
- [Phase 03-git-diff-overlay]: Color legend placed below toolbar as second row in TopBottomPanel — avoids modifying canvas/panel layout code
- [Phase 03-git-diff-overlay]: Auto-trigger guards !state.scanning to prevent triggering git diff walk during a full scan
- [Phase 03-git-diff-overlay]: NO_DATA_GRAY constant extracted to colors.rs (was magic literal duplicated in 3 files)
- [Phase 03-git-diff-overlay]: SinceLastTag with no tags returns empty report instead of unbounded walk — prevents freeze on fresh repos
- [Phase 04-gsd-phase-overlay]: DiffWindow::CommitRange adds String fields making Copy impossible; preset_slice() OnceLock fn replaces PRESETS const; callers updated to .clone()
- [Phase 04-gsd-phase-overlay]: ColorMode::GsdPhase serializes to GsdPhase (PascalCase serde rename) consistent with GitDiff pattern
- [Phase 04-gsd-phase-overlay]: color_by_gsd_phase uses find_directory_match() from gsd_phase_adapter for directory prefix entries; NO_DATA_GRAY for unassociated files

### Pending Todos

None yet.

### Blockers/Concerns

- **[Phase 2 blocker]**: PMAT has no confirmed library API (lib.rs not found on GitHub). Phase 2 planning MUST begin with a spike to determine subprocess vs. library integration before any other Phase 2 tasks are written.
- **[Phase 3 concern]**: ColorMode enum is serialized to disk — adding new variants (GitDiff, GsdPhase) will break deserialization of saved user preferences. Must add `#[serde(other)]` fallback before Phase 3 ships.
- **[Phase 4 concern]**: GSD path matching is brittle — snapshot paths are scan-root-relative; plan files may use absolute, ./prefixed, or wrong-case paths. Path normalization utility needed before Phase 4.

## Session Continuity

Last session: 2026-03-15T20:05:29.796Z
Stopped at: Completed 04-gsd-phase-overlay 04-01-PLAN.md
Resume file: None
