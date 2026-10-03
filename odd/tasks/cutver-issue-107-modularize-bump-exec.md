# Feature: Modularize Mutation Execution Engine & Doctor Checks (Issue #107)

Goal: Decompose `src/bump/exec.rs` (732 lines) into focused submodules under `src/bump/exec/` adhering to `CONTRACT.md` Pillar V.4 (Module Budget $\le 300-400$ lines) behind a clean Facade in `src/bump/exec.rs`.

## Tasks
- [x] Task 1: Decompose `src/bump/exec.rs` into submodules under `src/bump/exec/` (`pipeline.rs`, `transaction.rs`, `doctor.rs`, `command.rs`, `tests.rs`) with a thin Facade in `src/bump/exec.rs`.
- [x] Task 2: Audit module line budgets ($\le 350-400$ lines per production file) and ensure 0 unwraps in production code.
- [x] Task 3: Comprehensive verification and quality gates (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Completion Evidence
- Modularization layout:
  - `src/bump/exec.rs`: 16 lines (Thin Facade re-exporting API with 100% backwards compatibility)
  - `src/bump/exec/transaction.rs`: 54 lines (RAII `MutationTransaction` drop guard and unstage)
  - `src/bump/exec/tests.rs`: 61 lines (Isolated unit tests)
  - `src/bump/exec/doctor.rs`: 82 lines (Manifest drift and changelog drift verification)
  - `src/bump/exec/manifest.rs`: 99 lines (Manifest compute, apply, and current_source resolution)
  - `src/bump/exec/changelog.rs`: 111 lines (Changelog update preparation and application)
  - `src/bump/exec/command.rs`: 150 lines (Command execution, shell spawning, tree kill, lockfile detection, read helper)
  - `src/bump/exec/pipeline.rs`: 308 lines (Orchestrates bump pipeline, decomposed into focused subfunctions with clean parameter structs)
- Quality Gates:
  - Line budgets: Every single production submodule strictly satisfies CONTRACT.md Pillar V.4 (target <= 300, hard ceiling <= 400 lines; max production file is `pipeline.rs` at 308 lines).
  - Zero unwraps / expects in production code (`src/bump/exec/` production files contain 0 instances of `.unwrap()` or `.expect()`).
  - `cargo test`: 277 unit tests, 23 bump e2e tests, all test suites passed cleanly.
  - `cargo clippy --all-targets -- -D warnings`: Clean, zero warnings.
  - `cargo fmt -- --check`: Clean formatting across repository.

### Work-Unit Commit
- Commit: `56fc7ea` (`refactor(bump): modularize mutation execution engine and doctor checks (#107)`)
