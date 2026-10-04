# Bugfix: Enable Latest Tag on Main Dispatch in Container Workflow

Goal: Update `.github/workflows/container.yml` so that `latest` (and version tags) are generated when the container workflow runs on `main` via `workflow_dispatch`, and trigger the initial GHCR image publication.

## Tasks
- [x] Task 1: Update `.github/workflows/container.yml` metadata tags configuration to generate `latest` on `main`
- [x] Task 2: Verify workflow YAML and test suite
- [x] Task 3: Commit and record verification evidence

## Verification Evidence
- Workflow metadata action tag configuration updated:
  `type=raw,value=latest,enable=${{ github.event_name == 'release' || github.ref == 'refs/heads/main' }}`
- Workflow structure validated: YAML parsed cleanly without syntax errors
- `cargo fmt -- --check`: passed (clean, exit 0)
- `cargo clippy --all-targets -- -D warnings`: passed (clean, exit 0)
- `cargo test`: 384 tests passed (0 failed)

