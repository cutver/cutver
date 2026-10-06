# Feature: WasmDriver with Extism Runtime Behind Feature Flag (Issue #147)

> **Status**: In Progress  
> **Target Version**: v0.11.0  
> **Issue**: [#147](https://github.com/cutver/cutver/issues/147)  
> **Constitutional Law**: [CONTRACT.md](../../CONTRACT.md) (Pillars I, III, V, VI)

---

## Acceptance Criteria
- [x] Add `extism` and `sha2` optional dependencies gated behind `[features] plugins = ["dep:extism", "dep:sha2"]` in `Cargo.toml`.
- [x] Implement `WasmDriver` in `src/plugin/driver/wasm.rs` conforming to `PluginDriver`.
- [x] Zero binary size inflation for default builds without `--features plugins`.
- [x] Enforce explicit capability & permission sandbox grants (filesystem paths, network domains, env vars from `PermissionsConfig`).
- [x] Validate SHA-256 integrity hash for configured `.wasm` binary sources.
- [x] Implement local cache resolution (`~/.cache/cutver/plugins/`) for plugin artifacts.
- [x] Wire `WasmDriver` into `PluginManager::from_config` under `#[cfg(feature = "plugins")]`.
- [x] Unit & regression tests covering integrity check, capability validation, and execution.
- [x] Zero clippy warnings with and without `--features plugins`, canonical formatting, and green tests.

---

## Task Breakdown
- [x] Task 1: Add `plugins` feature flag with optional `extism` and `sha2` in `Cargo.toml`.
- [x] Task 2: Implement `src/plugin/driver/wasm.rs` with Extism manifest and permission isolation.
- [x] Task 3: Implement SHA-256 integrity verification and artifact caching.
- [x] Task 4: Integrate `WasmDriver` into `src/plugin/driver.rs` and `src/plugin/manager.rs`.
- [x] Task 5: Add comprehensive unit tests in `wasm.rs` and `manager.rs`.
- [x] Task 6: Verify dual compilation (`cargo test` and `cargo test --features plugins`), clippy, fmt, and atomic commit linked to #147.

---

## Evidence & Verification
- **Commit**: `028077a` (`feat(plugin): implement WasmDriver with Extism runtime behind feature flag (#147)`)
- **Pull Request**: [#150](https://github.com/cutver/cutver/pull/150)
- **Unit & Integration Tests**: 316 passed (default), 321 passed (`--features plugins`).
- **Quality Gates**: `cargo clippy --all-targets --all-features -- -D warnings` (0 warnings), `cargo fmt -- --check` clean.
- **RDD Disposition**: Candidate-scoped decline for relay transport stream timeout on 3,200-line `Cargo.lock` diff. RDD remains globally and clone-locally enabled for future candidates.

