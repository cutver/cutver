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
commit being published — a required gate being everything `[gates] require` declares plus everything this invocation
names — and the refusal names **the gate and the commit**; a `failure` conclusion refuses too, with
the same voice. Verification happens at the same point as S2, before anything irreversible.

S10. The attestation directory is exempt from `publish`'s clean-tree guard exactly like the declared notes file
(`src/bump/exec/publish.rs:136`), because the attestation is an input to the command, not a change in the tree.

S11. `cutver plan --json` fills `gates[]` with the gates `[gates] require` declares and, when an attestation exists for
the commit it reports, its conclusion — so a CI job can see the gate state before asking for a tag.

S12. Attestations are data, not trust: the tool records and reads them, and the docs state plainly that a forged
attestation is only as trustworthy as the machine that wrote it. No signing, no network.

## Tasks

- [x] T1 — S1: `[[artifact]]` config table, validation, unknown-key rejection, typed model with defaults.
- [x] T2 — S3: one glob authority outside the `plugins` feature, used by publish, plan and the plugin manager.
- [x] T3 — S2: verification in `publish` before the first irreversible step, with the fail-closed error.
- [x] T4 — S4: `artifacts[]` in `cutver plan --json`.
- [x] T5 — S5, S7: no-change proof for undeclared projects + docs.
- [x] T6 — S8-S12 (3B): `cutver attest`, `--require-gate`, the clean-tree exemption, `gates[]` in plan, docs.
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

L11 (evidence, run `44444105817` on `f290f37`): the fold works and the starvation is gone. All seven jobs green in one
run, 6m wall clock, no leg cancelled: the folded entry `Test (macos-latest, default and plugins)` ran `Build`, `Test`,
`Build (plugins feature)` and `Test (plugins feature)` in 345s on a single runner, with `Clippy` correctly skipped
(ubuntu-only). PR #330 reads `mergeStateStatus: CLEAN`. A prior run had measured the two macOS legs at 148s (plugins,
served) and 902s (lean, starved) — the fold costs serialization inside one OS and buys acquisition.

L12 (decision, user): do **not** extend the fold to ubuntu or windows. Measured cost in the same run: folding ubuntu
would serialize 58s + 90s, folding windows 170s + 220s, making windows the new critical path (~6.5-7m per run vs ~6m),
and folding all three would leave `test-plugins` with nothing to run, forcing edits to `gate.needs`, its env wiring and
`LEGS` in `tests/ci_workflow_contract.rs`. Neither pool showed contention: all four ubuntu/windows legs were acquired in
every inspected run. The fold answers a measured scarcity, not a job count.

L13 (decision, parent — three resolutions the implementation forced, recorded so the spec and the code cannot drift
apart):

- **`[gates] require` is a standing requirement, enforced by `publish`.** S11 named the array's contents but not their
  source, and `plan` takes no `--require-gate`; the alternatives were worse — a `plan` that invented gates would be a
  second authority, and reading them from the consumer's workflow is the YAML-owns-the-pipeline drift #326 exists to
  end. The first implementation made `[gates] require` report-only and left `--require-gate` as the sole enforcement;
  the independent verifier flagged that as a decorative declaration, the same defect class as #300
  (`permissions.env` parsed and never enforced). Superseded: `publish` requires the union of `[gates] require` and
  `--require-gate`, deduplicated, so a project declares its gates once and one invocation can add to them.
- **`verified_at` is the UTC day (`YYYY-MM-DD`)**, from `changelog::format_date`, the repository's single clock
  authority. A full timestamp needs a second date formatter in `src/changelog/update.rs`, outside this slice. A gate is
  a per-release fact and the commit is recorded next to it, so the day is enough to decide; widening it is a follow-up.
- **A damaged or foreign attestation is "no attestation", never an error.** A truncated document, or one naming another
  gate, cannot vouch for anything, and refusing it must not crash a release. The refusal names the path and says it
  "does not exist or does not read as an attestation", so the operator learns the file is there and unreadable rather
  than absent — the condition is not swallowed, it is named.

L14 (review, three rounds — each one found something the code review of the diff alone would not have):

- **R1/D1 (blocker, round 2).** The first implementation exempted a *collapsed* untracked entry only. A tracked `.gitkeep`
  in `.cutver/attestations` makes git stop collapsing and list the attestation individually, so `publish` aborted on
  `?? .cutver/attestations/ci.json` — the exemption failed in the realistic shape. Fixed in `src/git/status.rs` by
  `is_subtree` plus a prefix match, so a declared directory covers its contents whichever way git spells them.
- **D5 (round 2).** `[gates] require` was parsed and only reported; `publish` enforced `--require-gate` alone. That is the
  #300 class exactly (a declaration nothing enforces). Superseded in L13: the union is required, deduplicated.
- **R1 (round 3, change-caused).** Widening the exemption to directories silently changed a *pre-existing* command: a
  `--notes-file` naming a directory passed the guard and died later with `Access is denied`. It reaches no irreversible
  step, but an unrequested change to an existing command's failure is a regression; the request now rejects it up front,
  before the guard, naming the flag and the fix.
- **R2 (round 3).** Symlinks were refused in the collapsed shape and exempt in the listed one. No escape was possible,
  but two shapes answering differently to the same question is a defect; both now fail closed through `symlink_metadata`,
  which never follows the link.

L15 (delivery, parent): T6 is one commit and one PR — `456b4e2`, 31 files, 2545 authored lines (~700 production,
~1470 tests, ~200 docs), past the ~400-line slice advisory in `CONTRACT.md:111`. The slice boundary is the whole gate
channel and it does not cut at file level: `tests/e2e_gates.rs` mixes the S8 and S9 cases and `tests/e2e_gates_channel.rs`
mixes S10 and S11, so a file-level split leaves an intermediate commit whose suite is red. The honest re-slice, if review
load demands it, is two PRs with those test files split along S8/S10 versus S9/S11. Recorded, not hidden: L5 drew the same
conclusion for 3A, and the next phase slices up front instead of after.
