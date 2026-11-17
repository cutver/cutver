# doctor --check-changelog honours the tag prefix too

Feature: the remaining half of audit F-13. `cutver doctor --check-changelog` compares tags with the changelog
through the configured `[git] tag_prefix` instead of the legacy hardcoded `v`. Branch
`fix/doctor-changelog-prefix`, base `origin/main` at `3e61c42`.

## Specs

S1. With `[git] tag_prefix = "release-"`, tags `release-1.1.0` and `release-1.2.0` and a changelog whose
headings carry those versions, `cutver doctor --check-changelog` reports no drift and exits `0`. Today it
lists versions through the legacy hardcoded-`v` path (`src/bump/exec/doctor.rs:129`) and reports
`Changelog drift detected: Missing in changelog … release-1.1.0, release-1.2.0`.

S2. A tag whose version genuinely has no heading is still reported as drift, with the current message and the
current exit code — the fix widens what matches, not what passes silently.

S3. With the default `v` prefix the output is unchanged, byte for byte, and the existing doctor tests stay
green (`cli::runner::tests::doctor_with_check_changelog_drift_missing_in_changelog` included).

S4. The audit record stops calling F-13 partially open: the F-13 row is closed once both halves are fixed,
naming the commit that closed each half.

## Tasks

| ID | Spec | What | Route | Commit |
| --- | --- | --- | --- | --- |
| T1 | S1 | Route doctor's changelog listing through the prefix-aware extraction the changelog read surface already uses. | worker | pending |
| T2 | S1, S2 | Pin both directions: a custom prefix with matching headings reports no drift; a genuinely missing version is still reported. | worker | pending |
| T3 | S3 | Pin the default-prefix output unchanged. | worker | pending |
| T4 | S4 | Close the F-13 row in the audit, naming both commits. | worker | pending |

## Constraints for the worker

- Prefer Windows-runnable tests: the doctor's own test module in `src/bump/exec/doctor.rs` or the existing
  `src/cli/runner/tests.rs` doctor tests. `tests/e2e_changelog_cli.rs` and friends are `#![cfg(unix)]` and run
  zero tests on this host.
- Out of scope: F-12's other halves (manifest, doctor's `.unwrap_or_default()` I/O read, process, wasm), F-14,
  F-15, F-16. If the fix seems to need one of them, stop and report.

## Log

L1. User, verbatim: «Sí, cuando marque verde mergea y sigue con el siguiente slic»

L2. Decision (parent): continue the same defect rather than open a new theme. The independent verification of
`#312` measured this half explicitly — with `tag_prefix = "release-"` and tags `release-1.1.0`/`release-1.2.0`,
`doctor --check-changelog` exits `2` with `Missing in changelog … release-1.1.0, release-1.2.0` while
`changelog latest` now exits `0`. Two commands disagreeing about the same repository is worse than either
being wrong alone, so this closes the disagreement before anything else.

L3. CI caught what no local gate covered: the documentation line ceiling. The doctor and changelog additions
took `docs/reference/cli.md` to 205 lines, and that ceiling is a shell loop inside the `test` job
(`.github/workflows/ci.yml`), which `tests/ci_workflow_contract.rs` cannot see as a step. Fixed by
trimming to 197 lines, and closed as a class by mirroring the loop as a local test
(`every_document_stays_under_the_line_ceiling`), proven to bite at 202 lines and pass at 197.
