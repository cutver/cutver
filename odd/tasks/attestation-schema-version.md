# Attestation file carries `schemaVersion`

GitHub issue **#409** (`type:feat`). One struct, one writer, one reader, one test, two documents.
Risk: item 1 — this changes a *stored* format that `publish --require-gate` reads through a deliberately total parse.

## Specs

**S1.** The file written by `cutver attest` opens with `schemaVersion`, first key, integer — the same rule and the same value the other surfaces carry (`src/cli/json.rs:15`).

**S2.** A test that every file in `[gates] dir` opens with `schemaVersion`, so a future writer cannot add a shape without one.

**S3.** An explicit statement of what happens to files written before this change: absent key means the legacy shape and stays readable, or it is refused with a message naming what to re-run. Either answer is fine; leaving it implicit is not.

**S4.** `attest --json` (shipped in #408) reports the same record, so the object and the file cannot disagree about the version.

**S5.** `CONTRACT.md:229` and `docs/internals/machine-contract-release-tail.md` describe the file's version once it exists.

### Decisions taken while implementing

**D1 (legacy policy, answers S3).** *Absent key means the legacy shape, and it stays readable.* Reason: `Attestation` has no `deny_unknown_fields` and the reader is total on purpose (`src/config/gates/store.rs:140-143`), so a file written before this change — and every `{"gate":"ci"}` fixture in `tests/` — keeps reading as an attestation. Refusing them would convert a recorded verdict into `GateMissing` (`src/bump/exec/guard.rs:80`), which is the "gate disables itself silently" failure the issue calls the worst this file can have.

**D2 (mechanism).** The writer wraps the record in `crate::cli::json::Versioned` instead of adding a `schemaVersion` field to `Attestation`. Reason: `docs/internals/machine-contract.md:8` states the rule lives in one place (`src/cli/json.rs`), and `SCHEMA_VERSION` is that place. A hand-rolled field would be a second copy of the number, free to drift. Precedent for a non-CLI module using it: `src/changelog/context/release.rs:60`.

## Tasks

- **T1** — `store::write` wraps in `Versioned`; the store's round-trip test pins the new first key; a new case proves a legacy file still reads. Covers S1, S3, S4. — done, in `fbb45b4`.
- **T2** — integration test: `cutver attest` writes into a custom `[gates] dir`, and *every* file there opens with `{"schemaVersion":1,`; the file's version equals the `schemaVersion` `attest --json` printed. Covers S2, S4. — done, in `fbb45b4`. Covers `.json` files and the one writer that exists; a new command writing elsewhere in that directory, a non-`.json` file or a subdirectory would not be examined, and the page says so.
- **T3** — `CONTRACT.md` and `docs/internals/machine-contract-release-tail.md` state the file's version and the legacy policy; `src/config/gates.rs`'s example object and `store::read`'s doc comment say the same. Covers S5. — done, in `fbb45b4`.
- **T4** — independent verification (Risk item 1) and the work-unit commit. — done: `gentle-ai-verify` returned pass on S1-S5, refuted two doc claims (the test's coverage scope, and "the plan file follows the same rule" — `cutver plan` writes no file, the other stored object is the file `bump --report` writes) and named a real gap: no end-to-end test proved a complete legacy record still satisfies the gate. All three are corrected in `fbb45b4`, which adds `a_legacy_attestation_without_the_version_key_still_satisfies_the_gate`.

## Log

**L1** (user, verbatim): «seguimos con el 409»

**L2** (issue #409, verbatim acceptance criteria): «The file written by `cutver attest` opens with `schemaVersion`, first key, integer — the same rule and the same value the other surfaces carry» · «A test that every file in `[gates] dir` opens with `schemaVersion`, so a future writer cannot add a shape without one» · «An explicit statement of what happens to files written before this change: absent key means the legacy shape and stays readable, or it is refused with a message naming what to re-run» · «`attest --json` (shipped in #408) reports the same record, so the object and the file cannot disagree about the version» · «`CONTRACT.md:229` and `docs/internals/machine-contract-release-tail.md` describe the file's version once it exists.»

**L3** (evidence, 2026-11-26). Real bytes from `target/debug/cutver.exe` in a scratch workspace: `{"schemaVersion":1,"gate":"ci","commit":"0123…","conclusion":"success","verified_at":"2026-11-27"}` — the key first, the record's keys in their old order, `run` appended last when `--run` names one. RED observed before the writer changed, on `7e9ace6` with the new tests in place: `src/config/gates/tests.rs:39` and `tests/attest_init_json.rs:269` both failed showing the old first bytes. `sh scripts/check.sh` → `gate: PASS (9 passed, 0 failed, 1 not run here)`; the not-run step is `clippy (x86_64-unknown-linux-gnu, plugins feature)`, which needs `x86_64-linux-gnu-gcc`, absent on this Windows host.

**L4** (delivery). Branch `feat/409-attestation-schema-version`, one work-unit commit `fbb45b4` (8 files, 209 authored lines), pushed and opened as **PR #412** (`type:feat`). Merge is the maintainer's decision and has not happened.
