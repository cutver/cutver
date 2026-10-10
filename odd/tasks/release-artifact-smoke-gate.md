# Feature: Release artifact smoke gate (Issue #174)

> **Status**: In Progress
> **Target Version**: v0.13.0
> **Issue**: [#174](https://github.com/cutver/cutver/issues/174)
> **Constitutional Law**: [CONTRACT.md](../../CONTRACT.md)

---

## Problem

Nothing in the pipeline ever executes the binary it ships. `release.yml` and `release-assets.yml`
build, package and upload; `ci.yml` exercises the **debug** profile only. The single artifact that has
ever been executed after shipping is `cutver --version` in `release.yml`'s last two steps — after the
release already exists, on one platform, with no real logic.

The release profile changed in #170 (`opt-level = "z"`). If that had broken the artifact, every gate
would still have been green. The `Containerfile` has already broken silently in this repository once.

## Design decisions (parent-owned)

**D1 — A dedicated `smoke` job, `build` untouched.** The proven build matrix stays exactly as it is
(it has broken releases twice before: a missing toolchain and a stale floating action tag). The new
`smoke` job sits between `build` and `publish`, downloads each per-target asset artifact, extracts the
archive and executes the shipped binary. `publish` changes from `needs: [version, build]` to
`needs: [version, build, smoke]`, so an unexecuted artifact can never be released.

**D2 — Every artifact runs on a natively matching runner.** Executing the host's binary while claiming
to test the target's is the exact failure the issue guards against. Smoke matrix:

| slug | smoke runner |
| --- | --- |
| `linux-x86_64` | `ubuntu-latest` |
| `linux-musl-x86_64` | `ubuntu-latest` |
| `linux-arm64` | `ubuntu-24.04-arm` |
| `linux-musl-arm64` | `ubuntu-24.04-arm` |
| `macos-x86_64` | `macos-15-intel` |
| `macos-arm64` | `macos-latest` |
| `windows-x86_64` | `windows-latest` |
| `windows-arm64` | `windows-11-arm` |

Runner labels verified 2026-10-10: arm64 Linux and Windows hosted runners are generally available at
no cost in public repositories; `macos-13` was retired 2025-12-04 and `macos-15-intel` is the supported
x86_64 macOS label (available until August 2027).

**D3 — Real logic, not only `--version`.** The smoke asserts the printed version matches the released
version, then runs real commands against a hermetic fixture: `doctor` and a read-only changelog/release
plan command. The fixture is self-contained (its own minimal project, git repo and tag created at
runtime) so the result does not depend on the repository's own state.

**D4 — Fail closed.** No `continue-on-error`; a missing or unextractable artifact, a version mismatch
or a non-zero exit fails the job and therefore the release. No conditional that silently turns a
target's smoke into a no-op.

**D5 — The container gets the same check.** `container.yml` executes the published image (both
`linux/amd64` and `linux/arm64`; QEMU is already configured in that workflow) and runs the same smoke
inside it.

**D6 — Bounded cost.** Seconds per target; no test suite, no rebuild.

## Acceptance Criteria

- [ ] `release.yml` executes the built artifact at least once per matrix target, before upload.
- [x] The check runs real logic, not only `--version` (`doctor --check-changelog` + `changelog latest --json`).
- [x] A failure fails the release; no skipped-target silent green (no `continue-on-error`; `publish` needs `smoke`).
- [ ] Runs on every target, including both musl targets, executing that target's binary.
- [ ] Windows targets included (PATHEXT/batch resolution has bitten this repo before: #152).
- [x] The container image gets the same check (both `linux/amd64` and `linux/arm64`).
- [x] Cost stays bounded: the smoke is three short CLI invocations, no test suite, no rebuild.

The three unchecked boxes depend on Task 5 (proving the native runner labels); they stay unchecked
until that proof runs, then this list is updated.

## Task Breakdown

- [x] Task 1: Add the hermetic smoke fixture and the smoke runner logic (version match + real logic).
- [x] Task 2: Add the `smoke` job to `release.yml` and gate `publish` on it.
- [x] Task 3: Add the container smoke to `container.yml`.
- [x] Task 4: Verify — validate workflow YAML, execute the same smoke commands against a real built
      binary locally, and prove the script fails closed.
- [ ] Task 5 (parent-owned): prove the native runner labels before merge, then commit/push/PR.

## Verification

### Local, parent-run (2026-10-10)

| Command | Result |
| --- | --- |
| `sh .github/scripts/release-smoke/run.sh target/release/cutver 0.12.0` | exit 0 — `--version => cutver 0.12.0`, `OK 0.12.0` |
| same, expected `9.9.9` | exit 1 — `FAIL: version mismatch` |
| same, missing binary path | exit 1 — `FAIL: binary not found` |
| `yq -e '.'` on both workflows | exit 0, both valid |
| `grep -rn continue-on-error .github/workflows/` | none |
| `yq '.jobs.publish.needs'` | `[version, build, smoke]` |
| `git diff` on the `build` job | untouched |
| `yq -r '.jobs.smoke.strategy.matrix.include[]'` | the 8 slugs on the D2 native runners |

### Security/design review notes

- The container runtime image already ships `git` (`Containerfile`, `apk add git`) and the
  `safe.directory '*'` system config, so the smoke script can build its fixture inside the image from a
  read-only mount.
- The container smoke pulls the just-pushed image by digest and runs it under QEMU for `linux/arm64`.
  It is a post-push check by construction (buildx multi-arch cannot `load` before push); that matches
  D5, which asked for the same check, not for a pre-push one.
- `release-assets.yml` (the manual backfill path) is not covered here; it is a recovery workflow, not the
  release workflow, and stays out of scope.

### Not proven here

- The eight native runner labels (`ubuntu-24.04-arm`, `macos-15-intel`, `windows-11-arm`, and Git Bash on
  the Windows ARM image) — proved separately in Task 5.
- `release.yml` cannot be exercised by PR CI: it is `workflow_dispatch`-only.

## Evidence & Verification

- Files: `.github/scripts/release-smoke/run.sh`, `tests/fixtures/release-smoke/{cutver.toml,package.json,CHANGELOG.md}`,
  `.github/workflows/release.yml` (new `smoke` job, `publish.needs`), `.github/workflows/container.yml`
  (digest output + new `smoke` job).
- Commit / PR: pending (Task 5).
