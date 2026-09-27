# Feature: Tree-Structured Release Plan Layout and Simulation Banner (Issue #69)

Goal: Upgrade the `Release Plan` output in `cutver bump` and `--dry-run` to an elegant tree hierarchy (`├──`, `└──`) with a dedicated simulation banner, adhering to `CONTRACT.md` (Pillar I, II, III, IV, and Pillar V: max 2 levels of nesting, function budget ≤ 35 lines, atomic helpers).

## Tasks
- [x] 1. Create atomic tree formatting helpers in `src/cli/tree.rs` adhering to Pillar V (max 2 levels nesting, functions ≤ 35 lines)
- [x] 2. Implement the simulation box banner for `--dry-run` with themed styling
- [x] 3. Refactor `print_bump_summary` to render the tree layout (Manifests, Preflight, Changelog, Git, Publish) with version diff highlights
- [x] 4. Update and add unit/integration tests verifying the tree structure across dry-run and live runs
- [x] 5. Run full verification suite (`cargo test --locked`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`) and record evidence

## Verification Evidence
- Commit: `5a59720` (`feat(cli): tree-structured release plan layout and simulation banner for 'cutver bump' (#69)`)
- `cargo test --locked`: Passed (316 tests passing across unit and integration suites).
- `cargo clippy --all-targets -- -D warnings`: Passed without warnings.
- `cargo fmt -- --check`: Passed clean formatting.
- Pillar V compliance: all functions in `src/cli/tree.rs` ≤ 21 lines, max nesting depth ≤ 2, zero unwrap in production code.
