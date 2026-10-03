# Feature: Modularize Changelog Extraction Engine (Issue #111)

Goal: Decompose `src/changelog/extract.rs` (585 lines) into focused submodules under `src/changelog/extract/` adhering to `CONTRACT.md` Pillar V.4 (Module Budget $\le 300-400$ lines) behind a clean Facade in `src/changelog/extract.rs`.

## Tasks
- [x] Task 1: Decompose `src/changelog/extract.rs` into submodules under `src/changelog/extract/` (`parser.rs`, `io.rs`, `tests.rs`) with a thin Facade in `src/changelog/extract.rs`.
- [x] Task 2: Audit module line budgets ($\le 350-400$ lines per production file) and ensure 0 unwraps in production code.
- [x] Task 3: Comprehensive verification and quality gates (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Completion Evidence
- **Decomposed Architecture**:
  - `src/changelog/extract.rs`: Thin Facade (7 lines) declaring `pub mod io;`, `pub mod parser;`, `#[cfg(test)] mod tests;` and re-exporting `extract_latest`, `extract_version`, `list_versions`, `read_latest`, `read_version`.
  - `src/changelog/extract/parser.rs`: Pure in-memory parsing (152 lines) containing `extract_latest`, `extract_version`, `list_versions`, `is_release_heading`, `normalize_version`, `extract_heading_version`.
  - `src/changelog/extract/io.rs`: Filesystem reading helpers (35 lines) containing `read_latest`, `read_version`.
  - `src/changelog/extract/tests.rs`: Unit test suite (416 lines) testing latest/version extraction, unreleased handling, prefixes, filesystem reading, and internal heading helpers.
- **Line Budget & Safety Invariants**:
  - Production files: `extract.rs` (7 lines $\le 300$), `io.rs` (35 lines $\le 300$), `parser.rs` (152 lines $\le 300$). All well below the $\le 300$ target and $\le 400$ hard ceiling.
  - Zero `unwrap` or `expect` calls in production code.
- **Verification Gates**:
  - `cargo test`: 278 unit tests + all integration suites pass (100% green).
  - `cargo clippy --all-targets -- -D warnings`: Clean with 0 warnings.
  - `cargo fmt -- --check`: Canonical formatting strictly enforced with 0 diffs.

### Work-Unit Commit
- Commit: `c06f7e4` (`refactor(changelog): modularize historical extraction engine and tests (#111)`)
