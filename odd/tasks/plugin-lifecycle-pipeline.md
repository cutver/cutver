# Feature: Wire Lifecycle Hooks into Two-Phase Mutation Pipeline (Issue #146)

> **Status**: In Progress  
> **Target Version**: v0.11.0  
> **Issue**: [#146](https://github.com/cutver/cutver/issues/146)  
> **Constitutional Law**: [CONTRACT.md](../../CONTRACT.md) (Pillars I, II, III, V, VI)

---

## Acceptance Criteria
- [x] Implement `plugins_for_event(&self, event: &str)` helper in `src/plugin/manager.rs` matching `Capability::LifecycleV1` and declared `events`.
- [x] Extend `src/bump/types.rs` with `Error::Plugin` and `Error::PreBumpRejected`.
- [x] Wire `on_pre_bump` hook into `src/bump/exec/pipeline.rs` during in-memory phase before opening `MutationTransaction` (fail-closed, aborts with pristine workspace).
- [x] Wire `on_post_bump` hook into `src/bump/exec/pipeline.rs` within `MutationTransaction` boundary before `git::stage` (fail-closed, triggers RAII drop rollback on failure).
- [x] Wire `on_post_release` hook into `src/bump/exec/pipeline.rs` after commit & tag creation.
- [x] Pass strongly-typed DTOs (`PreBumpPayload`, `PostBumpPayload`, `PostReleasePayload`) with dry-run support.
- [x] Unit & E2E integration tests in `tests/e2e_lifecycle.rs` verifying pre-bump abortion, post-bump rollback, and clean state.
- [x] Zero clippy warnings, canonical formatting, 100% green test suite.

---

## Task Breakdown
- [x] Task 1: Add event query helper `plugins_for_event` in `src/plugin/manager.rs` and extend `src/bump/types.rs` errors.
- [x] Task 2: Implement in-memory `on_pre_bump` hook execution in `src/bump/exec/pipeline.rs`.
- [x] Task 3: Implement transactional `on_post_bump` hook execution inside `MutationTransaction` boundary.
- [x] Task 4: Implement `on_post_release` hook execution after commit/tag.
- [x] Task 5: Add comprehensive E2E tests in `tests/e2e_lifecycle.rs` for pre-bump rejection and post-bump rollback.
- [x] Task 6: Verify full test suite, clippy `-D warnings`, fmt, and close with atomic Conventional Commit linked to #146.
