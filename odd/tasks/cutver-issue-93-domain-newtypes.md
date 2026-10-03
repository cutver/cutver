# Feature: Introduce TagName and TagPrefix Newtypes & Unify Tag Normalization (Issue #93)

Goal: Implement Phase 1 of the hardening roadmap by eliminating primitive obsession for Git tags and unifying scattered tag normalization heuristics under `CONTRACT.md` Pillars III.1 and V.6.

## Tasks
- [x] Task 1: Implement `TagName` and `TagPrefix` domain newtypes with Git ref validation and unit tests.
- [x] Task 2: Unify `normalize_tag` logic across `src/bump/exec.rs`, `src/cli/open.rs`, and `src/cli/runner.rs` using canonical `TagName` / `TagPrefix` methods.
- [x] Task 3: Comprehensive verification and quality gates (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Completion Evidence
- **Domain Newtypes**: Added `TagError`, `TagPrefix`, and `TagName` to `src/git.rs` with `git-check-ref-format` validation, `Deref`, `Display`, `AsRef`, `TryFrom`, `FromStr`, `format_tag`, `normalize_version`, `strip_prefix`, and `is_floating_major`.
- **Logic Unification**:
  - `src/bump/exec.rs`: Removed local `normalize_tag`, parsing git tags into `TagName` and normalizing via `TagName::normalize_version`.
  - `src/cli/open.rs`: Removed local `normalize_tag`, formatting browser compare and release tags via `TagPrefix::format_tag`.
  - `src/cli/runner.rs`: Refactored `resolve_release_tag_and_prefix` to return `(TagName, TagPrefix)` using domain newtypes instead of ad-hoc slicing.
  - `src/git.rs`: Refactored standalone `is_floating_major_tag` helper to delegate to `TagName::is_floating_major`.
- **Quality Gates**:
  - `cargo test --lib`: 272 tests passed.
  - `cargo test`: All 272 lib unit tests, e2e bump/changelog/conventional/doctor/init/lifecycle/open/style tests passed.
  - `cargo clippy --all-targets -- -D warnings`: Clean, 0 warnings.
  - `cargo fmt -- --check`: Formatting verified cleanly.

