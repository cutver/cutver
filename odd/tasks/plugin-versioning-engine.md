# Feature: Introduce versioning.v1 DTOs and Pluggable Bump Strategy Engine (Issue #155)

> **Status**: Completed  
> **Target Version**: v0.12.0  
> **Issue**: [#155](https://github.com/cutver/cutver/issues/155)  
> **Constitutional Law**: [CONTRACT.md](../../CONTRACT.md) (Pillars I, III, IV, VI)

---

## Acceptance Criteria
- [x] Define `VersioningComputeRequest` and `VersioningComputeResponse` in `src/plugin/dto/versioning.rs` and export in `src/plugin/dto.rs`.
- [x] Add `plugin: Option<String>` to `VersionSection` in `src/config/types.rs` supporting `[version] strategy = "plugin"` and optional `plugin = "<name>"`.
- [x] Implement `resolve_versioning_plugin` in `PluginManager` with rich actionable `PluginError` variants (`NoVersioningPlugin`, `AmbiguousVersioningPlugin`) per CONTRACT.md Pillar III.3.
- [x] Implement `PluginManager::dispatch_versioning` delegating through `dispatch_raw` with `Capability::VersioningV1`.
- [x] Wire pluggable version calculation into `resolve_next_version` in `src/bump/exec/pipeline.rs`.
- [x] Enforce fail-closed validation: verify calculated version parses as valid SemVer; abort cleanly without disk mutation on failure (Pillars I & II).
- [x] Add unit tests in `src/plugin/manager.rs`, `src/config/types.rs` and E2E integration tests in `tests/versioning_plugin.rs`.
- [x] Zero clippy warnings with and without `--features plugins`, canonical formatting, and green test suite.

---

## Task Breakdown
- [x] Task 1: Create `src/plugin/dto/versioning.rs` with `VersioningComputeRequest` / `VersioningComputeResponse` and serde tests.
- [x] Task 2: Update `VersionSection` in `src/config/types.rs` supporting `strategy = "plugin"` and `plugin: Option<String>`.
- [x] Task 3: Implement `resolve_versioning_plugin` and `dispatch_versioning` in `PluginManager` with actionable `PluginError`s.
- [x] Task 4: Integrate pluggable version calculation into `resolve_next_version` in `src/bump/exec/pipeline.rs`.
- [x] Task 5: Add integration tests in `tests/versioning_plugin.rs` (explicit plugin, automatic resolution, invalid version fail-closed).
- [x] Task 6: Full verification suite (`cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt -- --check`, `cargo test`, `cargo test --features plugins`).

---

## Evidence & Verification
- **Branch**: `feat/plugin-versioning-engine-155`
- **Issue**: [#155](https://github.com/cutver/cutver/issues/155)
- **Validation**:
  - `cargo test`: passed (336 unit tests, all e2e suites green)
  - `cargo test --features plugins`: passed (336 unit tests, all e2e suites green)
  - `cargo clippy --all-targets --all-features -- -D warnings`: passed (0 warnings)
  - `cargo fmt -- --check`: passed (clean formatting)
- **RDD Receipt**: Lineage `review-1bf2fbbea74c6c39`, state `approved`, authority burned with `burn_evidence: gentle-ai.review-acknowledged/v1`.


