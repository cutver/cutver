# Feature: ANSI-Styled Terminal Rendering for Changelog Commands (Issue #72)

Goal: Enhance `cutver changelog latest` and `cutver changelog show` with clean ANSI terminal rendering in interactive TTY sessions while preserving byte-for-byte pure Markdown when piped or redirected, adhering to `CONTRACT.md` (Pillar I, II, III, IV, and Pillar V: max 2 levels of nesting, function budget ≤ 35 lines, atomic helpers).

## Verification Evidence
- Commit: `785e016` (`feat(changelog): ANSI-styled terminal rendering for changelog commands in interactive TTY (#72)`)
- `cargo test --locked`: Passed all 327 tests across unit tests and integration test suites.
- `cargo clippy --all-targets -- -D warnings`: Passed cleanly with zero warnings.
- `cargo fmt -- --check`: Clean formatting conforming to workspace rules.
- Pillar V compliance: all functions in `src/cli/changelog.rs` ≤ 20 lines, max nesting depth ≤ 1, zero unwrap in production code.

## Verification Evidence
- `cargo test --locked`: Passed all unit and integration tests (250 unit tests, 21 bump e2e tests, 9 changelog cli tests, 4 style e2e tests, 7 template tests, 12 conventional tests, 3 doctor tests, 9 init tests, 11 lifecycle tests).
- `cargo clippy --all-targets -- -D warnings`: Passed with zero warnings.
- `cargo fmt -- --check`: Clean formatting adhered to.
