# Feature: [project] Configuration Table and project_name Context Variable (Issue #82)

Goal: Introduce an optional `[project]` configuration table with `name = "..."` attribute in `cutver.toml`, expose `project_name` and `project` in `ReleaseContext` and `InterpolationContext` with clean fallback precedence (config -> git remote -> root directory), and dogfood in templates, adhering to `CONTRACT.md` Pillars I, II, III, IV, and V.

## Tasks
- [x] Task 1: Add `Project` struct (`pub name: Option<String>`) to `src/config/types.rs` and wire into `Config` as `#[serde(default)] pub project: Project`.
- [x] Task 2: Add `project_name: Option<String>` to `ReleaseContext` and `InterpolationContext` in `src/changelog/context.rs`, serialize aliases `project_name` and `project`, and wire resolution precedence.
- [x] Task 3: Dogfood `[project] name = "cutver"` in `cutver.toml`, and update `.github/templates/cutver/RELEASE.md` and `DEFAULT_RELEASE_TEMPLATE` in `src/init/scaffold.rs` (`## {{ project_name or repo }} [{{ tag }}] - {{ date }}`).
- [x] Task 4: Add unit and integration tests for `[project]` configuration parsing and `project_name` / `project` template evaluation.
- [x] Task 5: Verify all checks (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`, `cargo run -- doctor --check-changelog`).
