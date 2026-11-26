# ADR-0005: Name the prerelease ladder a channel, and qualify the CI one

**Status**: accepted
**Date**: 2026-11-25
**Owner**: repository maintainer
**Review date**: 2027-11-25 (proposal; not yet committed by the maintainer)
**Implementation**: merged to `main` as the chained stack #391-#396. `9c86966` is `--channel` on `bump` and `plan` with its arithmetic, its tests and its reference rows (#392); `0a7a3ac` is promotion and the reported no-op, with `Summary::released()` behind `--json`, `--report` and the CI sinks (#393); `661ce19` is `publish --resume` (#394); `cef59a5` is the reference and the guide that describe the shipped behaviour (#396).
**Supersedes**: [ADR-0004](0004-prerelease-stage-not-channel.md)

## Context

[ADR-0004](0004-prerelease-stage-not-channel.md) rejected `--channel` for the prerelease ladder and
chose `--stage`. Its deciding argument was that "channel" is already printed in public text — in
`--ci`'s own `--help` (`src/cli/args/commands.rs:13,16`) and in the published reference
(`docs/reference/cli.md:37`) — so a fifth sense would make the next collision "a deprecation cycle
(`CONTRACT.md:232`), not a rename".

**That premise is wrong, and it was the load-bearing one.** A deprecation cycle is owed when a *flag
name* or a *schema key* changes meaning. Neither was proposed: `--ci` keeps its name, its behaviour
and its outputs; only the sentence describing it would change. Help text and a reference row are not
a parseable contract — `tests/cli_reference_parity.rs` reads flags, not prose. So the alternative
ADR-0004 rejected was cheap, and it rejected it on a cost it had invented.

Two things it did not weigh:

1. **"Channel" is the vocabulary of this domain.** `rustup` installs `stable`, `beta` and `nightly`
   and calls them channels. A user arriving from that world already owns the word for exactly this
   concept, and `--channel` is the spelling they will try first.
2. **"Stage" collides with something cutver already publishes.** `cutver plan --json` emits
   `phases[]` — `validate`, `mutate`, `publish`, each with a `mutates` boolean — and a "stage" is,
   in release tooling, a step of a pipeline. ADR-0004 traded a visible, mechanical collision for a
   quieter conceptual one with vocabulary the tool itself puts on stdout.

What ADR-0004 got right stays right: [ADR-0002](0002-shared-version-with-path-filtered-changelogs.md)
adopted one shared version per repository, so a channel is a state of that version and never an
independent release train. Only the word changes.

## Decision

The prerelease ladder is a **channel**: `--channel <alpha|beta|rc|stable>` on `bump` and `plan`, with
promotion as the same flag applied to an existing prerelease (`cutver bump --channel stable` on
`1.2.0-rc.1` produces `1.2.0`). There is still no separate promotion verb and no new state machine.

Two senses of the word coexist, and both carry their qualifier **every time they are written**:

- **CI output channel** — the `$GITHUB_OUTPUT` / step-summary / annotation surface. `--ci`'s `--help`
  (`src/cli/args/commands.rs:13,16`) and its row in `docs/reference/cli.md:37` adopt that wording in
  the same change as this ADR, so the qualifier is present before the second sense arrives.
- **Gate channel** — the attestation store, which already qualifies itself
  (`src/bump/exec/guard.rs:115`, `src/bump/plan/finish.rs:207`).

- **Stable-only surfaces** — **a prerelease publishes exactly its own version**: the floating major
  tag (`v0`), container `latest`, the Homebrew tap and the Scoop bucket all follow the stable release
  and nothing else. The floating tag is one rule decided in two places — what the plan reports
  (`src/bump/plan/resolve.rs`) and what `cutver publish` moves (`src/bump/exec/publish.rs`), a separate
  process that reads no plan — so both answer from the version being released rather than from a flag a
  caller remembered to pass. The other three follow from the same hazard: `uses: cutver/setup@v0`
  resolves to `version: latest` by default, so a surface that follows a prerelease hands an alpha to a
  consumer who asked for the default. Promotion produces a stable release, so `0.22.0` still moves all
  four. The crate is the exception and goes out on every channel: `[publish].commands` publishes it on
  each run, because the registry path is what the ladder exists to exercise, and no default requirement
  selects a prerelease — a consumer opts in by naming the version.

The bare word "channel", with no qualifier, means the prerelease ladder from now on.

## Alternatives

| Option | Trade-off | Verdict |
| :--- | :--- | :--- |
| **Null case** — keep `--stage` from ADR-0004 | No new decision; keeps a flag name that collides with `phases[]` and that this domain does not use for prerelease tiers | **rejected** — the collision ADR-0004 avoided was cheaper to fix than the one it created |
| **(a)** `--channel` for the ladder, the CI surface qualified | Standard vocabulary, one word per concept, and the qualifier costs two sentences | **accepted** |
| **(b)** Rename the CI concept itself (not just its prose) | Would make "channel" unambiguous with no qualifier at all; touches a shipped surface's identity, its anchor in `machine-contract-surfaces.md` and every page linking it, for a distinction the qualifier already makes | **rejected** — the qualifier is the cheaper tool for the same job |
| **(c)** `--dist-tag`, the npm spelling | Familiar to a JavaScript audience; a dist-tag is a *mutable pointer* (`latest` moves), while this ladder is an immutable version suffix — the metaphor would mislead | **rejected** — wrong semantics |
| **(d)** Qualify nothing and accept bare "channel" for all four senses | Zero prose change; the reader has to infer which channel from context, in help text | **rejected** — that is the state ADR-0004 was right to call out |

## Consequences

- Easier: the flag name is the one a user from `rustup` or the container world already guesses, and nothing new collides with `phases[]`.
- Easier: `docs/internals/design.md:135` reads `Pre-release channels (--channel, promotion)` as `Implemented`, citing the code that decides both cases (`src/semver_bump.rs:77`) and the decision that named it.
- Harder: every page that mentions the CI surface must keep its qualifier, forever. A test pins the two help strings so the qualifier cannot erode silently.
- Downside: "channel" is overloaded even when qualified. Three qualified senses is better than five unqualified ones, but it is not zero.
- Kept honestly: this ADR reverses one made hours earlier, and it records *why* — the earlier cost estimate was wrong, not the earlier evidence. The evidence ADR-0004 gathered about where the word appears is still accurate and still the reason the qualifier is required.

## Reopen condition

This reopens **only** with evidence that the qualifier does not work — a reader or a consumer confusing the CI output channel with the prerelease channel in a real report, issue or review. It does not reopen on the observation that the word is overloaded, which is already recorded above.

If it does reopen, the work starts from the CI surface's identity (option b), never from the ladder's flag.

## Traceability

- Superseded decision: [ADR-0004](0004-prerelease-stage-not-channel.md) — same question, and the reversal is the record.
- Issue: [#326](https://github.com/cutver/cutver/issues/326), gap 6 and phase 5.
- Roadmap row: [Evolution Roadmap](../internals/design.md#evolution-roadmap) — `docs/internals/design.md:135`.
- Surfaces that take their qualifier from this decision: `src/cli/args/commands.rs:13,16`, `docs/reference/cli.md:37`.
- Delivered as the chained stack #391-#396, one merge commit per slice, all six ancestors of `main`: `12fa98c` (#391), `34417ce` (#392), `fe5dbcd` (#393), `f0a4fc0` (#394), `7bfd241` (#395) and `f243e80` (#396).
- The six `pr/*` branches were deleted once each was verified an ancestor of `main`; `main` is the only branch on the remote.
- History: the delivery shape — one pull request with a commit-per-slice reading order, or chained ones — was left undecided when this ADR was written and was settled at publication. Nothing about the delivery is pending.
