# Feature: Modularize CLI Args Definitions & Normalization (Issue #113)

Goal: Decompose `src/cli/args.rs` (560 lines) into focused submodules under `src/cli/args/` adhering to `CONTRACT.md` Pillar V.4 (Module Budget $\le 300-400$ lines) behind a clean Facade in `src/cli/args.rs`.

## Tasks
- [x] Task 1: Decompose `src/cli/args.rs` into submodules under `src/cli/args/` (`commands.rs`, `normalize.rs`, `tests.rs`) with a thin Facade in `src/cli/args.rs`.
- [x] Task 2: Audit module line budgets ($\le 350-400$ lines per production file) and ensure 0 unwraps in production code.
- [x] Task 3: Comprehensive verification and quality gates (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Completion Evidence
- **Line budgets**:
  - `src/cli/args.rs`: 8 lines (Thin Facade re-exporting `Cli`, `Commands`, `ChangelogCommands`, `BumpLevel`, and `normalize_args`)
  - `src/cli/args/commands.rs`: 141 lines ($\le 300$ target)
  - `src/cli/args/normalize.rs`: 24 lines ($\le 300$ target)
  - `src/cli/args/tests.rs`: 395 lines (unit tests under `mod tests`)
- **Unwrap / Expect Audit**:
  - 0 unwrap/expect in production code (`commands.rs`, `normalize.rs`, `args.rs`). Only allowed inside `tests.rs`.
- **Quality Gates**:
  - `cargo test`: 278 lib unit tests + integration test suites passed 100%.
  - `cargo clippy --all-targets -- -D warnings`: Clean (0 warnings).
  - `cargo fmt -- --check`: Clean formatting across all files.

### Work-Unit Commit
- Commit: `bb3c220` (`refactor(cli): modularize CLI args definitions and normalization (#113)`)
