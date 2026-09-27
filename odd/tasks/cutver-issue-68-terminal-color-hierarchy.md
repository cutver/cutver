# Feature: Terminal Color Hierarchy and TTY/NO_COLOR Auto-Detection (Issue #68)

Goal: Implement an opinionated, subtle muted terminal color hierarchy across `cutver` CLI outputs with zero external dependencies, using `std::io::IsTerminal` and strict compliance with `NO_COLOR`, `CLICOLOR`, and `CLICOLOR_FORCE` standards per `CONTRACT.md`.

## Tasks
- [x] 1. Implement `src/cli/style.rs` with zero-dependency color engine, TTY detection, NO_COLOR handling, and semantic palette
- [x] 2. Wire `style` into `src/cli.rs` and apply standard styling across `src/cli/runner.rs` (status icons, simulation banner, error reporting)
- [x] 3. Add unit and integration tests verifying ANSI styling, NO_COLOR suppression, and deterministic fallback in pipes
- [x] 4. Run full verification suite (`cargo test --locked`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`) and record evidence

## Verification Evidence
- Commit: `9e25b82` (`feat(cli): terminal color hierarchy and TTY/NO_COLOR auto-detection (#68)`)
- `cargo test --locked`: Passed (235 unit tests in `src/lib.rs`, all e2e suites including 3 new tests in `tests/e2e_style.rs` passing).
- `cargo clippy --all-targets -- -D warnings`: Passed cleanly with zero warnings.
- `cargo fmt -- --check`: Passed cleanly with formatting matching workspace rules.
