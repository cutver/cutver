# Feature: Modularize Bump Domain Types & Test Suite (Issue #117)

Goal: Decompose `src/bump.rs` (506 lines) into focused submodules under `src/bump/` adhering to `CONTRACT.md` Pillar V.4 (Module Budget $\le 300-400$ lines) behind a clean Facade in `src/bump.rs`.

## Tasks
- [x] Task 1: Decompose `src/bump.rs` into submodules under `src/bump/` (`types.rs`, `tests.rs`) with a thin Facade in `src/bump.rs`.
- [x] Task 2: Audit module line budgets ($\le 350-400$ lines per production file) and ensure 0 unwraps in production code.
- [x] Task 3: Comprehensive verification and quality gates (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Completion Evidence
- **Decomposition**:
  - `src/bump.rs` reduced from 506 lines to 8 lines (thin Facade exporting submodules and domain types/functions).
  - Extracted domain errors and structs into `src/bump/types.rs` (139 lines): `Error`, `Change`, `Touched`, `Summary`, `Drift`, `ChangelogDrift`.
  - Extracted unit test suite into `src/bump/tests.rs` (359 lines).
- **Line Budgets & Quality**:
  - `src/bump.rs`: 8 lines ($\le 20$ lines target).
  - `src/bump/types.rs`: 139 lines ($\le 300$ lines target).
  - `src/bump/tests.rs`: 359 lines ($\le 500$ lines test suite target).
  - Production code in `src/bump/` strictly respects hard ceiling $\le 400$ lines with 0 `unwrap()`/`expect()` in non-test production code.
- **Verification Gates**:
  - `cargo test`: 278 unit tests + 76 integration/e2e tests passed (100% green).
  - `cargo clippy --all-targets -- -D warnings`: Clean with 0 warnings.
  - `cargo fmt -- --check`: Canonical formatting strictly verified.

### Work-Unit Commit
- Commit: `32a5493` (`refactor(bump): modularize bump domain types, errors, and test suite (#117)`)
