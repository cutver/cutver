# ADR-0004: Name the prerelease ladder a stage, and leave "channel" to the surfaces that already own it

**Status**: proposed
**Date**: 2026-11-25
**Owner**: repository maintainer
**Review date**: 2027-11-25 (proposal; not yet committed by the maintainer)
**Implementation**: not started
**Supersedes**: none

## Context

Issue #326 gap 6 asks for `--channel` and a `release` bump level, and `docs/internals/design.md:135`
carries the row `Pre-release channels (--channel, bump release) | Planned`, with the reason "No
`--channel` on `bump` and no `release` level (`src/cli/args/commands.rs:35-47`)". The reason is still
true. What has changed is the word.

**"Channel" now names four different things in this repository, and one of them is public.**

1. **The CI output channel** — shipped. `src/cli/ci.rs:1` opens with "The CI output channel: what a run
   tells the runner (#326, phase 4A)". It has three sinks, none of them stdout, and it is user-facing in
   the `--help` text of the flag that turns it on: `src/cli/args/commands.rs:13` and `:16` say "Turn the
   CI channel on" and "Turn the CI channel off". `docs/reference/cli.md:37` publishes the same sentence
   as the reference row for `--ci` / `--no-ci`.
2. **The gate channel** — shipped, and it is a store of attestations, not an output. `src/bump/exec/guard.rs:115`
   speaks of "what the gate channel does hold"; `required_gates` (`:105-113`) unions the standing
   `[gates] require` declaration with the invocation's `--require-gate`; `src/bump/plan/finish.rs:207`
   reports "the conclusion an attestation records for the commit this plan reports".
3. **A distribution target** — the contract's own prose. `CONTRACT.md:222`: "GitHub is a channel, never
   an identity"; `CONTRACT.md:213` uses "a channel cost" for artifact size.
4. **The prerelease ladder** — the meaning #326 wants, and the only one that does not exist yet.

Two consequences follow. First, a rename of any existing sense is a contract break with a deprecation
cycle, not an edit: `CONTRACT.md:232` describes the live channel for exactly that (#272). Second, adding
the fifth sense now makes every later collision expensive, and the cheapest moment to avoid it is before
the flag exists.

The governing constraint is already decided. [ADR-0002](0002-shared-version-with-path-filtered-changelogs.md)
adopted one shared version per repository and deferred independent release cadence, so a prerelease
"channel" here **cannot** be an independent release train. Whatever we call it, it is a state of the
single version.

**What exists to build on.** `is_prerelease` and `prerelease` are already carried, but only as rendering
facts parsed out of the version string for templates (`src/changelog/context/release.rs:39-40`,
`:183-189`; `src/changelog/context/types.rs:19-20`). There is no release **behaviour** attached to them:
a search of non-test `src/` for `draft` or `promote` returns nothing, so the forge draft→published flip
that CI performs today is `gh release edit --draft=false` in workflow YAML and not a cutver verb.

## Decision

We will name the prerelease ladder a **stage**: `--stage <alpha|beta|rc|stable>` on `bump` and `plan`.

Promotion is the same flag applied to an existing prerelease: `cutver bump --stage stable` on
`1.2.0-rc.1` produces `1.2.0`. There is no separate promotion verb, and no new state machine.

The word **channel** is left to the CI output channel and the gate channel, both already shipped. The
forge draft→published flip stays out of this decision and out of this release: it is a different
question (what a forge release state means) with its own reopen condition below.

## Alternatives

| Option | Trade-off | Verdict |
| :--- | :--- | :--- |
| **Null case** — leave the ladder Planned | No new surface; CI consumers keep writing `gh release edit --draft=false` by hand, and `design.md:135` keeps a row that never moves | **rejected** — the whole point of #326 is that the pipeline stops living in the consumer's YAML |
| **(a)** `--channel` for the ladder, as #326 words it | Reads well in isolation; becomes the fifth sense of a word already printed in `--ci`'s `--help` (`src/cli/args/commands.rs:13,16`) and in the published reference (`docs/reference/cli.md:37`), and collides with the attestation store (`guard.rs:115`) | **rejected** — the collision is with public text, so fixing it later is a deprecation cycle, not a rename |
| **(b)** `--stage` on the version, promotion as the same flag | One authority (the version), no new verb, no state machine; the ladder stays a property of a semver string, which is where `is_prerelease` already reads it | **accepted** |
| **(c)** A separate `cutver release promote` verb | Natural once promotion must also flip the forge draft state; today it duplicates the version authority and needs a ledger the plan does not carry | **dropped for now** — reopen condition below |
| **(d)** Channel as a distribution target (separate registries) | No demonstrated consumer; `[publish].commands` already covers publishing to a second destination | **named as today's answer** |

## Consequences

- Easier: the ladder rides the version, so `bump`, `plan` and the changelog context agree by construction, and `--stage stable` is the version move #326 gap 6 describes rather than a second mechanism beside it.
- Easier: `design.md:135` stops citing a flag that will never exist, and `docs/reference/cli.md` gains a row that does not repeat the word "channel" three rows apart.
- Harder: `--stage` and the four existing `channel` uses sit in the same `--help` output, so the reference must name both senses explicitly rather than relying on the reader to infer it.
- Downside: `bump --stage stable` is a version move, not a publication, so promoting still cannot flip a forge draft. A consumer that wants "publish the release I already built" keeps doing it by hand until (c) reopens.
- Downside: `--stage` on `plan` is a second place to keep the ladder honest; a plan that predicts a stage the bump then refuses would be a new class of disagreement between the two commands.
- Kept honestly: this decision does **not** deliver the draft→published flip, and it does not claim the forge state is unimportant. It only refuses to let one word carry both.

## Reopen condition

Option (c) reopens **only** when the forge draft→published flip enters scope, and the work then **starts
from the forge release state, never from the flag**.

Option (a) reopens **only** if the CI output channel is renamed first — that is, if `--ci`'s `--help`
text (`src/cli/args/commands.rs:13,16`) and `docs/reference/cli.md:37` stop calling it a channel. Until
then the word is occupied by a shipped surface.

The null case reopens if a consumer demonstrates that `--stage` duplicates an existing mechanism.

## Traceability

- Issue: [#326](https://github.com/cutver/cutver/issues/326), gap 6, and phase 5 of its sequencing.
- Sibling decision: [ADR-0002](0002-shared-version-with-path-filtered-changelogs.md) — one shared version per repository, which is what makes a release channel a state rather than a train.
- Roadmap row this decision corrects: [Evolution Roadmap](../internals/design.md#evolution-roadmap), `Pre-release channels (--channel, bump release)` at `docs/internals/design.md:135`.
- Surfaces that already own the word: `src/cli/ci.rs`, `src/bump/exec/guard.rs:105-115`, `src/bump/plan/finish.rs:207`, `CONTRACT.md:213,222,232`, `docs/reference/cli.md:37`.
- No issue or pull request number is recorded for the implementation of this decision, because there is none yet.
