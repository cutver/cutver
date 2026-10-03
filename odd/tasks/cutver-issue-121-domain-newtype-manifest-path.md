# Feature: Introduce ManifestPath Domain Newtype (Issue #121)

Goal: Implement the final mandatory domain newtype `ManifestPath` under `CONTRACT.md` Pillar III.1 and Pillar V.5, enforcing forward-slash normalization and non-empty path validation upon construction.

## Tasks
- [x] Task 1: Create `ManifestPath` domain newtype in `src/config/path.rs` with forward-slash normalization, validation, traits (`Deref`, `AsRef`, `Display`, `Serialize`, `Deserialize`, `PartialEq`), and unit tests.
- [x] Task 2: Adopt `ManifestPath` in `src/config/types.rs` (`Manifest.path`) and wire across `config`, `bump`, `init`, and `doctor`.
- [x] Task 3: Comprehensive verification and quality gates (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`).

## Completion Evidence
- Created `src/config/path.rs` implementing `ManifestPath` (*Parse, Don't Validate*):
  - Normalizes Windows backslashes (`\`) to forward slashes (`/`).
  - Normalizes leading `./` prefixes.
  - Strips Windows verbatim prefix (`\\?\`).
  - Validates non-empty paths via `ConfigError::EmptyManifestPath`.
  - Implements: `Deref<Target = str>`, `AsRef<str>`, `AsRef<Path>`, `Borrow<str>`, `Display`, `Clone`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Serialize`, `Deserialize`, `TryFrom<&str>`, `TryFrom<String>`, `FromStr`, `From<ManifestPath> for String`, `PartialEq<&str>`, `PartialEq<str>`, `PartialEq<String>`, `as_str()`, `as_path()`.
  - Unit tests covering normalization, invalid empty paths, ordering/hashing, and trait implementations.
- Updated `src/config/types.rs`:
  - Re-exported `ManifestPath`.
  - Added `ConfigError::EmptyManifestPath`.
  - Updated `Manifest.path: ManifestPath`.
- Re-exported `ManifestPath` in `src/config.rs`.
- Interoperability across `src/bump/exec/manifest.rs`, `src/bump/exec/doctor.rs`, `src/cli/doctor.rs`, and `src/config/discovery.rs`.
- Quality gates:
  - `cargo test`: 286 unit tests + all e2e suites passed 100%.
  - `cargo clippy --all-targets -- -D warnings`: passed with 0 warnings.
  - `cargo fmt -- --check`: passed cleanly.
  - 0 `unwrap`/`expect` in production code.

### Work-Unit Commit
- Commit: `cd87dc3` (`refactor(domain): introduce ManifestPath domain newtype and enforce forward-slash normalization (#121)`)
