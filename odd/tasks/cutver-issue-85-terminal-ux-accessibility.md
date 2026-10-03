# Feature: Terminal UX & Accessibility: Workspace-Relative Paths, OSC 8 Hyperlinks & 'cutver open' (Issue #85)

Goal: Enhance terminal user experience and accessibility across `cutver`:
1. Normalize manifest and changelog paths in the `Release Plan` tree to workspace-relative paths.
2. Introduce terminal hyperlinks (OSC 8) for pull requests, commits, and diffs with accessible footnote references for screen reader users.
3. Add a dedicated `cutver open` (and `cutver changelog open`) subcommand to open releases, PRs, or compare views in the default system browser via keyboard.
Adhering strictly to `CONTRACT.md` Pillars I, II, III, IV, and V.

## Tasks
- [x] Task 1: Normalize release plan paths to workspace-relative in `src/bump.rs`, `src/bump/exec.rs`, `src/cli/tree.rs`, and verify via `tests/e2e_bump.rs`.
- [ ] Task 2: Implement OSC 8 terminal hyperlinks helper and accessible footnote reference formatting in `src/cli/link.rs` / `src/cli/style.rs` honoring `NO_COLOR` and `--plain`.
- [ ] Task 3: Implement `cutver open` (and `cutver changelog open`) subcommand with `--print-url` / `--dry-run` and system browser dispatch (`src/cli/open.rs`, `src/cli/args.rs`, `src/cli/runner.rs`).
- [ ] Task 4: Comprehensive integration tests, e2e test suite, and quality gates (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Completion Evidence
