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
  `plan` a dry run (`Pillar I.2`). D3 answered: no machine-contract change; the human tree names the transition.
  Done — commits `eb87739` (room under the 400-line ceiling) and `9c86966` (the flag, its arithmetic, its
  tests and the reference rows).
- **T3** Promotion: `--channel stable` on an existing prerelease produces the release version. Must state
  what it does to the tag, the floating major tag and the changelog entry before writing it. D1 answered:
  any prerelease promotes, with no rung validation.
- **T4** `publish --resume`, driven by the plan that gap 1 already ships. D2 answered: repeat only
  idempotent steps, and refuse when the evidence contradicts the plan.
- **T5** Docs with the behaviour: the `--channel` reference row and help text (bare "channel" = the
  ladder), the guide that explains promotion, `docs/internals/design.md:135` from `Planned` to
  `Implemented`, and the qualifier test of S3 if it does not exist yet.
- **T6** Measurement: `sh scripts/check.sh` green, the new tests red before their fix, and one documented
  run of the promotion path end to end on a scratch repository.

## Decisions

Three decisions were open when this plan was written. They were closed before any code, by the maintainer
accepting the recommendations below, and they are decisions now: a later change that disagrees argues with
the reasoning, it does not re-open the bullet.

**D1 — promotion does not validate the rung.** Any prerelease promotes to `stable`; `1.2.0-alpha.3
--channel stable` produces `1.2.0`, and skipping `beta` and `rc` is allowed. ADR-0005 rules out a separate
promotion verb and a new state machine, and refusing `alpha → stable` would be exactly that machine — an
order enforced between suffixes the tool otherwise knows nothing about. The four cases T3 implements and
pins:

- same channel on a prerelease: increment the counter (`1.2.0-alpha.1` → `1.2.0-alpha.2`);
- a different channel on the same base version: start that channel at 1 (`1.2.0-alpha.3 --channel beta` →
  `1.2.0-beta.1`), where the `level` argument is ignored;
- `stable` on a prerelease: drop the prerelease segment (`1.2.0-rc.1` → `1.2.0`), no increment;
- `stable` on a stable version, or the channel the version already carries with no level: a **reported**
  no-op — not a silent success, and not an error, because erroring would force every caller to branch on
  the current version, which is the inference this tool exists to remove.

**D2 — `--resume` repeats only what is idempotent.** It re-reads the plan and refuses whenever evidence
disagrees with it: HEAD moved, a manifest's version changed, or a tag that exists pointing elsewhere.
Within that, a step may be repeated only when repeating it is harmless — creating a tag that already
matches, pushing it, moving the floating major, verifying a gate (read-only). `[publish] commands` is not
on that list: it is an arbitrary command, so `--resume` refuses to run it again unless the gate channel
already holds an attestation naming it as done, and it names the step when it refuses. A retry that
re-runs `cargo publish` because the tool could not tell is the failure this decision exists to prevent.

**D3 — `plan --json` does not grow a `channel` field.** No machine-contract change, so gap 5 stays
untouched and T2 cannot alter an unversioned surface by accident. The information is already in
`touched[]`: `1.2.0-rc.1 → 1.2.0` is the promotion. What the human tree must add is the transition in
words — the channel the run asked for and the channel the version ends in — so approving a promotion is
not parsing suffixes by eye.

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

**L5. The three decisions, closed before code.** The maintainer accepted the plan's recommendations:
promotion does not validate the rung (D1); `--resume` repeats only what is idempotent and refuses when the
evidence contradicts the plan (D2); and `plan --json` gains no field because `touched[]` already carries
the transition (D3). D3 was the one that could have reshaped T2 — it removes work instead of adding it and
leaves gap 5 exactly where #326 says it is.

**L6. T2 landed, and the ceiling chose part of its shape.** `src/bump/plan.rs` and `src/cli/tree.rs` were
both at 399 of the 400-line hard ceiling and neither is allowlisted, so the ratchet's own remedy applied
before any feature line was written: `PlanMode`/`ComputeRequest` moved to `src/bump/plan/request.rs` and the
renderer's tests to `src/cli/tree/tests.rs` (`eb87739`, no behaviour change). `src/cli/args/tests.rs` is
allowlisted at 738 and may not grow, so `plugin`'s three argument tests moved to
`src/cli/args/tests/plugin_args.rs` inside the feature commit, paying for the six `let Commands::{Bump,Plan}`
patterns that now bind `channel`. The tree needed no new `Summary` field: it reads the channel of the two
versions the header already prints, which is why every release without a channel prints exactly what it
printed before. Slice boundary: T2 is two commits and 968 authored lines, and the phase's delivery shape —
one PR or chained ones — is decided when it is published, not here.

**L7. What T2 does not do, on purpose.** A version that is *already* a prerelease still takes the level and
then the channel, so `1.2.0-alpha.1 --channel alpha` cuts `1.2.1-alpha.1` where D1 requires `1.2.0-alpha.2`.
That is T3, together with the other two transitions of D1 and the statements about the tag, the floating
major tag and the changelog entry it owes before it is written.
