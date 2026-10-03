# Feature: Centralize ReleaseContext Resolution & Primary Manifest Version Lookup (Issue #95)

Goal: Implement Phase 2 of the hardening roadmap by eliminating cross-subcommand duplication between `bump`, `changelog`, and `doctor` under `CONTRACT.md` Pillars I.1 and V.6.

## Tasks
- [x] Task 1: Add `primary_manifest(&self) -> Option<&Manifest>` to `Config` in `src/config/types.rs` and unify primary manifest lookup across `src/bump/exec.rs` and `src/cli/doctor.rs`.
- [x] Task 2: Implement canonical `assemble_release_context` helper in `src/changelog/context.rs` and adopt it in `src/bump/exec.rs` and `src/cli/runner.rs`.
- [x] Task 3: Comprehensive verification and quality gates (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Completion Evidence
- **Primary Manifest Lookup**: Added `Config::primary_manifest` in `src/config/types.rs`, unifying primary manifest detection across `src/bump/exec.rs` and `src/cli/doctor.rs`. Added unit tests in `src/config/types.rs`.
- **ReleaseContext Assembly**: Introduced `AssembleContextParams` and `assemble_release_context` in `src/changelog/context.rs` (re-exported in `src/changelog.rs`). Replaced duplicated parameter assembly and configuration extraction in `src/bump/exec.rs` and `src/cli/runner.rs`. Added unit test `test_assemble_release_context`.
- **Quality Gates**:
  - `cargo test --lib`: 274 passed (2 new unit tests).
  - `cargo test`: All 274 unit tests and all integration test suites passed cleanly.
  - `cargo clippy --all-targets -- -D warnings`: Clean, 0 warnings.
  - `cargo fmt -- --check`: Clean formatting verified.
