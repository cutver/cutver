# Feature: Enriched ReleaseContext & MiniJinja Environment (Issue #43 Phase 1)

Goal: Enrich `ReleaseContext` with granular SemVer components (`major`, `minor`, `patch`, `prerelease`, `build`), repository forge metadata (`owner`, `repo`, `forge`), and register an `env()` global function in MiniJinja, adhering to the non-negotiable architectural contract.

## Tasks
- [x] 1. Add SemVer breakdown fields (`major`, `minor`, `patch`, `prerelease`, `build`) to `ReleaseContext` in `src/changelog/context.rs`
- [x] 2. Add repository forge metadata parsing (`owner`, `repo`, `forge`) to `ReleaseContext` in `src/changelog/context.rs`
- [x] 3. Register `env(var_name, default="")` global function in MiniJinja environment in `src/changelog/render.rs`
- [x] 4. Add comprehensive unit tests for context fields and MiniJinja `env()` evaluation, and verify zero clippy warnings
