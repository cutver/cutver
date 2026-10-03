# Feature: Modularize Conventional Commit Engine & Deduce Logic (Issue #109)

Goal: Decompose `src/conventional.rs` (671 lines) into focused submodules under `src/conventional/` adhering to `CONTRACT.md` Pillar V.4 (Module Budget $\le 300-400$ lines) behind a clean Facade in `src/conventional.rs`.

## Tasks
- [x] Task 1: Decompose `src/conventional.rs` into submodules under `src/conventional/` (`types.rs`, `parse.rs`, `deduce.rs`, `tests.rs`) with a thin Facade in `src/conventional.rs`.
- [x] Task 2: Audit module line budgets ($\le 350-400$ lines per production file) and ensure 0 unwraps in production code.
- [x] Task 3: Comprehensive verification and quality gates (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Completion Evidence
- Modularized `src/conventional.rs` into submodules:
  - `src/conventional.rs` (9 lines): Facade re-exporting public API.
  - `src/conventional/types.rs` (88 lines): `ConventionalCommit` and `BumpRationale` structs, display and summary methods.
  - `src/conventional/parse.rs` (190 lines): Header, paragraph, and footer parsing logic.
  - `src/conventional/deduce.rs` (78 lines): SemVer bump and rationale deduction logic.
  - `src/conventional/tests.rs` (315 lines): Unit test suite for parsing and deduction.
- Production files strictly within budget (target <= 300, ceiling <= 400 lines; maximum production file is 190 lines).
- 0 `unwrap()`, `expect()`, or `panic!()` in production code.
- 100% test pass rate (`cargo test` - 277 unit tests, 23 bump e2e tests, 12 changelog cli e2e tests, 7 template e2e tests, 12 conventional e2e tests, 3 doctor e2e tests, 9 init e2e tests, 11 lifecycle e2e tests, 5 open e2e tests, 5 style e2e tests, 1 context test).
- Quality gates clean: `cargo clippy --all-targets -- -D warnings` and `cargo fmt -- --check`.

### Work-Unit Commit
- Commit: `7da33a1` (`refactor(conventional): modularize commit parser, types, and bump deduction (#109)`)
