# Phase 5 of #326 — the prerelease channel, promotion, and `publish --resume`

Branch: `feat/prerelease-channel` (first write opens it). Scope: gaps 6 and 7 of #326 only.
Decision: `docs/adr/0005-prerelease-channel-supersedes-0004.md` (accepted). Issue: #326.

## Specs

- **S1.** "The prerelease ladder is a **channel**: `--channel <alpha|beta|rc|stable>` on `bump` and
  `plan`, with promotion as the same flag applied to an existing prerelease (`cutver bump --channel
  stable` on `1.2.0-rc.1` produces `1.2.0`). There is still no separate promotion verb and no new state
  machine."
- **S2.** "Two senses of the word coexist, and both carry their qualifier **every time they are
  written**" — **CI output channel** and **gate channel** — and "The bare word \"channel\", with no
  qualifier, means the prerelease ladder from now on."
- **S3.** "A test pins the two help strings so the qualifier cannot erode silently." The prose half is
  already in place (`src/cli/args/commands.rs:13,16`, `docs/reference/cli.md:37`): this phase adds the
  ladder, not the qualifier.
- **S4.** "no `publish --resume`, no idempotent re-entry" — and the decision it owes: "what it **refuses**
  to resume, so a retry never hides a diagnosis" (#326, gap 7).
- **S5.** Non-goals stay non-goals: "A CI provider SDK or an HTTP client in the lean build", "Stdio, exit
  codes and files only (`Pillar IV.3`, `VI.4`)" (#326, `## Non-goals`).
- **S6.** Scope is the sequencing's own line: "**Channels and resume** — `--channel`, promotion,
  `publish --resume` (gaps 6, 7)". Gaps 2, 5, 8 and 9 are not part of it, and the issue records that 2, 5,
  8 and 9 were "not re-checked" — treat them as unknown, not as done.
- **S7.** The prerequisite this phase stood on is gone as of `a5ae78f` (#384/PR #388): the timeout family
  no longer reports a red that does not exist one run in four, so the gate is evidence again. No phase-5
  task may widen a timeout, serialise a test or accept a flaky red to get past it.

## Tasks

- **T1** Correct #326 to the accepted ADR (gap 6 and the sequencing pointed at ADR-0004 and `--stage`).
  Done — the issue now names ADR-0005 and links this plan.
- **T2** `--channel` on `bump` and `plan`. Parse and validate the four values, compute the version
  (`1.2.0-alpha.1` from `1.2.0`), report it in the human tree and in the plan's machine output, and keep
  `plan` a dry run (`Pillar I.2`). Blocked on **D3**.
- **T3** Promotion: `--channel stable` on an existing prerelease produces the release version. Must state
  what it does to the tag, the floating major tag and the changelog entry before writing it. Blocked on
  **D1**.
- **T4** `publish --resume`, driven by the plan that gap 1 already ships. Blocked on **D2**.
- **T5** Docs with the behaviour: the `--channel` reference row and help text (bare "channel" = the
  ladder), the guide that explains promotion, `docs/internals/design.md:135` from `Planned` to
  `Implemented`, and the qualifier test of S3 if it does not exist yet.
- **T6** Measurement: `sh scripts/check.sh` green, the new tests red before their fix, and one documented
  run of the promotion path end to end on a scratch repository.

## Open decisions — the plan does not invent them

- **D1 (promotion range).** Which suffixes promote. ADR-0005 says "an existing prerelease"; it does not
  say whether `1.2.0-alpha.3 --channel stable` is allowed (skipping beta and rc) or refused. Recommend:
  allow it and say so, because refusing invents a state machine the ADR ruled out.
- **D2 (what `--resume` refuses).** Candidates, and the issue demands the refusal be named rather than
  softened: a tag that already points somewhere else, a `cargo publish` that already ran, a draft release
  that already carries assets, a workspace that changed since the plan. Recommend: resume re-reads the
  plan and refuses on any step whose evidence disagrees with it.
- **D3 (the machine contract).** Whether `channel` enters `plan --json`. That contract is gap 5, which #326
  says is "thin and unversioned" and "not re-checked", and phase 1 (`schemaVersion`) is not assumed to
  have landed. Adding a field may be additive, but the decision belongs to gap 5. It has to be answered
  before T2 lands, because T2 is the change that would answer it by accident.

## Log

**L1.** «Bien, con 384 adentro ya tenemos via libre para la fase 5 del 326 como lo acordamos?» — and, when
asked how to start: "Corregir #326 y escribir el plan de fase 5", explicitly without code.

**L2. State verified against the tree, not assumed.** `docs/adr/0005-prerelease-channel-supersedes-0004.md`
is `accepted`, `Supersedes: ADR-0004`, `Implementation: not started`. No `--channel` in
`src/cli/args/commands.rs:35-47`. The qualifier half is shipped: `--ci`/`--no-ci` help
(`src/cli/args/commands.rs:13,16`), `docs/reference/cli.md:37`, and `docs/internals/design.md:135` cites
ADR-0005 with the row still `Planned`.

**L3. The drift this task fixed.** #326 named ADR-0004 as the deciding ADR in three places (status table
row 6, gap 6, sequencing point 5) and quoted `--stage`. ADR-0004 is superseded; the issue now points at
ADR-0005 in all three and keeps the reversal as the record rather than deleting it.

**L4. What #384 did and did not unblock.** It was never a dependency of this phase's code; it was the gate's
honesty. Phase 5 is a CLI surface verified by many suite runs, so an invented red one run in four was a tax
on every task here — and on the ADR work that preceded it, which is how #384 was found.
