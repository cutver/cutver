# Feature: Wire changelog.v1 Capability into Release Notes Formatting Pipeline (Issue #154)

> **Status**: Completed  
> **Target Version**: v0.12.0  
> **Issue**: [#154](https://github.com/cutver/cutver/issues/154)  
> **Constitutional Law**: [CONTRACT.md](../../CONTRACT.md) (Pillars I, III, IV, VI)

---

## Acceptance Criteria
- [x] Add `plugin: Option<String>` to `Changelog` in `src/config/types.rs` supporting `[changelog] format = "plugin"` and optional `plugin = "<name>"`.
- [x] Implement `resolve_changelog_plugin` in `PluginManager` with rich, actionable `PluginError` variants for missing or ambiguous plugins declaring `changelog.v1` (Pillar III.3).
- [x] Implement `ChangelogRenderRequest` assembly mapping `ReleaseContext` (commits, scopes, breaking changes, authors, forge URLs) into strongly-typed DTOs.
- [x] Wire `PluginManager::dispatch_changelog` into changelog rendering (`src/changelog/render/body.rs` and `src/bump/exec/changelog.rs`).
- [x] Enforce fail-closed validation: failures in changelog plugins abort before phase 2 and trigger RAII atomic rollback (Pillars I & II).
- [x] End-to-end integration tests in `tests/changelog_plugin.rs` verifying custom changelog formatting and rollback safety.
- [x] Zero clippy warnings with and without `--features plugins`, canonical formatting, and green test suite.

---

## Task Breakdown
- [x] Task 1: Update `src/config/types.rs` to support `[changelog] format = "plugin"` and `plugin: Option<String>`, with parsing tests.
- [x] Task 2: Implement `resolve_changelog_plugin` in `PluginManager` and add rich error variants in `src/plugin/error.rs`.
- [x] Task 3: Implement `ReleaseContext` to `ChangelogRenderRequest` conversion helper and update `src/changelog/render/body.rs`.
- [x] Task 4: Wire `PluginManager` through `ChangelogPlanParams` in `src/bump/exec/changelog.rs` and `src/bump/exec/pipeline.rs`.
- [x] Task 5: Add unit tests in `src/changelog/render/tests.rs` and integration tests in `tests/changelog_plugin.rs` (happy path & rollback).
- [x] Task 6: Full verification suite (`cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt -- --check`, `cargo test`, `cargo test --features plugins`).

---

## Evidence & Verification
- **Branch**: `feat/plugin-changelog-adapter-154`
- **Issue**: [#154](https://github.com/cutver/cutver/issues/154)
- **Commit**: `b8b5dfe` (`feat(plugin): wire changelog.v1 capability into release notes formatting pipeline (#154)`)
- **Pull Request**: [#157](https://github.com/cutver/cutver/pull/157)
- **Validation**:
  - `cargo test`: passed (332 unit tests, all e2e suites green)
  - `cargo test --features plugins`: passed (332 unit tests, all e2e suites green)
  - `cargo clippy --all-targets --all-features -- -D warnings`: passed (0 warnings)
  - `cargo fmt -- --check`: passed (clean formatting)
- **RDD Receipt**: Lineage `review-d894d3e6725590fa`, state `approved`, authority burned with `burn_evidence: gentle-ai.review-acknowledged/v1`.



