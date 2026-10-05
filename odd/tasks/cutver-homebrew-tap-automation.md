# Feature: Automate Homebrew Tap Release Updates

Goal: Automate Homebrew tap formula synchronization directly in `.github/workflows/release.yml` upon Cutver releases using declarative asset slugs and pre-calculated SHA256 checksums, and immediately synchronize `Formula/cutver.rb` in `Row0902/homebrew-tap` to v0.10.0.

## Tasks

- [x] Task 1: Update `Formula/cutver.rb` in `Row0902/homebrew-tap` to v0.10.0 with accurate multi-platform checksums and disable failing `autobump.yml`
- [x] Task 2: Add automated Homebrew tap formula update step to `.github/workflows/release.yml` using `HOMEBREW_TAP_TOKEN`
- [x] Task 3: Verify workflow YAML integrity and repository health checks
- [x] Task 4: Finalize work-unit commits and record verification evidence

## Verification Evidence

| Task | Commit | Checks |
| --- | --- | --- |
| 1 | 2c7aca6 (homebrew-tap) | Updated `Formula/cutver.rb` to v0.10.0 with accurate checksums for macOS (arm64, x86_64) and Linux (musl x86_64); removed `.github/workflows/autobump.yml`; Homebrew CI `brew test-bot` passed in run 37336875281 |
| 2 | d188d4a | Step added to `release.yml` in job `publish` after release asset upload, resolving SHA-256 sums from `dist/SHA256SUMS` and updating `Formula/cutver.rb` |
| 3 | d188d4a | YAML syntax validated via `yaml-validator`; Rust suite passed cleanly (`cargo test`, 294 unit tests, 84 integration tests) |
| 4 | d188d4a | Work-unit commit on branch `ci/automate-homebrew-tap-release` |
