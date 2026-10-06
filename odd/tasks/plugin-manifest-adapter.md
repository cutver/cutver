# Feature: Wire manifest.v1 Capability into Manifest Discovery and Mutation Pipeline (Issue #153)

> **Status**: Completed  
> **Target Version**: v0.12.0  
> **Issue**: [#153](https://github.com/cutver/cutver/issues/153)  
> **Constitutional Law**: [CONTRACT.md](../../CONTRACT.md) (Pillars I, II, III, IV, VI)

---

## Acceptance Criteria
- [x] Add `ManifestKind::Plugin { plugin: Option<String> }` to `src/config/types.rs` supporting `[[manifest]] kind = "plugin"` declaring explicit `plugin = "<name>"` or omitting `plugin` for automatic glob binding.
- [x] Implement `resolve_plugin_for_manifest` in `PluginManager` supporting wildcard glob patterns across `PluginConfig.manifest_match` (Pillar IV.3 zero extra dependencies, using std/regex).
- [x] Add rich, actionable `PluginError` variants for missing capability, ambiguous plugin matches, and unmatched manifests (Pillar III.3).
- [x] Implement `PluginManifestEditor` in `src/manifest/plugin.rs` conforming to `crate::manifest::ManifestEditor`, delegating to `PluginManager::dispatch_manifest_read` and `PluginManager::dispatch_manifest_write`.
- [x] Wire `PluginManager` into `src/bump/exec/manifest.rs` (`current_source`, `read_manifest`, `compute`), `pipeline.rs`, and `doctor.rs` with `Arc<PluginManager>` safe across Rayon concurrency boundaries.
- [x] Enforce surgical diffs and two-phase mutation safety within `MutationTransaction` (Pillars I & II).
- [x] Integration and regression tests verifying third-party manifest synchronization with zero disk corruption on failure.
- [x] Zero clippy warnings with and without `--features plugins`, canonical formatting, and green test suite.

---

## Task Breakdown
- [x] Task 1: Add `ManifestKind::Plugin { plugin: Option<String> }` in `src/config/types.rs` with serde and validation tests.
- [x] Task 2: Implement glob pattern matching and `PluginManager::resolve_plugin_for_manifest` with actionable `PluginError`s.
- [x] Task 3: Implement `PluginManifestEditor` in `src/manifest/plugin.rs` and update `src/manifest/mod.rs` (`editor_for` / `editor_for_with_manager`).
- [x] Task 4: Integrate `PluginManager` into manifest execution pipeline (`src/bump/exec/manifest.rs`, `pipeline.rs`, `doctor.rs`).
- [x] Task 5: Add unit and integration tests covering process plugin manifest read/write, glob matching, and RAII rollback on failure.
- [x] Task 6: Full verification suite (`cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt -- --check`, `cargo test`, `cargo test --features plugins`).

---

## Evidence & Verification
- **Branch**: `feat/plugin-manifest-adapter-153`
- **Issue**: [#153](https://github.com/cutver/cutver/issues/153)
- **Validation**:
  - `cargo test`: passed (324 unit tests, all e2e suites green)
  - `cargo test --features plugins`: passed (329 unit tests, all e2e suites green)
  - `cargo clippy --all-targets --all-features -- -D warnings`: passed (0 warnings)
  - `cargo fmt -- --check`: passed (clean formatting)
- **RDD Receipt**: Lineage `review-a053d9c404d3bb16`, state `approved`, authority burned with `burn_evidence: gentle-ai.review-acknowledged/v1`.

