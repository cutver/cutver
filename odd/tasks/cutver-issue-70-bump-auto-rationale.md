# Feature: Explain SemVer Deduction Rationale in 'cutver bump auto' (Issue #70)

Goal: Provide semantic transparency in `cutver bump auto` and `--dry-run` by modeling and detailing the conventional commit rationale (`BumpRationale`) in the functional core (`crate::conventional`) and surfacing it in the release summary per `CONTRACT.md`.

## Tasks
- [x] 1. Model `BumpRationale` in `src/conventional.rs` and implement pure derivation function with zero unwrap
- [x] 2. Integrate `BumpRationale` into `crate::bump::Summary` and populate during `bump auto` and `--first-release` in `src/bump/exec.rs`
- [x] 3. Render semantic rationale with theme styling in `src/cli/runner.rs` (`print_bump_summary`)
- [x] 4. Add unit tests in `src/conventional.rs` and e2e integration tests in `tests/e2e_conventional.rs` verifying rationale output
- [x] 5. Run full verification suite (`cargo test --locked`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`) and record evidence

## Verification Evidence
- Commit: `94bac90` (`feat(bump): explain SemVer deduction rationale in 'cutver bump auto' (#70)`)
- `cargo test --locked`: Passed all 314 tests across unit tests and integration suites (lib: 239 passed, e2e_bump: 21, e2e_changelog_cli: 8, e2e_changelog_template: 7, e2e_conventional: 12, e2e_doctor: 3, e2e_init: 9, e2e_lifecycle: 11, e2e_style: 3, test_context: 1).
- `cargo clippy --all-targets -- -D warnings`: Passed cleanly with zero warnings.
- `cargo fmt -- --check`: Checked and formatted cleanly.
