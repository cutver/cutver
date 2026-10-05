# Feature: Automate Homebrew Tap & Scoop Bucket Release Updates

Goal: Automate Homebrew tap formula and Scoop bucket manifest synchronization directly in `.github/workflows/release.yml` upon Cutver releases using declarative asset slugs and pre-calculated SHA256 checksums, authenticated via `cutver-release[bot]` (with `HOMEBREW_TAP_TOKEN` fallback), and immediately synchronize `Formula/cutver.rb` in `Row0902/homebrew-tap` to v0.10.0.

## Tasks

- [x] Task 1: Update `Formula/cutver.rb` in `Row0902/homebrew-tap` to v0.10.0 with accurate multi-platform checksums and disable failing `autobump.yml`
- [x] Task 2: Add automated Homebrew tap formula update step to `.github/workflows/release.yml`
- [x] Task 3: Add automated Scoop bucket manifest update step to `.github/workflows/release.yml`
- [x] Task 4: Integrate `cutver-release[bot]` GitHub App token minting (`actions/create-github-app-token@v3`) with graceful fallback
- [x] Task 5: Verify workflow YAML integrity and repository health checks
- [x] Task 6: Finalize work-unit commits and record verification evidence

## Verification Evidence

| Task | Commit | Checks |
| --- | --- | --- |
| 1 | 2c7aca6 (homebrew-tap) | Updated `Formula/cutver.rb` to v0.10.0 with accurate checksums for macOS (arm64, x86_64) and Linux (musl x86_64); removed `.github/workflows/autobump.yml`; Homebrew CI `brew test-bot` passed in run 37336875281 |
| 2 | fa92f87 | Step added to `release.yml` in job `publish` after release asset upload, resolving SHA-256 sums from `dist/SHA256SUMS` and updating `Formula/cutver.rb` |
| 3 | 89d90ed | Step added to `release.yml` in job `publish` updating `bucket/cutver.json` in `Row0902/scoop-bucket` with 64bit and arm64 architecture targets |
| 4 | 9cb7d15 | Integrated `actions/create-github-app-token@v3` generating ephemeral tokens for `Row0902` repositories; commits authored as `cutver-release[bot]` |
| 5 | 9cb7d15 | YAML syntax validated via `yaml-validator`; Rust suite passed cleanly (`cargo test`, 294 unit tests, 84 integration tests) |
| 6 | 9cb7d15 | Work-unit commit on branch `ci/automate-homebrew-tap-release` |
