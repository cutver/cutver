# Feature: Structured JSON Export for Release Context (Issue #66)

Goal: Implement `--json` flag in `cutver changelog latest` and `cutver changelog show <version>` to serialize the authoritative `ReleaseContext` as pretty JSON directly to stdout, adhering strictly to `CONTRACT.md` (Pillar I, II, III, IV, and Pillar V: structural purity, flat hierarchy, max nesting depth <= 2, function budget <= 35-40 lines, exhaustive enum dispatch).

## Tasks
- [x] Task 1: Add `--json` flag to `ChangelogCommands::Latest` and `ChangelogCommands::Show` in `src/cli/args.rs`.
- [x] Task 2: Implement domain `ChangelogFormat` enum and execution helpers in `src/cli/changelog.rs` adhering strictly to Pillar V (clean dispatch for JSON vs Template vs Markdown, function budget <= 35 lines, max nesting depth <= 2).
- [x] Task 3: Refactor `run_changelog` in `src/cli/runner.rs` to extract common context resolution into small, focused helpers and dispatch through `ChangelogFormat`.
- [x] Task 4: Add unit and integration tests in `tests/e2e_changelog_cli.rs` and `src/cli/args.rs` ensuring valid JSON structure, absence of ANSI sequences, and pipe compatibility with `serde_json`.
- [x] Task 5: Verify all checks (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Verification Evidence
- `cargo test`: All 316 unit, integration, and doc tests pass.
- `cargo clippy --all-targets -- -D warnings`: 0 warnings.
- `cargo fmt -- --check`: Clean formatting across all modified files.
- `e2e_changelog_cli`:
  - `changelog_latest_cli_json_export` passes (valid JSON, ANSI-clean under `CLICOLOR_FORCE=1`).
  - `changelog_show_cli_json_export` passes (valid JSON with target version, previous version, and SemVer breakdown).
