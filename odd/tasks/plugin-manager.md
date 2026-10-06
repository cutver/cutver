# Feature: PluginManager and Declarative Config Orchestration (Issue #145)

> **Status**: In Progress  
> **Target Version**: v0.11.0  
> **Issue**: [#145](https://github.com/cutver/cutver/issues/145)  
> **Constitutional Law**: [CONTRACT.md](../../CONTRACT.md) (Pillars I, III, V, VI)

---

## Acceptance Criteria
- [x] Parse `[plugins]` table from `cutver.toml` into strongly-typed `HashMap<PluginName, PluginConfig>` within `Config`.
- [x] Implement `PluginManager` in `src/plugin/manager.rs` adhering to Facade pattern and strict function line budgets (<= 35-40 lines).
- [x] Support polymorphic driver instantiation (`Box<dyn PluginDriver>`) for `runtime = "process"` (and stub/feature-gated `wasm`).
- [x] Validate declared capabilities (`Capability::ManifestV1`, `Capability::LifecycleV1`, `Capability::ChangelogV1`, etc.) prior to invocation.
- [x] Provide strongly-typed dispatch methods consuming/producing DTOs from `src/plugin/dto/`.
- [x] Model actionable errors in `src/plugin/error.rs` (`PluginNotFound`, `MissingConfiguration`, `WasmNotSupported`).
- [x] Comprehensive unit tests covering config parsing, driver registration, capability enforcement, and DTO dispatch.
- [x] Zero clippy warnings (`-D warnings`), canonical formatting (`cargo fmt`), green test suite.

---

## Task Breakdown
- [x] Task 1: Integrate `plugins` table in `Config` (`src/config/types.rs`) with `PluginName` and `PluginConfig`.
- [x] Task 2: Extend `PluginError` with actionable variants for lookup and driver configuration.
- [x] Task 3: Create `src/plugin/manager.rs` with driver registration and polymorphic instantiation (`PluginManager::from_config`).
- [x] Task 4: Implement typed dispatch methods for manifest, lifecycle, and changelog DTOs.
- [x] Task 5: Add comprehensive unit tests in `src/plugin/manager.rs` and wire into `src/plugin.rs`.
- [x] Task 6: Verify full test suite, clippy, fmt, and close with atomic Conventional Commit linked to #145.

---

## Evidence & Verification
- **Commit**: `38c75bc` (`feat(plugin): implement PluginManager and declarative config orchestration (#145)`)
- **Pull Request**: [#148](https://github.com/cutver/cutver/pull/148)
- **Unit & Integration Tests**: 315 library tests, 25 e2e_bump, 12 e2e_changelog_cli, 7 e2e_changelog_template, 12 e2e_conventional, 3 e2e_doctor, 9 e2e_init, 11 e2e_lifecycle, 6 e2e_open, 5 e2e_style, 1 test_context - all passed.
- **Quality Gates**: `cargo clippy --all-targets -- -D warnings` (0 warnings), `cargo fmt -- --check` clean.
- **RDD Review Receipt**: Lineage `review-db7b5c4d72f19379`, lens `review-reliability` approved, authority burned (`gentle-ai.review-acknowledged/v1`).

