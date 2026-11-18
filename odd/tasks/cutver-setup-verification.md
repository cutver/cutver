# Verify the setup download, and pin the action by commit

Feature: issue #322. `cutver/setup` verifies the archive it downloads against the release's `SHA256SUMS`
before extracting it, fails closed when the checksum asset is missing, and this repository pins the action by
commit instead of the floating `v1` tag. Branch `fix/setup-pin`, base `origin/main` at `9f8abaa`.

## Specs

S1. After downloading an archive and before extracting it, the action fetches that release's `SHA256SUMS`,
computes the archive's SHA-256 and compares. On a mismatch the step fails with the expected and the actual
digest, and extracts nothing.

S2. When the release has no `SHA256SUMS` asset, the step fails closed unless the caller set
`verify-checksum: false`. The default verifies, so an existing workflow gains the check without editing
anything.

S3. The comparison is a pure function with its own test, and the wiring is pinned too: verification runs
before extraction, a failure prevents extraction, and the legacy fallback verifies the legacy asset name.

S4. This repository pins `cutver/setup` by commit in `.github/workflows/release.yml`, and a test fails if the
reference drifts back to a tag. The documentation states the real input and output set, what verification
covers, and what it does not.

S5. Nothing else changes: the cache path, the legacy-asset fallback, the extraction and the `PATH` handling
keep their behaviour, and the skills catalog stops describing the action as unverified.

## Tasks

| ID | Spec | What | Route | Commit |
| --- | --- | --- | --- | --- |
| T1 | S1-S3 | Verification in `cutver/setup`: pure module, its tests, the wiring tests, the opt-out input, a rebuilt `dist/`. | worker | `f5750f1` in `cutver/setup` |
| T2 | S4 | Pin the action by commit here, plus the test that fails on a tag. | inline | `1e07c84` |
| T3 | S4 | Document the real inputs (`version`, `github-token`, `cache`, `plugins`, `verify-checksum`) and the `installed-version` output. | inline | `1e07c84` |
| T4 | S5 | Update the skills catalog and the installed copy. | inline | `8f9d1a9` in `cutver/skills` |

## Log

L1. User, verbatim: «crea el issue de cutver/setup»

L2. User, verbatim: «cuando cierres el 321 ataca de una el 322»

L3. User, verbatim: «Termina con el issue 322 que te haré una pregunta final para cerrar por hoy»

L4. Evidence before the change: `setup/src/services/installer.ts` downloaded with `tc.downloadTool` and
extracted with no digest check — a grep for `sha256|checksum|cosign|signature` across `setup/src/` returned
nothing — while `.github/workflows/release.yml:646-659` already published `SHA256SUMS` and signed it, and
`:705` refused to promote the draft without both. The material to verify existed and was unused.

L5. Independent verification of the setup change, and the four defects it found: the README dropped the
`plugins` input row, it claimed the action never extracts an unverified archive while documenting the
opt-out, the wiring was untested (deleting the guard left the suite green), and the pure layer missed the
hostile-input cases. All four closed; the wiring is now pinned by mutation — removing the verification call
fails five tests, moving extraction before it fails the same five. Reproduced by the parent.

L6. The pin's own test earned its place immediately: the first sha written by hand was one character short,
and `the_install_smoke_pins_its_action_by_commit` failed on it before the workflow could pin a commit that
does not exist. Reverting the pin to `@v1` fails the same test.

L7. Residual, declared rather than implied: the Cosign signature on `SHA256SUMS` is not verified, so an actor
who can replace both release assets defeats the digest check; a tool-cache hit is trusted and verified by
nothing; and no test downloads a real release, so the published asset shape is asserted by construction, not
by execution.
