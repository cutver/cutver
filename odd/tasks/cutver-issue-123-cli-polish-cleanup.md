# Feature: Drop release.toml, Workspace-Relative Diagnostics, Dry-Run Polish & WSL Browser Support (Issue #123)

Goal: Implement Issue #123 adhering strictly to `CONTRACT.md` Pillars I, II, III, IV, and V:
1. Drop backward-compatible `release.toml` support; require `cutver.toml` as the sole canonical configuration file.
2. Centralize `relativize_path` helper and apply workspace-relative path formatting in `doctor` (dashboard row and version drift diagnostics) and `changelog` runner error messages.
3. Clean up the `--dry-run` simulation banner in `tree.rs` by removing the redundant `ℹ` info icon.
4. Support `$BROWSER` environment variable and auto-detect WSL to fall back to `wslview` in `src/cli/open.rs`.
5. Full verification and quality gates.

## Tasks
- [x] Task 1: Drop legacy `release.toml` support in `src/config/discovery.rs`, `src/config/types.rs`, `src/cli/doctor.rs`, `src/cli/args/commands.rs`, runner tests, and docs.
- [x] Task 2: Centralize `relativize_path` helper in `src/cli/path.rs` and apply to `doctor` (dashboard manifests row and version drift diagnostics) and `src/cli/runner/changelog.rs`.
- [x] Task 3: Polish `--dry-run` simulation banner in `src/cli/tree.rs` (remove redundant `ℹ` before `SIMULATION MODE`).
- [x] Task 4: Add `$BROWSER` resolution and WSL `wslview` fallback to `src/cli/open.rs`.
- [x] Task 5: Comprehensive test suite verification, clippy, and formatting (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Completion Evidence
- `cargo test`: 290 unit tests passed; all integration test suites passed (bump, changelog_cli, changelog_template, conventional, doctor, init, lifecycle, open, style, context).
- `cargo clippy --all-targets -- -D warnings`: 0 warnings.
- `cargo fmt -- --check`: passed cleanly.
