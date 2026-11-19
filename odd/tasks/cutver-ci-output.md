# CI output — phase 4A of #326 (the tool writes what the workflow derives with jq today)

Slice 4A is tool-side only: detection, `$GITHUB_OUTPUT`, step summary and annotations. It does **not** touch
`.github/workflows/release.yml` — that is 4B, a separate PR, so each lands reviewable.
Requirement source: issue #326 "Where: 4. The tool does not speak the CI contract" and acceptance criterion 6.

## Specs

S1. CI is detected when `GITHUB_ACTIONS` reads as true — compared after trimming and case-insensitively, because a
runner may set `TRUE` — **and** at least one of `GITHUB_OUTPUT` / `GITHUB_STEP_SUMMARY` names a writable file. The
channel defaults to on when detected. `--ci` and `--no-ci` are **global** flags: every command accepts them, `--ci`
forces the annotation channel on even when no file sink is named, and `--no-ci` turns the whole channel off, annotations
included. A flag that silently does nothing is the decorative-declaration defect in another costume. No other provider is
detected: a second provider is a later table, not a guessed heuristic.

S2. `$GITHUB_OUTPUT` receives the same field names the machine payload already carries, one `key=value` per line,
appended and never truncated. `bump`: `released`, `deferred`, `version`, `tag`, `commit`. `plan`: `next_version`, `tag`,
`commit`. A value that is absent is omitted rather than written empty. These are exactly the five keys
`.github/workflows/release.yml:159` extracts with `jq` today, which is what makes 4B a deletion rather than a rewrite.

S3. `$GITHUB_STEP_SUMMARY` receives one markdown block per run: for `bump`, what was released (version, tag, commit,
and whether publication was deferred); for `plan`, the plan in brief. Any text that originates in user content — a
changelog entry, a commit subject — is escaped or fenced so it cannot inject markdown structure, and the block is
bounded: no changelog dump, ever.

S4. On failure, one `::error::` line is written to **stderr**, carrying the same text as the human error, so the run page
and the pull request show what failed instead of burying it in a log. On success, no annotation. The annotation is
emitted whenever CI is detected, independently of machine mode; when the machine envelope is emitted too, it follows the
annotation on its own line. **Nothing is ever added to stdout.**

S5. With CI detected, stdout is byte-for-byte identical to the same command with CI off, for every command. Pinned by a
byte comparison, never by a semantic one.

S6. With `GITHUB_ACTIONS` absent, the tool opens no file for writing, prints no annotation and creates no path. Pinned by
a test that strips the environment and asserts zero side effects.

S7. Docs: `docs/internals/machine-contract.md` gains the CI channel section **under its 200-line ceiling** (it is at 198
lines today, so prose must be compacted without losing a documented behaviour, and migration history is the first
candidate), and `docs/reference/cli.md` documents the two flags. The doc states plainly which stream annotations use —
this repository is silent on it today, so the statement is new information, not a restatement.

## Tasks

- [ ] T1 — S1: `src/cli/ci.rs` with the detection rule and the `--ci`/`--no-ci` flags.
- [ ] T2 — S2: the `$GITHUB_OUTPUT` writer and the per-command key mapping for `bump` and `plan`.
- [ ] T3 — S3: the step-summary writer, with escaping and a bound.
- [ ] T4 — S4: the annotation on the failure path, through the hook that already exists for every command.
- [ ] T5 — S5, S6: byte-invariance over stdout and the no-side-effects-outside-CI proof.
- [ ] T6 — S7: docs, compacting `machine-contract.md` first.

## Log

L1 (user, verbatim): `Si pasa todo verde mergea. Mientras termina dime de la siguiente fase`

L2 (decision, user, verbatim): `Fase 4A y parar para revisar`

L3 (requirement, issue #326, verbatim): `**What**: no CI detection, and no native output channel — `$GITHUB_OUTPUT`,
step summary, annotations or check-runs.` … `**Where**: `grep -rn "GITHUB_OUTPUT\|GITHUB_STEP_SUMMARY" src/` → 0
matches.` — and acceptance criterion 6: `On a detected CI, the tool writes the outputs the workflow currently derives with
`jq`, and the workflow stops parsing the report itself.`

L4 (evidence, `path:line`): the workflow's derivation is one step — `.github/workflows/release.yml:151` `Read the release
report`, `id: cutver` (`:152`), reading `.cutver-release.json` (`:154`) and extracting exactly `released deferred version
tag commit` (`:159`) into `$GITHUB_OUTPUT` (`:161`), consumed in-job at `:165,:169` and cross-job as
`needs.version.outputs.{released,commit,tag,version}` throughout. `bump --report` writes that file
(`src/cli/runner/bump.rs:107-115`). **The failure side has a single hook for every command** (`src/cli/machine.rs:149
try_emit_failure`, reached through `src/cli/runner/dispatch.rs:22,35`); the success side has **no** funnel — each command
prints its own output and the `match` in `dispatch.rs:44-151` is the tail expression, so a success hook means binding the
result and emitting after it. No `GITHUB_*` or `CI` read exists anywhere in `src/`, and the only convention for an
environment input is one named variable resolved at a boundary with an injected lookup (`src/plugin/cache.rs:4-6,33-45`).

L5 (decision, parent): **the CI channel never writes to stdout.** `bump --json` already documents that its JSON is "the
last line … not necessarily the only one", because `[preflight]` steps and git subprocesses inherit stdout
(`src/cli/runner/bump.rs:1-6`, `docs/reference/cli.md:58`); any extra stdout write would corrupt `plan --json`,
`doctor --json` and `changelog --json` outright, which parse the whole document. So the two file sinks are files and the
one stream sink is stderr. This also keeps the machine contract's invariant ("nothing added to stdout",
`docs/internals/machine-contract.md:62-65`) literally true.

L6 (decision, parent): annotations on stderr rather than stdout, stated as a new fact in the docs because the repository
is silent on the contract today and only practises `echo "::error::…"` inside workflow `run:` blocks
(`release.yml:156,460,536,680-681`). A machine reader that parses stderr for the envelope keeps working: the annotation is
one `::`-prefixed line and the envelope follows on its own line, which S4 pins as an order, not a hope.

L7 (decision, parent): 4A stops at the tool. Criterion 6 has two halves — the tool writes the outputs, and the workflow
stops parsing the report — and only the first is verifiable inside this PR. 4B is a separate branch from a fresh
`origin/main`, and it is small: `tests/release_pipeline_contract.rs:199-201` is the only assertion that names the
report-reading step, and nothing asserts the `jq` text, the report path or the key list. The doc copies to keep honest are
`docs/integrations/github-actions.md:65-68` and `docs/integrations/production-release-workflow.md:90-92`.

L8 (review, independent verifier — two blockers, both found only by running with the environment CI actually has):

- **B1 (blocker).** Every GitHub Actions `run:` step has `GITHUB_ACTIONS=true` and both sink variables, so the annotation
  fires inside the project's own test suite. `tests/e2e_gates_channel.rs:404` counts occurrences of `gate 'ci'` in
  stderr and the annotation adds a second one: the suite went red in the exact environment CI uses, while a local run
  without the variables stayed green. Repair is not a per-file patch — **every** integration suite that spawns the binary
  and asserts on stderr or stdout must strip the three variables through one shared helper in `tests/common/mod.rs`, with
  `tests/ci_output.rs` opting in to setting them. The acceptance gate is the full suite green **with** `GITHUB_ACTIONS`
  and both sinks set, which is how it will actually run.
- **B2 (blocker).** The flags existed only on `bump` and `plan` while the annotation fires on every command, so a runner
  could not opt out: `cutver doctor --no-ci` was a usage error. Global flags fix both halves, and `--ci` without a named
  sink now enables annotations instead of doing nothing.
