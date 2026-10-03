# Feature: Parallelize Read-Only Manifest Compute, Drift Checks & Directory Discovery with Rayon (Issue #99)

Goal: Implement Phase 4 of the hardening roadmap by adopting `rayon` for data-parallel, read-only manifest AST evaluation and monorepo directory discovery under `CONTRACT.md` Pillars I.3 and IV.1.

## Tasks
- [x] Task 1: Add `rayon = "1"` dependency to `Cargo.toml`.
- [x] Task 2: Parallelize `compute()` and `doctor()` in `src/bump/exec.rs` using `rayon::par_iter()`, preserving deterministic manifest ordering.
- [x] Task 3: Parallelize child directory subtree traversal in `src/init/discovery.rs` using `rayon`, maintaining deterministic sorting.
- [x] Task 4: Comprehensive verification and quality gates (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Completion Evidence
- Added `rayon = "1"` to Cargo.toml.
- `src/bump/exec.rs`:
  - `compute()` uses `rayon::prelude::*` with `config.manifest.par_iter().map(...).collect()`, evaluating AST transformations concurrently while guaranteeing manifest order preservation.
  - `doctor()` uses `config.manifest.par_iter().filter(...).map(...).collect::<Result<Vec<Option<Drift>>, Error>>()`, flattening into `Vec<Drift>` deterministically.
- `src/init/discovery.rs`:
  - Subdirectory recursive scanning in `scan_dir()` parallelized via `subdirs.par_iter().map(...)`, merging discovered manifests and ecosystem hints into parent collections deterministically.
  - Existing `sort_manifests()` in `discover_project()` guarantees deterministic output ordering.
- All verification gates passed cleanly:
  - `cargo test --lib` (275 tests passed)
  - `cargo test --test e2e_init` (9 tests passed)
  - `cargo test --test e2e_bump` (23 tests passed)
  - `cargo test --test e2e_doctor` (3 tests passed)
  - `cargo test` (337 tests total passed across lib, unit, and e2e integration suites)
  - `cargo clippy --all-targets -- -D warnings` (0 warnings)
  - `cargo fmt -- --check` (clean canonical formatting)
