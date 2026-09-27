# Feature: Universal MiniJinja Interpolation in Git & Hooks (Issue #43 Phase 2)

Goal: Implement universal MiniJinja evaluation for `git.commit_message`, `hooks.post_bump`, and `publish.commands`, backed by an enriched interpolation context with full backward compatibility for rustic `{version}` and `{tag}` tokens.

## Tasks
- [x] 1. Design and implement `InterpolationContext` and centralized MiniJinja interpolation helper with `{version}` / `{tag}` backward compatibility
- [x] 2. Integrate MiniJinja interpolation into `git.commit_message` in `src/bump/exec.rs` and `src/git.rs`
- [x] 3. Integrate MiniJinja interpolation into `hooks.post_bump` and `publish.commands` in `src/bump/exec.rs`
- [x] 4. Add comprehensive unit and integration tests verifying MiniJinja expressions (conditionals, forge variables, SemVer parts, env) and legacy `{version}` / `{tag}` formatting, with zero clippy warnings
