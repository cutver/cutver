# Feature: Changelog Template Hardening & MiniJinja Ergonomics (Issue #125)

Goal: Implement Issue #125 adhering strictly to `CONTRACT.md` Pillars I, II, III, IV, and V:
1. Resolve `config.changelog.template_file` against `config.root_dir` during configuration load, ensuring monorepo parity from any subdirectory.
2. Propagate template errors cleanly via `Result<String, Error>` in `src/changelog/render/body.rs`, eliminating swallowed HTML comments and aborting in Phase 1.
3. Expose `body` and `breaking_description` on `CommitContext`, and `year`, `month`, `day` on `ReleaseContext`.
4. Add native MiniJinja filters `group_by_scope` and `group_by_type`, plus global function `env(name)` in `src/changelog/render/template.rs`.
5. Align `cutver init` configuration generation to set `mode = "template"` when scaffolding the release template.
6. Full test suite verification and quality gates.

## Tasks
- [x] Task 1: Resolve `config.changelog.template_file` against `config.root_dir` in `src/config/discovery.rs` and set `mode = "template"` in `src/init/scaffold/config_gen.rs`.
- [x] Task 2: Make `render_body` and `render_body_with_context` return `Result<String, Error>` in `src/changelog/render/body.rs`, propagating errors and aborting in Phase 1.
- [x] Task 3: Expand `CommitContext` (`body`, `breaking_description`) and `ReleaseContext` (`year`, `month`, `day`) in `src/changelog/context/`.
- [x] Task 4: Implement `group_by_scope`, `group_by_type`, and `env` helpers in `src/changelog/render/template.rs` with unit tests.
- [x] Task 5: Comprehensive test suite verification, clippy, and formatting (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Completion Evidence
- `cargo test`: 294 unit tests and 86 integration tests passing across all suites.
- `cargo clippy --all-targets -- -D warnings`: 0 warnings.
- `cargo fmt -- --check`: strictly formatted.
- Template resolution tested against `config.root_dir` from nested subdirectories.
- Fail-fast Phase 1 changelog template verification tested via `tests/e2e_bump.rs`.
- Native MiniJinja filters `group_by_scope`, `group_by_type`, and `env` tested in `src/changelog/render/tests.rs`.
- `CommitContext` (`body`, `breaking_description`) and `ReleaseContext` (`year`, `month`, `day`) tested with JSON roundtripping in `src/changelog/context/tests.rs`.
