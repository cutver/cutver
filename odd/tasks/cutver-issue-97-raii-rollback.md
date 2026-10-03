# Feature: Implement RAII MutationTransaction Guard & Eliminate Procedural Rollbacks (Issue #97)

Goal: Implement Phase 3 of the hardening roadmap by replacing scattered, manual procedural rollbacks in `src/bump/exec.rs` with an RAII `MutationTransaction` drop guard under `CONTRACT.md` Pillar I.2.

## Tasks
- [x] Task 1: Implement `MutationTransaction` RAII drop guard tracking file backups and git staging in `src/bump/exec.rs`.
- [x] Task 2: Refactor `run_with_first_release` to use `MutationTransaction`, eliminating manual `rollback()` calls and scattered error-path invocations.
- [x] Task 3: Comprehensive verification and quality gates (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Completion Evidence
- Implemented `MutationTransaction<'a>` with `record_backup`, `mark_staged`, `commit`, and `Drop` implementation that restores backups in reverse order and unstages git changes if staged.
- Updated `apply()` to record file backups before writing and pass errors directly without procedural rollback loops.
- Recorded changelog backups in `MutationTransaction` prior to atomic write.
- Replaced scattered manual `rollback(...)` and `unstage(repo)` calls in `src/bump/exec.rs` error handling with automatic RAII drop guard rollback.
- Deleted `fn rollback(...)`.
- Added unit test `changelog_write_failure_restores_previously_written_manifests` in `src/bump.rs` verifying RAII rollback on changelog write failure.
- Verified all quality gates pass:
  - `cargo test --lib`: 275 tests passed
  - `cargo test --test e2e_bump`: 23 tests passed
  - `cargo test --test e2e_lifecycle`: 11 tests passed
  - `cargo test`: all unit and integration tests passed (337 tests total)
  - `cargo clippy --all-targets -- -D warnings`: 0 warnings
  - `cargo fmt -- --check`: clean
