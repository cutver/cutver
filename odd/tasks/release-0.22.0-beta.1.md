# Release 0.22.0-beta.1

The second rung of 0.22.0, cut from `main` by this repository's own release workflow
(`.github/workflows/release.yml`, `workflow_dispatch`). Tag: **`v0.22.0-beta.1`**. Previous rung:
`v0.22.0-alpha.1`. Notes: `.github/releases/v0.22.0-beta.1.md`.

## The dispatch

`bump=auto`, `channel=beta`, `dry_run=true` first. `cutver plan` derived the same
`0.22.0-alpha.1 -> 0.22.0-beta.1` from `auto`, `patch` and `minor` alike: a channel change dominates the
level, so no level could have produced `0.23.0`.

## What the dry run proved, and where it stopped

It stopped in `Check & Version`, before any file, commit or tag was written:

```
X the minor release over 'v0.22.0-alpha.1..HEAD' declares no compatibility decision
```

That is #307's guard working: `bump auto` deduces **minor** from the two features in the range, and a minor
or major release must carry the decision in a commit inside the range. Nothing was written — the dispatch
never reached a build, the smoke matrix, the attest job or `publish`.

This is why the first run was a dry run. The same refusal would otherwise have arrived on a real cut, after
the bump push, nineteen minutes into a run.

The rest of the graph was reachable: the job list showed `Smoke`, `Build`, `Attest the release gates` and
`Upload release assets` registered, so the gate channel this repository wired in #406 is part of the run.

One warning, unrelated to the refusal and left alone in this record:

```
! Input 'app-id' has been deprecated with message: Use 'client-id' instead.
```

## The compatibility decision

None of the range breaks a consumer, and the declaration lives in the commit that carries this file:

- Three flags were added (`publish --json`, `attest --json`, `init --json`, #408).
- One key was added to the attestation file (#412), and files written before it keep reading.
- One reported value was corrected in `init --update --json` (#413) — a flag that did not exist at
  `v0.22.0-alpha.1`, so an alpha reader cannot observe the change.

`git diff v0.22.0-alpha.1..main -- src/cli/args/` is 9 added lines and 4 in its tests, with **0 removed**;
no flag, option or configuration key is removed or renamed.

## Residual

The first real execution of this path is the cut itself. Everything above is a `dry_run=true` dispatch.
