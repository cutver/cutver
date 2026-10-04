# Feature: Modular Human-Centered Documentation Architecture (Cognitive Doc Design)

Goal: Restructure `cutver` documentation using the Diátaxis taxonomy and Cognitive Doc Design principles. Establish focused subfolders in `docs/` (`guides/`, `reference/`, `templates/`, `integrations/`, `internals/`), write modular guides with a Direct Companion tone (human-first, domain-driven language, zero internal library leaks, clean semantic links without visible file paths), and streamline `README.md` into a fast, low-cognitive-load landing portal.

## Tasks

- [x] Task 1: Create structured subfolders in `docs/` (`guides/`, `reference/`, `templates/`, `integrations/`, `internals/`), move existing internal docs (`design.md`, `references.md`) to `docs/internals/`, and author focused guides in Direct Companion tone
- [x] Task 2: Refactor `README.md` into an approachable, concise landing portal (~120-150 lines) featuring core value proposition, 3-command happy path, installation options, and human-friendly navigation routing table
- [x] Task 3: Audit all internal markdown links across repository, verify test suites (`cargo test`), clippy, and code formatting
- [x] Task 4: Finalize work-unit commits and record verification evidence in tracking documents

## Verification Evidence

| Task | Commit | Checks |
| --- | --- | --- |
| 1 | 9797e73 | Subfolder organization created (`guides/`, `reference/`, `templates/`, `integrations/`, `internals/`). Moved `design.md` and `references.md` to `docs/internals/`. Authored `getting-started.md`, `monorepos.md`, `configuration.md`, `manifests.md`, `cli.md`, `release-notes.md`, `github-actions.md`, and `containers.md` adhering to Direct Companion tone and quantitative line budgets (110-150 lines). |
| 2 | 9797e73 | Streamlined `README.md` to 140 lines featuring 30-second mental model, 8-platform binary table, 3-step happy path, and semantic navigation routing table without raw file paths. |
| 3 | 9797e73 | Link verification audit passed (0 broken links, 0 raw file path labels). `cargo test` (374 tests passed: 294 unit + 80 E2E), `cargo clippy --all-targets -- -D warnings` (passed), `cargo fmt -- --check` (passed). |
| 4 | 9797e73 | Work-unit commit `9797e73` on branch `feat/modular-cognitive-docs`. |
