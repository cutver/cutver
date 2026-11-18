# CI-native contract — phase 2 of #326 (`cutver plan`)

Feature branch: `feat/326-plan`. Requirement source: issue #326, "Sequencing" phase 2 and acceptance criterion 3.

Phase 1 (schema version + machine errors) shipped as `ce22249` and `5219ea7`; its contract (`src/cli/json.rs`,
`src/cli/machine.rs`) is what this phase builds on.

## Specs

S1. `cutver plan` exists and is read-only: it computes the release the workspace is about to cut and mutates nothing —
no file write, no git mutation (stage/commit/tag/push), no preflight execution, no `[hooks]` execution, no
`[publish] commands`, and **no plugin dispatch** (not even `on_pre_bump`, which `bump --dry-run` does dispatch today).

S2. `cutver plan --json` emits the plan as one versioned object (`src/cli/json.rs`), with `schemaVersion` first:

```json
{
  "schemaVersion": 1,
  "read_only": true,
  "level": "minor",
  "current_version": "0.19.0",
  "next_version": "0.20.0",
  "tag": "v0.20.0",
  "floating_tag": "v0",
  "commit": "<HEAD sha the release would be cut from>",
  "branch": "main",
  "commit_message": "chore(release): v0.20.0",
  "notes": "## v0.20.0 — 2026-11-18\n\n### ✨ Features\n…",
  "changelog": { "path": "CHANGELOG.md", "prepends": "0.20.0" },
  "manifests": [{ "path": "Cargo.toml", "from": "0.19.0", "to": "0.20.0" }],
  "preflight": [{ "name": "test", "command": "cargo test", "skipped": false }],
  "phases": [{ "name": "validate", "mutates": false, "steps": ["…"] }],
  "artifacts": [],
  "gates": [],
  "blockers": []
}
```

S3. `notes` is the **rendered** entry the release would prepend to the changelog — the real body, produced without
writing the file. A plan cannot claim notes it did not render: if rendering is impossible the command fails closed
(exit 1) instead of emitting `notes: null`.

S4. `plan` answers "may I tag?" from evidence, not from YAML ordering: the guards that would abort a real `bump`
(clean tree, required branch, existing release tag locally or on the remote, tag-name validity) run read-only and
are reported as `blockers` — a list of one-line reasons — instead of aborting. A plan with blockers is still a plan:
it exits `0` and hands the decision to the consumer. Exit `1` is reserved for "the plan could not be computed".

S5. `plan` works on a dirty tree and outside the required branch, because that is the state a CI job plans in. The
guarantee S1 states is what makes that safe, and it is proven by a test that plans a dirty tree with a
`[publish] commands` section configured and compares every file's bytes before and after.

S6. `phases`, `artifacts` and `gates` are part of the document from day one:

- `phases` describes what the release would do, in order, each with `name`, `mutates` and its `steps`. It is derived
  from the same computation the pipeline uses — one authority, not a second description of the pipeline.
- `artifacts` and `gates` are emitted as empty arrays with the shape a consumer can code against. Nothing in the
  configuration declares artifacts or attests a gate yet; that is phase 3 (`artifacts`, `publish --require-gate`).

S7. Documentation ships in the same commit: `docs/reference/cli.md` gains `cutver plan`, and
`docs/internals/machine-contract.md` gains the plan surface (and stays under the 200-line ceiling). `cutver plan`
without `--json` prints the human plan tree, reusing the renderer the dry run already has rather than a second one.

S8. The existing surfaces do not change: `bump`, `bump --dry-run`, `bump --json`, `bump --report` keep their output,
their keys and their order. A plan is a new command, not a new mode of `bump`.

## Tasks

- [x] T1 — S1, S3: the read-only computation. Extract the release plan from `run_staged` into one authority, render the
  changelog entry without writing it, and keep the plugin/hook dispatch out of the read-only path.
- [x] T2 — S2, S6: the plan document (struct, `schemaVersion` first, snake_case keys, `phases` derived from the
  computation, `artifacts`/`gates` present and empty) + `cutver plan --json`.
- [x] T3 — S4, S5: guards as `blockers`, dirty tree and branch tolerance, exit-code rule.
- [x] T4 — S7, S8: the human plan output + docs; prove `bump` and its JSON are byte-identical to `main`.
- [x] T5 — tests: dirty-tree-no-mutation (byte comparison of every file), notes rendered with no write, blockers as
  data, shape of the document, and the S8 no-change pins.

## Log

L1 (user, verbatim): `mergea y vamos con la siguiente fase` — phase 2 of #326 after phase 1 merged as `5219ea7`.

L2 (requirement, issue #326 "Sequencing" phase 2, verbatim): `**Plan** — `cutver plan --json`, always a dry run, no
mutation (gap 1). This is the keystone: it converts `needs:` into data.`

L3 (requirement, issue #326 acceptance criterion 3, verbatim): `- [ ] `cutver plan --json` emits phases, artifacts,
gates, tag, commit and notes, mutates nothing, and is covered by a test proving that a dry run over a dirty tree
changes no file.`

L4 (exploration, gentle-ai-explore, `path:line`): the plan exists only as a `Summary` (`src/bump/types.rs:226-251`)
rendered as text (`src/cli/tree.rs:221-241`); there is no plan-only entry point — `run_staged`
(`src/bump/exec/pipeline.rs:56`) is one pipeline whose dry-run branches skip writes. The plan is complete at
`pipeline.rs:120-127`. `bump --dry-run` still dispatches `on_pre_bump` (`:98`), `on_post_bump` (`:138`) and
`on_post_release` (`src/bump/exec/publish.rs:408-441`) with `dry_run` in the payload, so a plugin may still act — which
is why S1 rules plugin dispatch out of `plan`. `release_notes` is `None` on a dry run because `prepare_changelog` sets
`updated = None` (`src/bump/exec/changelog.rs:224-247`), so S3 needs a render-without-write path. Guards:
`require_clean_tree` default `true` (`src/config/types.rs:308`), `require_branch` default `None` (`:309`), collisions at
`pipeline.rs:92`. Fallback if `notes` cannot be rendered read-only: emit the plan without `notes` and say so — never a
half-truth.

L5 (decision, parent): `plan` is plugin-free and hook-free by design. The alternative — reusing `bump --dry-run` and
calling it read-only — would keep a mutable third-party code path inside a surface whose only promise is that it
changes nothing, and the acceptance criterion's byte comparison could not honestly cover it.

L6 (decision, parent): blockers are data and a blocked plan still exits `0`. The release decision belongs to the
consumer (a gate, a human, a workflow); the command's job is to hand over evidence, and reserving exit `1` for "no plan
could be computed" keeps that single meaning.

L7 (decision, parent): `artifacts` and `gates` ship empty rather than absent. The criterion names them, phase 3 fills
them, and a consumer that codes against the shape today does not have to branch on their presence later.

## Implementation brief

Read this section before writing code; it is the parent's handoff, not a suggestion.

1. **One authority (T1).** Extract the phase-1 computation currently inline in `run_staged`
   (`src/bump/exec/pipeline.rs:65-127`: guards, current version, next version, tag + name validation, commit message,
   collision check, preflight plan, `assemble_scope`, `compute`, changelog planning) into `src/bump/plan.rs` as a
   function returning an owned `ReleasePlan`, and make `run_staged` call it, so both paths share the math. The plan path
   must not dispatch a plugin, must not execute `[hooks]` or preflight, and must not open a `MutationTransaction`.
2. **Notes rendered, never written (S3).** `prepare_changelog` sets `updated = None` under `dry_run`
   (`src/bump/exec/changelog.rs:224-247`), so a dry run cannot show the notes. Add a render-without-write result the plan
   carries as `notes`. Do not write the file, and do not change what `bump`/`bump --dry-run` report (S8).
3. **The document (S2, S6).** `src/bump/plan.rs` owns `ReleasePlan` + its `Serialize`, with exactly the keys and order
   of S2 (snake_case, `schemaVersion` first through `Versioned`). `commit` is the HEAD sha read read-only; `branch` comes
   from `current_branch()`. `phases` is built from what the plan knows (validate → mutate → publish, with `mutates` and
   the steps), never a hand-written parallel description of the pipeline.
4. **Guards as blockers (S4).** Add a read-only evaluation that reports instead of aborting: dirty tree when
   `[git] require_clean_tree` (default true), a required branch that does not match, an existing release tag locally or
   on the remote, and tag-name validity. One self-contained sentence per blocker, naming what to fix. A blocked plan
   still exits `0`; exit `1` is only "the plan could not be computed" and is reported through
   `print_error(FailureCode::…, …)` — add a `Plan` variant to `FailureCode` if that is cleaner.
5. **Never abort on a dirty tree or the wrong branch.** That is the CI state a plan is asked about.
6. **Human output (S7).** Reuse the tree renderer in `src/cli/tree.rs` (extending it is fine) instead of a second one,
   and print the blockers when there are any.
7. **Docs, same commit (S7).** `docs/reference/cli.md` gains `cutver plan` and must stay at or under 200 lines (it is at
   194: tighten prose elsewhere in the same file if needed, without deleting a true claim).
   `docs/internals/machine-contract.md` gains the plan surface and stays under 200 lines (it is at 155), stating that
   `artifacts`/`gates` are present and empty until phase 3 and that a blocked plan exits `0`.
8. **S8: prove nothing else moved.** `bump`, `bump --dry-run`, `bump --json`, `bump --report` byte-identical to `main`.
   The pins are `src/cli/runner/tests.rs:825` (exact JSON), `src/cli/tree.rs:295` (exact tree lines),
   `tests/e2e_bump.rs:83`, `tests/e2e_lifecycle.rs`.

### Tests (T5)

`tests/e2e_plan.rs`, not `#![cfg(unix)]` unless it truly needs it:

- **the acceptance criterion**: a dirty tree with a `[publish] commands` entry and a `[hooks] pre_bump` whose command
  would write a file; snapshot every file's bytes, run `plan --json`, snapshot again, assert every byte identical, no new
  file, no tag, no commit, unchanged `git status`.
- `plan --json` is one versioned object, `schemaVersion` first, whose `notes` is a non-empty rendered entry that does not
  appear in `CHANGELOG.md` afterwards.
- blockers as data: a dirty tree and a mismatched required branch both yield `blockers` entries and exit `0`.
- a blocker-free plan has `blockers: []` and the same `next_version` the equivalent `bump --dry-run --json` reports.

Unit tests in `src/bump/plan.rs` for the document shape/order and the phase list.

### Checks

`cargo test --workspace --features plugins`, `cargo clippy --all-targets --features plugins -- -D warnings`,
`cargo fmt --all` and `--check`, plus a manual run in this repository: `cutver plan --json`, `cutver plan`, and
`cutver bump --dry-run --json` (9 keys, `schemaVersion` first, unchanged).

L8 (decision, parent, after the worker hit it): `src/bump/exec` declares its submodules private, so a plan computed at
`src/bump/plan.rs` could not name `assemble_scope`, `compute`, `prepare_changelog`, `build_context` or `format_command`
(`error[E0603]`). Approved a `pub(crate) use` block in `src/bump/exec.rs` — visibility only, no logic — so the
extraction stays in `src/bump/` instead of moving under `exec/`.

L9 (decision, parent): `CONTRACT.md` caps a source file at 400 lines, and the first cut of `src/bump/plan.rs` was 994.
Split as a facade with no behaviour change: `plan.rs` 387, `plan/finish.rs` 198, `plan/guards.rs` 62,
`plan/resolve.rs` 118, `plan/tests.rs` 273. `impl ReleasePlan::summary` (the tree-renderer view) lives in `finish.rs`,
the one placement the split forced beyond the approved list.

L10 (decisions the implementation had to settle, recorded so they are not re-litigated):
- `level` reports the **resolved** level (`auto` → `major`, `--first-release` → `initial`), not the literal flag value:
  a consumer asking "what would this release be" needs the resolution.
- A plugin-driven surface (`[version] strategy = "plugin"`, `[changelog] format = "plugin"`, `kind = "plugin"` manifests)
  makes `plan` exit `1` with `PlanNeedsPlugin` instead of dispatching: S1/L5 forbid third-party code inside a command
  whose only promise is that it changes nothing. `bump --dry-run` stays the rehearsal path for those projects.
- A project with no `[changelog] path` fails closed (`PlanNotes`) rather than reporting `notes: null`: S3 says a plan
  never claims notes it did not render.

L11 (evidence, T5): the acceptance criterion proven by `tests/e2e_plan.rs::a_plan_over_a_dirty_tree_changes_no_file` —
a dirty tree with a `[hooks] pre_bump`, a `[preflight]` step and `[publish] commands` all armed to drop marker files;
`sha256` of every file identical before and after, zero markers, no tag, no commit, `git status` unchanged, and the
rendered `notes` never appearing in `CHANGELOG.md`. Plus a differential against `5219ea7` (bump real/dry-run/`--json`,
`doctor --json`, `publish --dry-run`): byte-identical after masking the short shas git prints from its own timestamps.
