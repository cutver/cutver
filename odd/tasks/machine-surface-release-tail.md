# Feature: machine surface for the release tail (`publish`, `attest`, `init`)

Issue: cutver/cutver#404 — `feat(cli): publish, attest and init have no machine surface, so a wrapper cannot read the release tail`.
Outcome: a subprocess-driven program reads the outcome of all three commands in one shape, from one stream, with a failure class in the envelope.
Blocks: ADR-0006 rule 4; the B1 observability product.

## Specs

S1. `cutver publish --json` prints one object on stdout whose `schemaVersion` is the first key, covering what the plan tree shows a human: the tag, the commit, the refs pushed, whether the floating major tag moved, each `[publish].commands` result with its exit code, and the `on_post_release` dispatch outcome.

S2. A `publish` failure prints one compact object on **stderr** and adds nothing to stdout, with an envelope `code`.

S3. `cutver attest --json` prints the record it just wrote — `gate`, `commit`, `conclusion`, `verified_at`, optional `run`, and the path written — in the same first-key shape.

S4. `cutver init --json` prints the manifests detected, the primary source of truth chosen, and the file written.

S5. `publish --dry-run` uses the same object with the same keys, so an adapter parses one schema instead of two.

S6. Every added object is additive per `CONTRACT.md:232-233`: no existing field changes name, type, meaning or position (field order is part of the contract), and no surface is removed in the release that renames it.

S7. Documented: the `machine-contract.md` table gains three rows, and `CONTRACT.md:229`'s enumeration grows by exactly those three.

S8. Tested where the contract is already tested: a golden per object shape, and one assertion that a refusal still writes nothing to stdout.

S9. Failure classes are not in the exit code: the class lives in the envelope's `code` (thirteen values, `machine-contract.md:88-103`) and `exitCode` always equals the process code — for these three commands too.

S10. **Out of scope by decision (L4).** The attestation file's missing `schemaVersion` is `contract-debt`, split into its own issue: this feature reads and writes that file exactly as today. `CONTRACT.md:229`'s "the attestation files" clause is left as the other issue's subject, not silently reworded here.

## Tasks

T1. Arg surface `json: bool` on `Publish`, `Attest`, `Init`, and the three `wants_machine` arms. — S1, S3, S4. — **folded into T2/T5/T6** — see L6.
T2. Publish success object on stdout, plan tree preserved on the human path. — S1, S6. — writer — c1.
T3. Publish `--dry-run` emits the same object with the same keys. — S5. — writer — c1.
T4. Publish failure: envelope on stderr, stdout untouched. — S2, S9. — writer — c1.
T5. Attest object: the record written plus the path. — S3, S9. — writer — c2.
T6. Init object: manifests detected, primary source, file written. — S4, S9. — writer — c2.
T7. Golden per object shape + refusal-writes-nothing assertion, in `tests/schema_compat.rs` and `tests/machine_error_contract.rs`. — S8. — parent inline — c3.
T8. Docs: `machine-contract.md` rows and the human-only paragraph, `CONTRACT.md:229`, `docs/reference/cli.md`, `machine-contract-surfaces.md`. — S7. — parent inline — c4.
T9. Open the contract-debt issue for the attestation file's missing `schemaVersion`. — S10. — parent — after c4.

## Log

L1 (user, verbatim): «seguimos con 404»
L2 (issue #404, verbatim acceptance section): "What you would accept as done" — the six bullets transcribed as S1-S8, plus item 4 ("Failure classes are not in the exit code, for every command") as S9.
L3 (user, verbatim): «Un solo PR con commits por unidad» — delivery shape: one PR, one commit per work unit, on a feature branch.
L4 (user, verbatim): «Qué me recomiendas» — the attestation-file `schemaVersion` decision is delegated to the architect. Decision: split into its own contract-debt issue. Reason: it changes the on-disk format of the file whose reader is deliberately total (`src/config/gates/store.rs:3-4`, "a file that does not parse is 'no attestation'`), so a shape mismatch silently disables a gate — that is a stored-format change (high-risk item 1), and it must not ride inside a PR that is already three commands plus docs plus goldens. Consequence: S10 is out of scope; T9 opens the issue.
L5 (explorer evidence, mwxnpsay-1-1mc8): `src/cli/args/commands.rs` is 380 lines under a 400-line ceiling with no allowlist entry (`tests/source_line_budget.rs:26`); `src/cli/machine.rs` is also unallowlisted; `wants_machine` returns `false` for `Init`/`Publish`/`Attest` at `machine.rs:53,55-56`; per-command exit codes for `[publish].commands` do not survive today (`src/bump/exec/command.rs:65-72` keeps status only on failure); `init`'s `created_files` is local and dropped (`src/init/scaffold/runner.rs:156-176`); `attest` already has everything it needs in `Recorded` (`src/config/gates/store.rs:40-47`); no test pins `--json`-on-publish as a usage error, so the documented behaviour inverts in prose only.

L6 (architect decision, recorded here): the flag and its object land in the same commit, per command, so no commit ships a `--json` that prints nothing — that is the decorative-declaration defect #300 already named. Consequence: T1 is not its own commit; c1 is publish (T2-T4), c2 is attest plus init (T5-T6), c3 pins the shapes (T7), c4 documents them (T8). Publishing and the other two cannot be built in parallel: both touch `src/cli/args/commands.rs` and `src/cli/machine.rs`.
L7 (architect decision): doc line ceilings bind this work — `docs/reference/cli.md` is 199/200 and `docs/internals/machine-contract-surfaces.md` is 199/200 under `every_document_stays_under_the_line_ceiling` (`tests/ci_workflow_contract.rs:301`, `CONTRACT.md:140`), while `docs/internals/machine-contract.md` is 112/200. So the three contract rows go where the room is, and any cli.md sentence this change adds is paid for.
L8 (evidence, c1): `b2311b2` "feat(publish): answer on a machine surface with --json", 13 files, +782/−63 (478 of those insertions are `tests/publish_json.rs`, 50 the feature doc). Verified: `cargo test --locked --workspace` with `GITHUB_ACTIONS=true` and both sinks writable — the whole suite green except `cli_reference_parity`, which named the missing `cutver publish --json` documentation; that row landed in this commit and the four documentation suites then went green. `cargo test --locked --lib` 557 passed. clippy `-D warnings` clean. `docs/reference/cli.md` is now at 200/200, exactly at the ceiling, which is why the two remaining rows need room made for them in c2.
