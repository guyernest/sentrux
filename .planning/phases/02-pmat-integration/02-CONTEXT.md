# Phase 2: PMAT Integration - Context

**Gathered:** 2026-03-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Integrate PMAT as sentrux's analysis engine. Every file node in the treemap displays a PMAT TDG grade, health/mutation panels show PMAT data, sentrux's own analysis engine is removed, and CLI commands (check/gate) are rewired to PMAT. The old ColorModes that depend on the metrics engine are pruned.

</domain>

<decisions>
## Implementation Decisions

### PMAT Integration Mode
- Subprocess integration: spawn `pmat analyze tdg --output json`, parse stdout via serde_json
- If PMAT API spike finds a library crate, prefer that — but subprocess is the confirmed fallback
- PMAT is **required** — sentrux will not scan without it
- Error on PMAT missing: show clear error banner ("PMAT not found. Install: cargo install pmat") and refuse to scan
- Cache PMAT results in AppState — only re-run on file changes (rescan), not every frame

### TDG Grade Display
- New `ColorMode::TdgGrade` variant with green-to-red gradient (A+ = deep green → F = red)
- TDG is the **default ColorMode** when opening a project (replaces Language as default)
- Letter grade badges (A+, B-, etc.) shown on treemap nodes **above a size threshold** — small nodes show color only
- If PMAT only provides aggregate grades (not per-file), that's fine — show project-level grade in panel, treemap coloring can use a different available signal

### Panel Layout
- Mutation testing results shown in the **file detail panel** (click a file to see mutation score + survived mutants)
- Progressive disclosure: show TDG grade + health score summary by default, expand for full breakdown on click
- Health panel approach: Claude's discretion (dedicated panel vs replacing existing)

### Metrics Transition
- **Prune ColorModes to essentials**: keep Language, Heat, Git, TDG. Drop Age, Churn, Risk, ExecDepth, BlastRadius
- **Rewrite `sentrux check` and `sentrux gate`** to use PMAT's TDG grades instead of sentrux's metrics engine
- **Metrics engine deletion**: assess which modules provide genuine engineering value that PMAT doesn't cover (code coverage, static analysis patterns). Keep those; delete what PMAT replaces. Don't blindly delete everything — if evo (git churn, bus factor, temporal coupling) or other modules provide value PMAT lacks, keep them as add-ons
- `#[serde(other)]` fallback on ColorMode enum before adding TdgGrade variant (prevents breaking saved preferences)

### Claude's Discretion
- Health panel layout approach (dedicated vs replacing existing panel)
- Which metrics/ submodules to keep vs delete (based on PMAT capability assessment during spike)
- PMAT subprocess invocation details (timeout, working directory, argument format)
- Loading/progress UX during PMAT subprocess execution

</decisions>

<specifics>
## Specific Ideas

- User wants good software engineering practices visible: code coverage, static analysis, mutation testing — either from PMAT or kept from sentrux as add-ons
- The tool should feel simpler than today's complex rating system — PMAT's A+-F scale is the simplification
- Green-to-red is the expected health color language (familiar from CI dashboards, SonarQube, etc.)

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `renderer/rects.rs:file_color()` — existing dispatch for ColorMode variants, adding TdgGrade is a new match arm
- `renderer/badges.rs` — existing badge rendering infrastructure for health/git indicators on treemap nodes
- `layout/types.rs:ColorMode` — enum with 9 variants, serde serialization, tier gating, `ALL`/`FREE` const arrays
- `app/channels.rs:ScanReports` — carries analysis results from scanner thread to UI; add `pmat: Option<PmatReport>` field
- `core/heat.rs` — overlay color pattern (template for how new overlays integrate with renderer)

### Established Patterns
- Scanner thread produces `ScanMsg::Complete` with `Arc<Snapshot>` + `ScanReports` — PMAT subprocess result joins this flow
- `RenderContext` carries `color_mode` to renderer — renderer never imports `AppState`
- Pro tier gating via `ColorMode::is_pro()` — decide if TDG is free or pro tier

### Integration Points
- `sentrux-bin/src/main.rs:run_check()` and `run_gate()` — rewire from `metrics::compute_health` to PMAT subprocess
- `app/panels/` — existing panel infrastructure for health display; replace content with PMAT data
- `app/toolbar.rs` — ColorMode selector dropdown; will show pruned set (Language, Heat, Git, TDG)

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-pmat-integration*
*Context gathered: 2026-03-14*
