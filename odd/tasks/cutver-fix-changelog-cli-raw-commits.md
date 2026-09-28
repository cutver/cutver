# Fix: Preserve Raw Git Commits in Changelog CLI Commands

Goal: Update `resolve_latest_target_info` and `resolve_show_target_info` in `src/cli/runner.rs` to fetch and preserve `RawCommit` (via `raw_commits_since` and `raw_commits_between`), pass them to `build_context_with_raw_and_filter`, so that `c.line`, `c.short_hash`, `c.commit_url`, and `c.author` are fully enriched in `cutver changelog latest` and `cutver changelog show`, adhering strictly to `CONTRACT.md`.

## Tasks
- [x] Task 1: Update `ReleaseTargetInfo` in `src/cli/runner.rs` to hold `raw_commits: Vec<crate::git::RawCommit>`.
- [x] Task 2: Update `resolve_latest_target_info` and `resolve_show_target_info` to call `crate::git::raw_commits_since` and `crate::git::raw_commits_between`.
- [x] Task 3: In `resolve_changelog_context`, parse conventional commits from raw commit messages and pass `Some(&target.raw_commits)` to `build_context_with_raw_and_filter`.
- [x] Task 4: Add E2E tests in `tests/e2e_changelog_cli.rs` asserting that `cutver changelog latest` and `cutver changelog show` with templates render short hashes and author attributions.
- [x] Task 5: Verify all checks (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`, `cargo run -- doctor --check-changelog`).

## Verification Evidence
- `cargo test`: All 334 tests passed cleanly across unit tests and e2e integration suites (`e2e_bump`, `e2e_changelog_cli`, `e2e_changelog_template`, `e2e_conventional`, `e2e_doctor`, `e2e_init`, `e2e_lifecycle`, `e2e_style`, `test_context`).
- `cargo clippy --all-targets -- -D warnings`: Passed cleanly with zero warnings.
- `cargo fmt -- --check`: Checked and formatted cleanly.
- `cargo run -- doctor --check-changelog`: Passed with exit code 0 (`✔ cutver.toml is valid`, Changelog consistent with Git tags).
