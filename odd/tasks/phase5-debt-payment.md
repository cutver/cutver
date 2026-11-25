# Phase 5 debts (#326 gaps 6-7) — the follow-up the merged stack left behind

Status: **prepared, not started.** The maintainer deferred execution explicitly and will resume with a
single word. Branch to create at that moment: `chore/phase5-debts` off `main` (`cec818e`). Nothing here
reopens phase 5's decisions D1-D6 or ADR-0005; each task pays one debt that phase recorded, or closes one
gap it declared unproven.

## Specs

- **S1.** «Deja todo preparado para atacarlo después» and «te voy a decir continuemos y debes de saber
  todo sin tener que volver a explorar». This document is the handoff: every task carries its file,
  line, the change it owes, the test that proves it and the trap that file already has. An executor
  starts from here with zero discovery reads.
- **S2.** «cerré demaciadas PR entre ayer y hoy» — phase 5 is merged and closed as the chain #391-#396
  (`12fa98c`, `34417ce`, `fe5dbcd`, `f0a4fc0`, `7bfd241`, `f243e80`). Every item below is a follow-up
  on `main`, never a reopen of the phase's own review. `docs/internals/design.md:135` already reads
  `Pre-release channels (--channel, promotion) | Implemented`; that row is current, not debt.
- **S3.** The two debts phase 5 recorded, verbatim from `odd/tasks/prerelease-channel-and-resume.md:193`:
  «Accepted debt, unchanged: `PublishCommitMismatch` still offers "drop `--commit`", advice a resume
  would refuse, because a resume-specific hint would push `src/bump/types.rs` (399 of 400) over the
  ceiling» — and from `:195`: «and the new `Resume` line builds a second `PluginManager` because
  `PublishSummary` cannot gain a field without touching `src/cli/ci/tests.rs`».
- **S4.** The ratchet this phase leaned on: `tests/source_line_budget.rs:32` still caps
  `src/bump/exec/publish.rs` at **469** while the file is **448** lines after the `publish/validate` +
  `publish/resume` split (`odd/tasks/prerelease-channel-and-resume.md:191`). The guard's own rule at
  `tests/source_line_budget.rs:152`: a file under the ceiling "is no longer debt, so delete its
  ALLOWLIST entry".
- **S5.** ADR-0005's traceability line is stale, `docs/adr/0005-prerelease-channel-supersedes-0004.md:84`:
  «No pull request number is recorded for the implementation: the work sits on `feat/prerelease-channel`,
  and the delivery shape — one pull request with a commit-per-slice reading order, or chained ones — is
  decided when it is published.» It contradicts the same ADR's `:7` ("merged to `main` as the chained
  stack #391-#396") and the plan's `L12` (`odd/tasks/prerelease-channel-and-resume.md:236`), which
  records all six `pr/*` branches deleted.
- **S6.** The gaps phase 5 did **not** close, verbatim from `odd/tasks/prerelease-channel-and-resume.md:23`:
  «Gaps 2, 5, 8 and 9 are not part of it, and the issue records that 2, 5, 8 and 9 were "not re-checked" —
  treat them as unknown, not as done.» They are unknown, so they may only be **re-checked and recorded**;
  no task here implements them (gap 8 and 9 are sequenced to phase 6, `.git/issue-ci-native.md:114`).
- **S7.** The gap-7 residual is unproven, not closed: `.git/issue-ci-native.md:131` still asks for "a test
  for the crate-published / draft-invisible residual"; `docs/internals/pipeline.md:108` still states that
  residual **without** mentioning `publish --resume`; and `tests/release_deferred_publish.rs:1221-1458`
  covers tags, pushes and refusals only.
- **S8.** `CHANGELOG.md` owes nothing. Its latest entry is `v0.21.0` (`CHANGELOG.md:6`, matching
  `Cargo.toml:3`), the phase merged after that release, and `bump` generates the note when the release is
  cut. Writing a changelog entry by hand here would be inventing the tool's own output.
- **S9.** Non-goals, unchanged from phase 5's S5: a CI provider SDK, an HTTP client in the lean build, and
  any surface beyond stdio, exit codes and files. No task here touches a timeout, serialises a test, or
  accepts a red that is not reproducible.

## Tasks

Each task is one work unit and one commit on `chore/phase5-debts`. Expected total is well under 400
authored lines, so the default delivery is **one PR** with a commit-per-debt reading order; chained PRs
only if the diff crosses ~400 lines.

- **T1 — ADR-0005 traceability tells the truth.** (S5) Rewrite the paragraph at
  `docs/adr/0005-prerelease-channel-supersedes-0004.md:84` to record the merged chain #391-#396, the six
  slice shas as ancestors of `main`, and the deleted `pr/*` branches; keep the "delivery shape was decided
  at publication" fact as history instead of as a pending state. Route: inline. No test (prose).
- **T2 — Retire the stale ratchet headroom.** (S4) Lower `("src/bump/exec/publish.rs", 469)` to the file's
  real length in `tests/source_line_budget.rs:32` — or delete the entry if it lands at or under the 400
  ceiling (448 now, so it stays 448 and the entry shrinks, not disappears). Confirm the guard's failure
  message with a deliberately wrong number first (RED), then set the true value. Route: inline.
- **T3 — `PublishCommitMismatch` stops giving advice a resume refuses.** (S3) Recommended approach **A**:
  reword the single static message at `src/bump/types.rs:83` to be true on both paths — keep «check out the
  commit you meant to publish (`git checkout {expected}`) and re-run», and replace the bare «or drop
  `--commit`» with the qualified form (a plain `cutver publish` with no `--commit` publishes the current
  HEAD; `--resume` requires `--commit`, which `ResumeNeedsCommit` states at `src/bump/types.rs:86-90`).
  This is **zero net lines**, so `types.rs` stays at 399 of 400. Approach **B** (a
  `PublishCommitMismatchOnResume` variant) is rejected: it needs ~6 lines the file does not have, so it
  would force a pre-emptive extraction — a structural cost for a wording defect. The only raise site is
  `src/bump/exec/publish/validate.rs:68-77`; the only test that names the variant is the match arm at
  `tests/release_deferred_publish.rs:554`, which matches the variant and its fields, not the sentence —
  verify that before editing. Test: a resume with a foreign `--commit` must print the qualified Fix and
  never the bare `drop --commit`; a plain `publish --commit` with a foreign commit keeps its exit code and
  `Where:` line. RED first. Route: inline.
- **T4 — One `PluginManager` per publish.** (S3) Give `PublishSummary` (`src/bump/exec/publish.rs:121-129`)
  a `post_release_left_alone: Vec<String>` filled where the manager already exists — `run_publish`'s final
  `Ok(PublishSummary { .. })` at `src/bump/exec/publish.rs:196` — and render it from the summary in
  `src/cli/runner/publish.rs:90` instead of loading a second manager inside `left_alone_line`
  (`src/cli/runner/publish.rs:114-136`: comment at `:117-119`, `PluginManager::from_config` at `:121-124`,
  plus the `Err(_)` branch that names a load failure). The recorded blocker is **wrong and must be
  corrected in the plan**: `src/cli/ci/tests.rs` is 215 lines with **no** `source_line_budget.rs`
  ALLOWLIST entry, so it has ~185 lines of headroom; the cost is editing two literals (`:174` and the
  `..real` spread at `:188`), not the ceiling. Leading `_`-less field addition only. Tests: the rendered
  `Resume` text for a workspace with plugins must stay byte-identical (`tests/release_deferred_publish.rs`
  already asserts it), plus a unit that the field is empty for a config declaring no `on_post_release`
  plugin, so the "declares no plugin" branch keeps its current wording. RED first: the new field absent
  fails the unit. Route: inline; escalate to a worker only if the render path turns out to need a
  signature change in `print_publish_summary` (`src/cli/runner/publish.rs:154`).
- **T5 — The gap-7 residual gets its proof and its doc line.** (S7) Add the missing test: a release whose
  `[publish] commands` (the crate publish) succeeded while the draft stayed invisible, resumed with
  `--resume --commit <sha>`, must refuse to repeat that command without a `success` attestation, must say
  which step it refused by its `publish-<n>` position, and must leave the tag/remote evidence unchanged;
  with the attestation present it may repeat only the idempotent steps. Build on the existing fixture
  range `tests/release_deferred_publish.rs:1221-1458` rather than a new harness. Then state the residual
  honestly at `docs/internals/pipeline.md:108`, naming `--resume` as the recovery path and what it refuses.
  Route: inline if one test suffices; one `gentle-ai-worker` only if the fixture needs a new fake command
  driver.
- **T6 — Re-check gaps 2, 5, 8 and 9 and record each as open or closed, with evidence.** (S6) Read-only.
  Sources: the local notes `.git/issue-ci-native.md` (gap 2 `:41-47`, gap 5 `:65-71`, gap 8 `:89-95`,
  gap 9 `:97-103`, sequencing `:114`) — **untracked local scratch**, so if it is gone read the issue
  (`gh issue view 326`) — plus `docs/adr/0005-prerelease-channel-supersedes-0004.md` and
  `docs/internals/design.md`. Deliverable: one line per gap appended to this document's `## Log`, stating
  current state and the evidence that establishes it. The **issue edit** is not done here: the #326 status
  table is corrected by `odd/tasks/release-channel-dogfooding.md` T1, which owns that write and the ask
  that precedes it. No code. Route: inline.
- **T7 — Ratchet pressure: give a full file its room back, or record why it stays full.** (S4) This is
  the third debt the phase recorded in memory, and the only structural one: the four files the phase left
  near their caps are `src/bump/types.rs` **399/400** (hard ceiling), `src/cli/runner/tests.rs` **949/950**
  (allowlisted cap — cannot grow at all), `src/bump/plan.rs` **390/400** and
  `src/bump/exec/publish.rs` **448/469** (T2 shrinks that cap). Nothing in T1-T6 needs a new line in any of
  them once T3 takes approach A, so T7 is **conditional**: execute it only if a later edit lands in one of
  those files. When it is executed, the phase's own recommendation was to split `src/bump/types.rs` into
  `types/errors.rs` + `types/summary.rs` and to extract one test module out of `src/cli/runner/tests.rs`;
  both are pure moves with no behaviour change, each needs its own commit, and the split is what would have
  unblocked a per-path `PublishCommitMismatch` hint (T3 approach B). If the decision is to leave them alone,
  this document's `## Log` must say so explicitly — a 399/400 file is a blocking constraint on the next
  person, and "we chose not to split it" is the only honest alternative to splitting it. Route: inline,
  moves only.
- **T8 — Close.** `sh scripts/check.sh` (9 of 10 steps on this host; the tenth needs
  `x86_64-linux-gnu-gcc`, absent here — report it, never widen it), every unit observed RED first, and one
  scratch-repository run of `cutver publish --resume` showing the qualified hint and the `Resume` line.
  Record the run verbatim in `## Log`.

## Decisions

Recommendations, not accepted decisions: S1 defers execution to one word, so these are what «continuemos»
accepts or overrides in the same breath.

- **D1 — Order.** T1 → T2 → T3 → T5 → T4 → T6 → T8, with T7 pulled in only under its own condition:
  cheapest and safest first, each independently
  mergeable, so the queue can stop anywhere. T4 is the only task that changes a struct the CLI layer
  reads and is the only one worth dropping if review load is the constraint — nothing breaks without it.
- **D2 — T3 pays with wording, not with a variant.** Approach A (S3, zero lines) over B, because the
  ceiling at `src/bump/types.rs` is one line and a pre-emptive extraction would be phase 5's mistake
  repeated.
- **D3 — T4 pays, because its recorded blocker is false.** `src/cli/ci/tests.rs` has headroom; the second
  `PluginManager` is a duplicated config/plugin load per resume, not a necessary cost.
- **D4 — T6 records, never implements.** Gaps 2, 5, 8 and 9 stay phase 6's; making them open here would
  silently widen this plan past what phase 5's scope line allows.
- **D5 — No hand-written changelog entry, no new issue, no push or PR without an explicit ask.** `bump`
  generates the note (S8); publishing the branch stays the maintainer's call.

## Log

**L1.** «deudas de la fase 5» — the request that opened the inventory.

**L2.** «Deja todo preparado para atacarlo después, cerré demaciadas PR entre ayer y hoy, te voy a decir
continuemos y debes de saber todo sin tener que volver a explorar» — the request that turned the inventory
into this plan. Execution is deferred by the maintainer, not by a missing decision.

**L3. Evidence state when this plan was written, verified against `main` at `cec818e`.** Read directly:
`src/bump/types.rs:70-95` (`PublishCommitMismatch` `#[error]` at `:82-84`, variant at `:85`; file 399
lines), `src/bump/exec/publish/validate.rs:55-85` (commit check `:68-77`, the single raise site at `:71`),
`src/cli/runner/publish.rs:100-135` (`publish_sections:90`, `left_alone_line:114-136`), and
`src/bump/exec/publish.rs:118-129` + `:196` (`PublishSummary` fields, the `Ok` construction). Counted:
`src/bump/exec/publish.rs` 448, `src/cli/ci/tests.rs` 215, `src/bump/types.rs` 399,
`src/cli/runner/publish.rs` 160. `tests/source_line_budget.rs:32` still says 469, and a targeted grep for
`ci/tests.rs` in that allowlist returned nothing. Reported by an explorer subagent and not personally
re-read: the `docs/adr/0005` lines around `:84` beyond the quoted paragraph, `docs/internals/pipeline.md:108`,
`.git/issue-ci-native.md` line numbers, and the range `tests/release_deferred_publish.rs:1221-1458`. Open
each before acting on T5 or T6; the rest of this document needs no re-reading.

**L4. Three corrections to the phase's own record, made here.** (a) `src/cli/ci/tests.rs` is not at any
ceiling — the phase's stated reason for the second `PluginManager` is a fixture-touch cost, not the ratchet
(S3, D3). (b) `CHANGELOG.md` having no phase-5 entry is not a debt but the release that has not happened
(S8). (c) The stale `publish.rs` allowlist entry (469 against 448) is a debt the phase created and did not
list (S4). No TODO/FIXME/HACK marker exists in any phase-5 source file: the debts live in prose only, which
is why this document — not the code — is their index.

**L5. What «continuemos» must be read as.** Accepting D1-D5 and executing T1 through T8 on
`chore/phase5-debts`, with no further exploration: this document plus the files it names are the whole
input. Any task whose named evidence does not match the tree is an incident to report, not a cue to widen
scope.

**L6. Reconciled with the phase's own closing record (memory 166, topic
`odd/prerelease-channel-and-resume/tasks`).** That record lists the same three technical debts in the order
"3 → 1 → 2", where debt 3 is the ratchet pressure and debt 1 carries the note that the per-path hint "needs
~10 lines" and is "blocked by `types.rs` having 399 of 400". Both statements are right and both become
inapplicable once T3 takes approach A: a reworded static message is true on both paths and costs no line, so
the split (now T7) leaves the critical path. The manager debt maps to T4 unchanged. The phase record also
ends with an open question this plan does not answer — "whether #326 closes with gaps 2, 5, 8 and 9 out of
scope" — which T6 turns into evidence instead of a question.

**L7.** «continuemos» — D1-D5 accepted as written (order T1 → T2 → T3 → T5 → T4 → T6 → T8, T7 conditional on its own condition), execution started on `chore/phase5-debts` off `cec818e`.

**L8. T6 — gaps 2, 5, 8 and 9 re-checked against the tree, one line each.** Sources: `.git/issue-ci-native.md`
(gap 2 `:41-47`, gap 5 `:65-71`, gap 8 `:89-95`, gap 9 `:97-103`, sequencing `:114`), still present and
matching the quotes in S6.

- **Gap 2 — the artifact model — CLOSED.** `src/config/artifact.rs:1` declares `[[artifact]]` (a glob and
  the name a message uses), and `src/bump/exec/publish/validate.rs:25` (`verify_artifacts`) refuses a
  declared glob that matches nothing before the first irreversible step. The config can say what a
  release produces, and the tool checks it.
- **Gap 5 — the machine contract is versioned — CLOSED.** `src/cli/json.rs:15` is `SCHEMA_VERSION = 1`
  and `:32-35` wraps a payload with it, so every `--json` surface that goes through the wrapper carries
  the version; `src/bump/plan/tests.rs:147` pins the object with `"schemaVersion":1` first.
- **Gap 8 — declared per-phase needs — OPEN.** Nothing declares what a phase needs: `src/cli/args/commands.rs`
  has no such flag, and every `requires` in the tree means a release gate (`src/config/types.rs:114`),
  which is a different concept. Sequenced to phase 6 (`.git/issue-ci-native.md:114`).
- **Gap 9 — the repository root is ambient — OPEN.** `grep '"root"' src/cli/args/*.rs` returns nothing, so
  the working directory is still the implicit root (`src/config/discovery.rs`). Sequenced to phase 6.

Both open gaps stay phase 6's (D4): recording them here is the deliverable, and the #326 status table is
`odd/tasks/release-channel-dogfooding.md` T1's write, not this branch's.

**L9. T7 — the ratchet pressure, and which files stay full on purpose.** The condition fired: T4 needed a
line in `src/bump/exec/publish.rs`, which sat at exactly the 448 T2 had just recorded. Two pure moves gave
it room — `publish/post_release.rs` (the `on_post_release` dispatch, 54 lines) and `publish/context.rs` (the
interpolation context, the notes and the previous version, 70) — and the facade is now **356**, inside the
400-line ceiling, so its `ALLOWLIST` entry is gone and the table is **14 rows**. `git diff -M` reads both as
moves; the only behaviour change in the tree is T4's.

Left alone deliberately, with the measurement that says so: `src/bump/types.rs` **399/400** (T3 took
approach A, so nothing in T1-T6 owed it a line), `src/cli/runner/tests.rs` **949/950** (allowlisted and
cannot grow; no task needed a line there, and a split with no defect behind it is churn), `src/bump/plan.rs`
**390/400** (10 lines of headroom, untouched). Decision: not split. The next change that lands in any of
them owes the extraction in its own commit, because a file at its ceiling is a blocking constraint on the
next person, and "we chose not to split it" is the only honest alternative to splitting it.

**L10. T8 — close: gate, the scratch run, the differential and the commit map.** All six code units were
observed RED first except T5, which is a characterization test for behaviour that already existed and had
no test (there is no fix to redden): T2 failed with `["src/bump/exec/publish.rs was 447, is now 448"]`; T3
failed on both paths with the old sentence (`is not HEAD ('4df0e19...')\n  Where: ...\n  Fix: ... or drop
`--commit` to publish the current HEAD`); T4 failed to compile with `expected &Config, found &[_; 0]` at the
quote of the line that still loaded the second manager. T1 is prose, T6 read-only, T7 pure moves.

`sh scripts/check.sh` — **PASS, 9 passed / 0 failed / 1 not run here**: toolchain, lean dependency shape,
build, test (default), test (plugins), clippy (default), clippy (plugins), clippy (x86_64-unknown-linux-gnu
default), format. The step that did not run is `clippy (x86_64-unknown-linux-gnu, plugins)`: `zstd-sys` via
extism compiles C for that target and needs `x86_64-linux-gnu-gcc`, which is not on PATH on this host. A
skipped step is not a pass and it is not widened here.

The scratch-repository run, verbatim (a 1.2.3 repo with `[publish] push = false`, no commands and no
plugins, release commit `3230009` while HEAD is `2b81f43`):

```
== 1. a --commit that is not HEAD: the qualified Fix (T3) ==
Error: release commit '32300095e50277138bdbefea6775b1ee56376b08' is not HEAD ('2b81f43c1e963de16d2834603e4131f5238d3859')
  Where: `cutver publish` validation, before the tag, the pushes and the publish commands
  Fix: check out the commit you meant to publish (`git checkout 32300095e50277138bdbefea6775b1ee56376b08`) and re-run. Dropping `--commit` is not an alternative: a plain `cutver publish` with no `--commit` publishes the current HEAD, and `--resume` requires `--commit`. Nothing was written.
EXIT=1

== 2. --resume over that release: the Resume section (T4) ==
✔ Resumed: v1.2.3 at 32300095e50277138bdbefea6775b1ee56376b08
├── Tag
│   └── v1.2.3
├── Floating tag
│   └── v1
└── Resume
    ├── repeated only the steps that are harmless to repeat: the release tag, its push, the floating major tag and the gates
    ├── a tag, a push or a floating major tag already in place was left as it was, and no `[publish] commands` entry ran without its `publish-<n>` attestation
    └── `on_post_release` not dispatched: this workspace declares no plugin for that event

== 3. the evidence the run left ==
v1 -> 48d400a
v1.2.3 -> fa56489
3230009
```

The differential check T4 owes, because the diff changes `post_release::dispatch`, which `bump` also reaches
and this plan did not touch: one scratch repository, reset to the same commit with `v1` restored between
runs, a `[publish] commands` entry and a real `on_post_release` plugin, `bump patch` run with the binary of
`cec818e` and with this branch's. Exit code, stderr, commit subject, tag list, the publish-command run, the
plugin dispatch count, HEAD and the floating major tag are **identical** (`EXIT_DIFF=0`, exit `0`, stderr
empty, one `published 1.2.4` line, one dispatch). Fixed commit dates are what make the two commit shas
comparable; nothing else was normalised.

Commit map on `chore/phase5-debts` off `cec818e`: `ec924ad` this document · `6c99489` T1 · `5ebb30f` T2 ·
`ddec386` T3 · `34dced2` T5 (tests, `pipeline.md`, and — after an amend folded it — this document's L8/L9) ·
`62c1468` T7's split of `publish.rs` · `29f4d3f` T4. One boundary is imperfect and is not rewritten: L8/L9
were meant to be their own commit and ended up inside T5's. The tree is right, the history is readable, and
rewriting a local branch to move two paragraphs is not worth the risk of a second surgery.

Pending, and not this branch's to do (D5): push, pull request, merge, and the #326 status table — that write
belongs to `odd/tasks/release-channel-dogfooding.md` T1. Nothing was published to the remote.
