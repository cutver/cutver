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

- **T1** — `store::write` wraps in `Versioned`; the store's round-trip test pins the new first key; a new unit test proves a legacy file still reads. Covers S1, S3, S4.
- **T2** — integration test: `cutver attest` writes into a custom `[gates] dir`, and *every* file there opens with `{"schemaVersion":1,`; the file's version equals the `schemaVersion` `attest --json` printed. Covers S2, S4.
- **T3** — `CONTRACT.md` and `docs/internals/machine-contract-release-tail.md` state the file's version and the legacy policy. Covers S5.
- **T4** — independent verification (Risk item 1) and the work-unit commit.

## Log

**L1** (user, verbatim): «seguimos con el 409»

**L2** (issue #409, verbatim acceptance criteria): «The file written by `cutver attest` opens with `schemaVersion`, first key, integer — the same rule and the same value the other surfaces carry» · «A test that every file in `[gates] dir` opens with `schemaVersion`, so a future writer cannot add a shape without one» · «An explicit statement of what happens to files written before this change: absent key means the legacy shape and stays readable, or it is refused with a message naming what to re-run» · «`attest --json` (shipped in #408) reports the same record, so the object and the file cannot disagree about the version» · «`CONTRACT.md:229` and `docs/internals/machine-contract-release-tail.md` describe the file's version once it exists.»
