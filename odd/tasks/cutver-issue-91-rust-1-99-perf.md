# Feature: Adopt Rust 1.99 In-Place Zero-Allocation String Conversions & Set MSRV (Issue #91)

Goal: Modernize `cutver` with Rust 1.99.0 zero-allocation string conversion APIs in `src/git.rs` and establish formal MSRV in `Cargo.toml`, adhering strictly to `CONTRACT.md` Pillars I, II, III, IV, and V.

## Tasks
- [x] Task 1: Set `rust-version = "1.99.0"` in `Cargo.toml`.
- [x] Task 2: Replace borrowed `String::from_utf8_lossy(&output.stdout)` in `src/git.rs` with in-place `String::from_utf8_lossy_owned(output.stdout)` and zero-allocation whitespace checking.
- [x] Task 3: Comprehensive verification and quality gates (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Completion Evidence
- Set `rust-version = "1.99.0"` under `[package]` in `Cargo.toml`.
- Replaced borrowed `String::from_utf8_lossy(&output.stdout).trim().is_empty()` with `output.stdout.iter().any(|b| !b.is_ascii_whitespace())` in `tag_exists` (`src/git.rs`).
- Replaced borrowed `String::from_utf8_lossy` with `String::from_utf8_lossy_owned` across `has_remote`, `remote_tag_exists`, `latest_tag`, and `list_authors_between` in `src/git.rs`.
- `cargo test --lib` passed (271 tests ok).
- `cargo test` passed (all 271 unit tests and all integration tests passed).
- `cargo clippy --all-targets -- -D warnings` passed cleanly.
- `cargo fmt -- --check` passed cleanly.
