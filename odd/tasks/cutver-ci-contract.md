# CI-native contract — phase 1 of #326 (schema version + machine errors)

Feature branch: `feat/326-json-schema-version` (slice A), then `feat/326-error-envelope` (slice B, from a fresh `origin/main`).
Requirement source: issue #326, "Sequencing" phase 1 — *Contract first* — and acceptance criteria 1 and 2.

## Specs

S1. Every surface that emits JSON carries `schemaVersion` as its **first** key, with the value `1`: `bump --json`,
`bump --report <PATH>`, `changelog latest --json`, `changelog show --json`, `plugin list --json`, `plugin info --json`,
`plugin search --json`, and the new `doctor --json`. Existing keys keep their current names, order and values.

S2. A consumer can detect an incompatible schema instead of misreading one: the key is always present, its value is an
integer, and the documented rule is that version `1` only grows additively — a new key is not a new `schemaVersion`.

S3. `plugin search --json` switches from a bare array to one object, exactly as the human chose in L3:

```json
{"schemaVersion": 1, "plugins": [ ... ]}
```

The change is declared breaking through the repository's own mechanism — a `feat(cli)!:` subject with a
`BREAKING CHANGE:` footer, which `cutver bump` renders into the changelog — and documented with the new shape
and the `.plugins[]` migration in `docs/reference/cli.md` and `docs/internals/machine-contract.md`. `CHANGELOG.md`
is generated from commits since v0.19.0, so editing it by hand would be drift the doctor check reports.

S4. `doctor --json` reports drift as data on stdout and keeps the exit codes meaning what they mean today (L5):
`0` no drift, `2` drift found, `1` a check failed to run. The rendered text stays on stderr for the human path, so a
machine reads stdout JSON instead of parsing prose.

S5. When a command was invoked in machine mode (`--json`, or `--report` for `bump`), a failure is written to **stderr** as
exactly one JSON object and nothing else:

```json
{"schemaVersion": 1, "error": {"code": "<stable machine string>", "exitCode": 1, "message": "..."}}
```

`code` is a stable, documented string per failure class, `exitCode` always equals the process exit code, and `message`
is the full `Error: …` text the human path would print (including its `Where:`/`Fix:` lines when the error carries
them). The process exit code itself is unchanged.

S6. Without machine mode the behaviour is byte-for-byte what it is today: `Error: …` on stderr, the same exit codes
`0` success / `1` general error / `2` drift, and the same tree, dashboard and notes output.

S7. Documentation ships in the same commit: `docs/reference/cli.md` gains the exit-code table, the list of JSON surfaces
with their `schemaVersion`, the compatibility rule from S2, and the machine-error rule from S5; `CHANGELOG.md` carries
the S3 breaking change.

S8. A JSON consumer must not be able to distinguish an empty result from an unavailable one on these surfaces: `doctor
--json` always carries a `drift` array and a `checks` entry per check, and a check that could not run fails closed with
`code` naming the check instead of printing an empty array.

## Tasks

- [x] T1 — S1, S2: shared schema-version authority (`src/cli/json.rs`) + `schemaVersion` first in `bump --json`/`--report`. Commit: cf6e646.
- [x] T2 — S1: `schemaVersion` first in the `ReleaseContext` JSON of `changelog latest|show --json`. Commit: cf6e646.
- [x] T3 — S1, S3: `schemaVersion` first in `plugin list|info`, and `plugin search` wrapped in `{schemaVersion, plugins}`. Commit: cf6e646.
- [x] T4 — S1, S4, S8: `doctor --json` emits drift and checks as data, exit codes preserved. Commit: cf6e646.
- [x] T5 — S2, S3, S7: docs (`docs/reference/cli.md`, new `docs/internals/machine-contract.md`, index row in `docs/internals/references.md`) and the breaking declaration in the commit footer. Commit: cf6e646.
- [ ] T6 — S5, S6: machine-mode error envelope + stable `code`/`exitCode`, human path untouched. Commit: slice B.
- [ ] T7 — S5: every failure site wired to the envelope (config load, bump, changelog, doctor, plugin, init). Commit: slice B.
- [ ] T8 — S5, S6, S7: tests for both paths and docs for the envelope. Commit: slice B.

## Log

L1 (user, verbatim): `dale al 326`

L2 (requirement, issue #326 "Sequencing", phase 1, verbatim): `**Contract first** — `schemaVersion` on every JSON
surface, a structured error envelope keeping the stable `0`/`1`/`2` exit codes (gaps 4, 5). Small, read-only, and it
unblocks everything.`

L3 (requirement, issue #326 acceptance criteria 1-2, verbatim): `- [ ] Every JSON surface carries a schema version, and a
consumer can detect an incompatible one instead of misreading it.` / `- [ ] Errors carry a structured envelope with the
stable exit codes preserved; `doctor` reports drift as data rather than as rendered text.`

L4 (exploration, gentle-ai-explore, `path:line` evidence): no shared `--json` plumbing exists — each surface serializes on
its own (`render_summary_json` `src/cli/runner/bump.rs:93`, `print_release_context_json` `src/cli/changelog.rs:28`,
`print_json` `src/cli/runner/plugin.rs:136`, inline `to_string_pretty` in `plugin/{info,search}.rs`). No crate-level
error enum: nine per-module `thiserror` enums whose `Where:`/`Fix:` text is baked into format strings. Exit codes come
from each runner returning `i32` (`src/cli/runner/dispatch.rs:21`), consumed by `src/main.rs:5`. `doctor` returns `2` on
drift (`src/cli/runner/doctor.rs:44`). Shape-pinning tests: `src/cli/runner/tests.rs:826` (byte-exact `bump --json`),
`src/changelog/context/tests.rs:1243` (exact ordered key vector). Consumers: `.github/workflows/release.yml:136-161`
(jq on the report file) and `.github/scripts/release-smoke/run{,-plugins}.sh` (literal greps).

L5 (human decision, repr. of the selected option): `plugin search --json` is wrapped in `{schemaVersion, plugins}` **and**
documented as breaking — `Envolver y documentar como breaking`.

L6 (human, verbatim): `Cuál me recomiendas` — the machine-error activation rule was delegated. Decision: the envelope is
machine mode driven (`--json`/`--report` on that command), not a new global flag: the caller that asked for machine
output on stdout gets machine output on stderr, `plan --json` inherits it, and no second flag has to be remembered in CI.

L7 (decision during slice A): `CHANGELOG.md` is generated by `cutver bump` from conventional commits, so the breaking
change is declared with a `feat(cli)!:` subject plus a `BREAKING CHANGE:` footer — the form the generator renders into
the changelog's breaking section — and the migration is written in the docs. A hand-written `Unreleased` section would
show up as an orphan section in `doctor --check-changelog`, i.e. drift, so it was not added.

L8 (evidence, slice A): RED observed before each change (`bump_json_renders_one_line_with_the_release_shape` failed on
the missing key). Verified by hand on this host, in throwaway workspaces, that all five surfaces open with
`schemaVersion: 1`: `bump --dry-run --json`, the same object in `--report`, `changelog latest --json`, `plugin list
--json`, and `doctor --json` in both outcomes (drift → exit 2 with `version_drift[0].path = "b.json"`; clean with
`--check-changelog` → exit 0 with both checks `ok`). `cargo test --workspace --features plugins` 488 + 26 suites green,
`clippy --all-targets --features plugins -D warnings` and `fmt` clean. Caveat: `tests/e2e_doctor.rs` is `#![cfg(unix)]`
by repository convention, so the two new end-to-end tests only run on the CI unix legs; the shape is pinned locally by
unit tests in `src/cli/doctor.rs`, `search.rs` and `info.rs`.
