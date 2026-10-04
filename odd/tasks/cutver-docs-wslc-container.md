# Feature: Document WSL Containers (WSLC) in Container Integration Guide

Goal: Document native execution of `cutver` container images on Windows using WSL Containers (`wslc.exe` / `container.exe`), ensuring the guide adheres to Cognitive Doc Design line budgets (110-150 lines) and Direct Companion tone.

## Tasks
- [x] Task 1: Update `docs/integrations/containers.md` to include WSL Containers (`wslc`) local execution examples
- [x] Task 2: Verify line budget ($\le 150$ lines), semantic links, and formatting
- [x] Task 3: Commit and record verification evidence

## Verification Evidence
- Line count check: `wc -l docs/integrations/containers.md` -> 125 lines (within budget 110-150 lines).
- Formatting check: `cargo fmt -- --check` -> pass (clean exit 0).
- Linter check: `cargo clippy --all-targets -- -D warnings` -> pass (clean exit 0).
- Test suite: `cargo test` -> pass (294 unit tests, 75 integration tests passed).
- Semantic relative links verified:
  - `docs/integrations/github-actions.md` exists
  - `docs/reference/cli.md` exists
  - `docs/reference/configuration.md` exists
