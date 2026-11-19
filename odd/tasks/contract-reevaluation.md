# CONTRACT.md re-evaluation, and the checks that make it binding

Branch `docs/contract-reevaluation`, cut from `origin/main` at `1695813`. Input is the four pending sections of
`odd/tasks/deferred-findings.md` (lines 71-176), which record decisions taken 2026-11-19. Homonymous work item of
#326's pre-4B queue.

Ordering principle: **the checks land before the prose that cites them.** A clause declaring a rule that no check
enforces is the exact defect this re-evaluation exists to remove, so the branch may not create one on the way.

## Specs

**S1 — the size number goes, the measurement does not.** *"Replace Pillar VI.4's ~4 MB target with a shape invariant
that can be checked mechanically — the lean build's dependency graph must not contain `extism` or `wasmtime` — plus
size *reported* per release rather than enforced."* Rationale to preserve verbatim: *"the number goes, the measurement
does not."* Today `CONTRACT.md:178` is one aspirational sentence with no threshold, no tool, and nothing asserting it;
docs say ~1.25 MB lean / ~5 MB plugins while the contract says ~4 MB.

**S2 — five numbers, three enforcement levels, one rule.** *"either a number is enforced or it is stated as a target
with the evidence rule attached."* The five and their real levels: 200 doc lines (enforced, works), 400 source lines
(enforced as a ratchet), 350 facade (judgement, says so), 300-400 review advisory (violated by three of four phases),
4 MB (fantasy).

**S3 — every declaration ships with its check.** *"Add the rule that every declaration ships with its check. In one
session four declarations were found unenforced: `permissions.env` (#300), `[gates] require`, `--ci` without a sink,
and the 400 ceiling itself. That is a pattern, not four accidents."*

**S4 — scope boundary, now that growth is welcome.** *"GitHub is a channel and never an identity (every CI integration
must degrade to stdio and exit codes); the core grows no HTTP client and no provider SDK; plugins extend the four typed
slots and nothing else."*

**S5 — compat and deprecation policy.** *"additive within a version, a major bump needs a deprecation cycle with a
warning channel (#272 is still open), and a surface cannot be removed in the same release that renames it."*

**S6 — the survivor rule.** *"Compaction moves a fact to another page; it never drops one."* `machine-contract.md` was
compacted to 199 lines in 4A and lost four statements of behaviour doing it.

**S7 — documentation freshness, in three parts, stopping where a test would start lying.** (1) *same commit* — a new or
changed surface ships its reference row and at least one worked example; (2) *freshness* — shipping a surface obliges
revisiting the pages that describe the flow *around* it; (3) *examples that were run* — *"an example is a transcript or
it is a claim."* Enforcement: surface parity between the clap tree and `docs/reference/cli.md` is mechanical; documented
`cutver …` lines in fenced blocks must **parse** (parse only, no execution); narrative pages (`design.md`,
`pipeline.md`, the guides) stay a review obligation and the rule must say so, because *"no test can verify that a
sentence is still true"*. A `last-verified: vX.Y.Z` stamp is vanity — do not add one.

**S8 — the doc ceiling stays, the trigger moves, the hole closes.** *"Keep it. Do not raise it. Do not remove it. Move
the trigger."* `trigger at 150, ceiling stays 200`; *"Crossing 150 obliges a decision: split the page, or record in the
PR why it is one thing."* `CONTRIBUTING.md` is outside the walk (`tests/ci_workflow_contract.rs:288-291` collects
`README.md` plus `docs/**`) — *"Either include it or exempt it with a written reason."* Decision: include it.

**S9 — `CONTRACT.md` itself gets a named exemption.** Maintainer's answer, 2026-11-19: *"Exención nombrada con razón"*.
The file is 181 lines and the four new clauses take it past 200; it is scanned rather than read linearly, which is the
rationale the ceiling's own exception clause already allows.

**S10 — the three mechanical checks land in this branch.** Maintainer's answer, 2026-11-19: *"Los tres checks ahora"* —
lean dependency graph, clap-to-reference parity, and parseable fenced examples. They are the enforcement half of S1, S3
and S7.

## Tasks

| ID | What | Specs | Route | Commit |
| :--- | :--- | :--- | :--- | :--- |
| T1 | Close the `CONTRIBUTING.md` hole: extend the doc walk in the test and the CI loop | S8 | inline | `a680e2f` |
| T2 | Lean dependency-graph check: the lean build carries no `extism` or `wasmtime` | S1, S10 | inline | `281765d` |
| T3 | Clap-to-`docs/reference/cli.md` parity test, and fix the `cutver open` drift it finds | S7, S10 | inline | `e4c9352` |
| T4 | Fenced `cutver …` lines must parse against the CLI, plus the example convention it needs | S7, S10 | inline | `852c411` |
| T5 | Rewrite `CONTRACT.md`: S1-S9, each clause naming the check that binds it | S1-S9 | inline | `3426a86` |
| T6 | Align the size figures in the docs with the measurement, and say where size is reported | S1 | worker + verifier | `4c3294e` |
| T7 | Run the local gate and record its result | — | inline | this commit |

## Log

- **L1** — Maintainer, session opener: *"Volví, no pude dormir, con qué seguimos de los pendientes?"*
- **L2** — Choice of starting point: *"Re-evaluación de CONTRACT.md, completa"*.
- **L3** — Scope of the checks: *"Los tres checks ahora"*.
- **L4** — `CONTRACT.md` against the ceiling: *"Exención nombrada con razón"*.
- **L5** — Evidence gathered before the first write: `CONTRACT.md:178` is the only unenforced number; nothing in
  `tests/`, `src/` or `.github/` asserts binary size; no `cargo tree`/`cargo metadata` assertion exists anywhere; no
  `docs/reference/cli.md` parity test exists and `cutver open` (`src/cli/args/commands.rs:131`) has no section in that
  page; `CONTRIBUTING.md` (130 lines) and `CONTRACT.md` (181) are both outside the doc walk.
- **L6** — Ordering decision: checks first, contract prose last, so no commit on this branch introduces a declaration
  whose check does not exist yet.
- **L7** — What the checks found on their first run, which is the argument for having built them: the parity test
  reported `cutver open` with no reference section *and* `cutver changelog --path` undocumented in both nested
  subcommands; the transcript test failed on fourteen lines that were all syntax diagrams, which is why the
  convention became the fence tag; the ratchet test corrected its author about `wasmtime`, which is transitive and
  therefore invisible to any manifest reading.
- **L8** — T3 could not document `open` without decomposing: nine commands at 195 lines against a 200-line ceiling
  means the next fact costs a fact. `docs/reference/cli-tools.md` took the four commands that report, read or reach
  outside the repository, whole; `cli.md` kept exit codes, the `--ci` channel and the five release commands at 146.
- **L9** — T6 went to a worker and then to an independent verifier because it edits the release workflow (risk item 5).
  The verifier confirmed the structure, confirmed the size report cannot fail a run, and confirmed no doc fact was
  dropped; it also found that the first version of the summary rows ran `basename` as a nested command substitution,
  so the artifact name never reached the table while the step exited 0, and that the report test's job anchoring could
  be fooled by moving the step into a job that never fills `dist/`. Both are fixed, and the mutation was replayed to
  prove the new assertions are not vacuous.
- **L10** — T7, the local gate on this host: **PASS**, 9 steps passed, 0 failed, 1 not run here
  (`clippy (x86_64-unknown-linux-gnu, plugins feature)` needs `x86_64-linux-gnu-gcc`, which this host does not
  have — a host limit that predates the branch, reported as not-run rather than as a pass). The `lean dependency
  shape` step is the new one in that list. `cargo test --locked --workspace` is 503 unit tests plus the integration
  suites, all green, and `git diff --name-only 1695813..HEAD` contains **no file under `src/`**: this branch changes
  tests, workflows and prose, not behaviour.
- **L11** — Known obligations for the pull request, so they are not rediscovered there: `docs/reference/plugins.md`
  is 197 lines, above the 150 trigger that S8 introduces, so the PR says why it is one thing or splits it; the size
  figures now live only in a release run's job summary and a reader on the releases page sees the archives without
  them, which is a deliberate first step and not a promise the pipeline keeps beyond that; and #272 (the deprecation
  warning channel) is referenced by Pillar VII.2 as an open dependency, not as completed work.
