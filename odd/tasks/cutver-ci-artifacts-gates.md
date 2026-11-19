# CI-native contract — phase 3 of #326 (declared artifacts, attested gates)

Two slices, each its own PR: **3A artifacts** (`feat/326-artifacts`) and **3B gates** (`feat/326-gates`).
Requirement source: issue #326 "Sequencing" phase 3 (gaps 2 and 3) and acceptance criteria 4 and 5.

Phase 2 shipped as `ed4e7fe`: `cutver plan` already emits `artifacts` and `gates` as empty arrays, and those two
arrays are what this phase fills.

## Slice 3A — declared artifacts (specs)

S1. A new `[[artifact]]` table declares what a release must produce:

```toml
[[artifact]]
glob = "dist/*.tar.gz"
at_least = 8
description = "lean binaries"
```

`glob` is required; `at_least` defaults to `1`; `description` is optional and only used in messages. The table is
registered like every other one — root allow-list, per-key allow-list, unknown-key rejection, a typed field on
`Config` with its `Default` — so a typo fails the same way a typo in `[publish]` does.

S2. `cutver publish` verifies every declared artifact **before the first irreversible step**: after `validate`
(`src/bump/exec/publish.rs:138`) and before `publish_prepared` (`:139`), which is before `create_tag` (`:276`) and
long before the tag push (`:280`). A missing artifact fails closed with exit `1` and an error that names the artifact
description (or its glob), the glob, how many files matched, how many were required, and states that nothing was
tagged, pushed or executed.

S3. Globs are workspace-relative, support `**`, `*` and `?`, never escape the workspace root, and are matched against
the files that exist at verification time. Matching is one authority shared by `publish`, `plan` and the plugin
manager's existing `manifest_match` — the matcher lives outside the `plugins` feature so the lean build ships it too
(`src/plugin/manager/glob.rs` has the translation today, behind the feature).

S4. `cutver plan --json` fills `artifacts[]` with the declaration plus what it observes: `glob`, `at_least`,
`description` (when set) and `matched` (the count now). A plan never blocks on a missing artifact — at plan time the
assets do not exist yet — and says so by reporting the count rather than a verdict.

S5. Projects that declare no artifacts are untouched: no new failure, no new output, no new config requirement
(S7 of phase 2's spec already pins that `bump` and its JSON do not change).

S6. No new dependency. `sha2` is optional and `plugins`-only; checksum verification and the target matrix are **not**
in this slice and are named as such in the docs and in the `[[artifact]]` documentation.

S7. Docs in the same commit: `docs/reference/configuration.md` gains the table, `docs/reference/cli.md` mentions what
`publish` verifies (inside its 200-line ceiling), and `docs/internals/machine-contract.md` fills in the `artifacts`
array it currently documents as empty.

## Slice 3B — attested gates (specs, implemented after 3A merges)

S8. `cutver attest <gate> --commit <sha> --conclusion success|failure [--run <url>]` records an attestation: what was
verified, for which commit, with which conclusion, and when. `[gates] dir` (default `.cutver/attestations`) `[gates]`
set the location; the file is one JSON object per gate.

S9. `cutver publish --require-gate <name>` (repeatable) refuses to run when a required gate has no attestation for the
commit being published, and the refusal names **the gate and the commit**; a `failure` conclusion refuses too, with
the same voice. Verification happens at the same point as S2, before anything irreversible.

S10. The attestation directory is exempt from `publish`'s clean-tree guard exactly like the declared notes file
(`src/bump/exec/publish.rs:136`), because the attestation is an input to the command, not a change in the tree.

S11. `cutver plan --json` fills `gates[]` with the gates the release would require and, when an attestation exists for
the commit it reports, its conclusion — so a CI job can see the gate state before asking for a tag.

S12. Attestations are data, not trust: the tool records and reads them, and the docs state plainly that a forged
attestation is only as trustworthy as the machine that wrote it. No signing, no network.

## Tasks

- [x] T1 — S1: `[[artifact]]` config table, validation, unknown-key rejection, typed model with defaults.
- [x] T2 — S3: one glob authority outside the `plugins` feature, used by publish, plan and the plugin manager.
- [x] T3 — S2: verification in `publish` before the first irreversible step, with the fail-closed error.
- [x] T4 — S4: `artifacts[]` in `cutver plan --json`.
- [x] T5 — S5, S7: no-change proof for undeclared projects + docs.
- [ ] T6 — S8-S12 (3B): `cutver attest`, `--require-gate`, the clean-tree exemption, `gates[]` in plan, docs.
- [x] T7 — runner capacity (delivery, not a spec): fold both macOS configurations into one `test`
      entry, drop macOS from `test-plugins`, add `workflow_dispatch`. Unblocks 3A's `CI gate`
      without touching the gate's `needs` or the three-leg contract.

## Log

L1 (user, verbatim): `mergea y fase 3`

L2 (requirement, issue #326 "Sequencing" phase 3, verbatim): `**Artifacts and gates** — declared artifacts,
`publish --require-gate <name>` reading an attestation (gaps 2, 3).`

L3 (requirement, issue #326 acceptance criteria 4-5, verbatim): `- [ ] `publish` refuses to run when a declared gate
has no attestation for that commit, and the refusal names the gate and the commit.` / `- [ ] Declared artifacts are
verified before the tag exists, and a missing one fails closed before any irreversible step.`

L4 (exploration, `path:line`): `publish` runs clean tree (`src/bump/exec/publish.rs:136`) → `validate` (`:138`,
checks at `:144-172`) → `publish_prepared` (`:139`) → `finalize` (`:197`), whose first write is `create_tag` (`:276`)
and whose point of no return is `push_release_tag` (`:280`); `[publish] commands` (`:282`) and `on_post_release` (`:283`)
come after. Config registration touches `ROOT_KEYS`/key lists/`reject_unknown_keys` (`src/config/validation.rs:136-227`),
`Config` + `Default` (`src/config/types.rs:95-140`), and `discovery.rs:16-21` when parsed out-of-band. No glob crate is a
direct dependency: `src/plugin/manager/glob.rs:10` has the matcher, behind the `plugins` feature, and there is **no**
directory-walking helper anywhere in `src/`. `sha2` is optional and `plugins`-only. The workflow produces assets into
Actions artifacts and only `dist/` inside the `publish` job, i.e. **after** `cutver publish` (`release.yml:639-661`), which
is why S4 reports counts instead of blocking.

L5 (decision, parent): phase 3 ships as two PRs. Together they are ~1500 lines — well past the review advisory — and the
two halves are independent: artifacts is a verification of files that must exist, gates is an attestation channel. Each
lands with its own tests and docs.

L6 (decision, parent): checksums and the target matrix are **not** in 3A. `sha2` is a `plugins`-only optional
dependency and the lean build must stay lean (VI.4); a `[[artifact]]` entry verifies that the declared files exist, not
that their bytes hash to something. The declaration stays additive: `checksum = "SHA256SUMS"` can be a later key
without a new table.

L7 (decision, parent): in 3A `plan` reports `matched` and never blocks. At plan time the assets are not built yet — the
workflow uploads them from Actions artifacts inside the publish job — so a blocker there would be a lie. In 3B the gates
array does carry a verdict, because an attestation either exists for that commit or it does not.

L8 (user, verbatim): `Tenemos que continuar con el 326 desde donde nos quedamos. Recuerda que la PR 330 en CI de MacOS
no pasó las tres veces que lo intentamos porque lo cancelaban, en el 326 dejaste comentario al respecto de todo`

L9 (evidence, `path:line`): the macOS leg was never acquired, and no repository-side YAML can cause that. Across
44332874622 (attempts 1 and 2) and 44339353782, the job payload carries `runner_id: 0`, `runner_name: ""`, `steps: []`
and `started_at == created_at` — queued, never in progress — for exactly 902s, with GitHub's own check annotation
`The job was not acquired by Runner of type hosted even after multiple attempts` plus the notice `Due to capacity
constraints, jobs targeting macOS arm64 runners may experience longer queue times.` The sibling macOS leg of each run
did get a runner (5m04s and 5m06s queue) and passed, so the pool was degraded, not absent. Ruled out as causes:
`fail-fast: false` (`ci.yml:19`, `:72`), `timeout-minutes` (zero matches under `.github/`), and workflow `concurrency`
(`ci.yml` has none; the only one is `container.yml:21-23`, keyed on its own `github.workflow`). `gate` failed only
because `needs.test.result` was `cancelled` (`ci.yml:111-129`), which is the fail-closed behavior its comment demands.

L10 (decision, user): fold the two macOS legs into one job and add `workflow_dispatch`. One macOS runner request per
run instead of two, both configurations preserved. Implemented inside `test` rather than as a fourth job because
`tests/ci_workflow_contract.rs` pins `LEGS = [test, test-plugins, publish-dry-run]` and fails when a job exists that the
gate does not need: a new job would have forced a change to the `needs` list and to the three-leg contract, i.e. to the
one thing guarding the branch ruleset. The macOS entry now runs the lean build (the VI.4 guard) and then the `plugins`
build and test on the same runner; `test-plugins` keeps ubuntu and windows and loses no coverage.
