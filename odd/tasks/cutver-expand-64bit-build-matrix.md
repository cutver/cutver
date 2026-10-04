# Feature: Expand 64-Bit Release Build Matrix & Declarative Asset Naming

Goal: Expand the `release.yml` matrix to comprehensively cover all modern 64-bit operating systems and CPU architectures (Linux x86_64, Linux ARM64, macOS Intel, macOS Apple Silicon, Windows x86_64, Windows ARM64) and adopt human-friendly declarative asset naming (`cutver-<version>-<os>-<arch>.<ext>`).

## Tasks

- [x] Task 1: Update `.github/workflows/release.yml` with the 8-target 64-bit matrix, cross-compilation toolchains, declarative asset slug names, and packaging steps
- [x] Task 2: Update `README.md` to document the full 64-bit platform matrix and declarative asset naming conventions
- [x] Task 3: Verify workflow YAML integrity and run project checks (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`)
- [ ] Task 4: Finalize work-unit commits and record verification evidence

## Verification Evidence

| Task | Commit | Checks |
| --- | --- | --- |
| 1 | working tree | Workflow YAML structure verified: 8 64-bit matrix targets (`linux-x86_64`, `linux-musl-x86_64`, `linux-arm64`, `linux-musl-arm64`, `macos-x86_64`, `macos-arm64`, `windows-x86_64`, `windows-arm64`), cross build and toolchain conditional steps, packaging using `OUT="cutver-${VERSION}-${{ matrix.slug }}"`, artifact upload `name: asset-${{ matrix.slug }}` |
| 2 | working tree | `README.md` updated with precompiled binaries table documenting 8 64-bit platforms, target slugs, target triples, archive formats, and sample declarative naming strings |
| 3 | working tree | `cargo test` (294 unit tests, 25 e2e_bump, 12 e2e_changelog_cli, 7 e2e_changelog_template, 12 e2e_conventional, 3 e2e_doctor, 9 e2e_init, 11 e2e_lifecycle, 6 e2e_open, 5 e2e_style, 1 test_context - all passed), `cargo clippy --all-targets -- -D warnings` (passed clean), `cargo fmt -- --check` (clean) |
| 4 | pending | Work-unit commit on feature branch |
