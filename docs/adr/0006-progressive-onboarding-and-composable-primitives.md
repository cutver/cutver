# ADR-0006: Disclose the surface progressively, and keep the CI file format out of the core

**Status**: proposed
**Date**: 2026-11-26
**Owner**: repository maintainer
**Review date**: 2027-11-26 (proposal; not yet committed by the maintainer)
**Implementation**: not started. This record revises the draft circulated on 2026-11-26: it keeps
progressive disclosure, moves the CI-file work out of the core, and puts the missing machine surface
([#404](https://github.com/cutver/cutver/issues/404)) before the new subcommands. The order is fixed by
rule 4: #404 first, then the honesty pass, then rule 1's scaffold, then rule 2's mutators, then the
adapter. `init` gains `--dry-run` and `--json` as part of rule 1 — it has neither today — so a user can
see the plan before a file is written.

## Context

Cutver is a release orchestrator whose primitives exist for hard pipelines: a two-phase release
(`--defer-publish` then `publish`), commit-bound gate attestations (`cutver attest`), capability
sandboxed plugins and a prerelease ladder. A library that wants a SemVer bump and a `CHANGELOG.md`
entry needs none of them, and today the first file it is handed describes all of it: the reference
spends 141 lines on eleven configuration tables (`docs/reference/configuration-tables.md`).

**What the full surface costs, measured in this repository.** PR
[#406](https://github.com/cutver/cutver/pull/406) wired one two-gate release by hand: 6 lines of
`cutver.toml`, 59 of `.github/workflows/release.yml`, 4 of `.gitignore`, plus an update to the
workflow's own contract test. Two defects surfaced only under verification — `actions/upload-artifact`
skips hidden paths, so `.cutver/` needed `include-hidden-files`, and
`tests/release_pipeline_contract.rs:20-32` pins the publish job's exact `needs:` line, so a new job has
to be declared there too or `cargo test` fails as the bump preflight and no release starts at all.
Neither is something a user can be expected to know.

**Drift is real, and it lands late.** `guard::verify_gates` has one caller, `run_publish`
(`src/bump/exec/publish.rs:179`). `bump --defer-publish` never consults it, so a `[gates] require`
without the matching `cutver attest` step is not noticed during the bump: the refusal arrives in the
publish job, after the bump branch was pushed.

**The surface also teaches keys that do nothing.** `[project] name` is parsed and never read —
`project_name` comes from the repository name (`src/changelog/context/assemble.rs:195`).
`[version] strategy` is read only when it equals `"plugin"` (`src/bump/plan/resolve.rs:134`).
`[plugins.<name>.config]` is parsed and the host does not deliver it
([#317](https://github.com/cutver/cutver/issues/317)). Under a full changelog template, four more keys
are inert (`src/changelog/render/body.rs:105-139`). A smaller surface that still lists these teaches
the wrong lesson.

**The commands a composable onboarding would build on have no machine surface.** `publish`, `attest`
and `init` reject `--json` as a usage error, and their failures are never enveloped
(`docs/internals/machine-contract.md:40-43`, `:104-106`). Verified against the published
`0.22.0-alpha.1` binary, not only the documents.

**The scope boundary decides the shape of the answer.** `CONTRACT.md:222`: *GitHub is a channel, never
an identity — every integration with a code host degrades to stdio and exit codes, and a CI integration
is a consumer of the machine contract rather than a dependency of the core.* The core also carries no
YAML parser (`Cargo.toml`); this repository's own tests read its workflow by scanning text
(`tests/release_pipeline_contract.rs:34-46`).

**And a static workflow check would misread the flagship workflow.** `release.yml:167-186` assembles
`args=(--defer-publish)` and appends to it conditionally, so a matcher looking for "bump without
`--defer-publish`" cannot decide that file. The most advanced user of the feature is the counterexample.

## Decision

Progressive disclosure is adopted, and it is split along the boundary above. Four rules.

1. **The scaffold starts smaller and grows by subcommand.** `cutver init` writes the minimal
   `cutver.toml`: `[project]`, the detected `[[manifest]]` entries, `[changelog]`, and `[git]`'s
   release guards. Nothing else is emitted until asked for. An existing `cutver.toml` is refused rather
   than overwritten (`src/init/scaffold/runner.rs:16,137` already names `--update` and `--force`), and
   the command gains `--dry-run` and `--json` — neither exists today — so the plan is readable before a
   file is written and the result is readable after. This resolves the direction that
   [#360](https://github.com/cutver/cutver/issues/360) raised: the generator covers the shipped surface
   through the mutators rather than through one maximal file, and the mutators read the same schema the
   loader does.
2. **Mutators for `cutver.toml` are core; the CI file is not.** `cutver gate add|remove|list` edits
   `[gates] require` surgically — comments, key order and formatting preserved, the standard
   `CONTRACT.md:72` already requires of manifest edits — and prints a channel-neutral next-steps block:
   which command goes where, in what order, and what the tool will not do for you. The GitHub-shaped
   YAML is emitted by a `cutver-gh` binary resolved through the dispatch that already exists
   (`src/cli/runner/external.rs:8,41`), or shipped as a template a user can read. The core prints no
   vendor's file format.
3. **`cutver doctor` does not read CI files.** The drift check has a home, not a veto: `cutver-gh doctor`
   may parse `.github/workflows/*.yml` precisely because it is not the core. Putting that check in the
   binary is available, and it is a boundary crossing under `CONTRACT.md:222` — a maintainer decision
   recorded in the pull request and the changelog, never an implementation detail.
4. **Honesty precedes disclosure, and machine output precedes automation.** Before a smaller surface is
   taught, the keys that do nothing are marked or removed: `[project] name`, `[version] strategy`'s two
   dead values, `[plugins.<name>.config]` ([#317](https://github.com/cutver/cutver/issues/317)). And
   every new mutator ships with `--json` and an enveloped failure, because a primitive nobody can read
   is not composable — which means [#404](https://github.com/cutver/cutver/issues/404) lands first.

## Alternatives

| Option | Trade-off | Verdict |
| :--- | :--- | :--- |
| **Null case** — document the surface better and change no code | No new dependency, no new command; the measured wiring cost of #406 and the late gate refusal both stay | **rejected** — the cost lands on whoever wires gates by hand, and the failure arrives after the branch push |
| **(a)** The draft: `init` scaffolds the workflow, `gate` emits GitHub YAML, `doctor` parses it | One artifact, one story, no adapter to discover | **rejected** — crosses `CONTRACT.md:222`, adds the core's first YAML dependency, and the check cannot read this repository's own workflow (`release.yml:167-186`) |
| **(b)** Move onboarding out of the core entirely, as an adapter | Core stays channel-free | **rejected** — mutating `cutver.toml` is the core's own contract, and a release tool that cannot scaffold itself has not onboarded anyone |
| **(c)** The maximal scaffold alone, as #360 asks | One file, no subcommands | **rejected** — it hands a first project gates, plugins and channels it did not ask for, which is the over-granting half of #360 itself |
| **(d)** This record: minimal scaffold, core mutators, adapter for the CI file | Two artifacts to ship, and the adapter must be discoverable | **accepted** |

## Consequences

- **Easier**: a first release needs `[project]`, one `[[manifest]]` and `[changelog]`, and the path to
  gates, plugins and channels is a subcommand rather than a paragraph of prose to transcribe.
- **Easier**: drift acquires a check that can see a stale snippet, because the component that emits YAML
  is the component that reads it — instead of a core check that must guess which file a user meant.
- **Harder**: two artifacts to release, test and document, and `cutver-gh` is undiscoverable unless the
  tool names it where a user looks: the next-steps block of `gate add` and a row in the guides.
- **Harder**: a snippet printed once goes stale. The adapter has to be able to re-render and compare, and
  that re-render is the check, which is why rules 2 and 3 are one decision and not two.
- **Downside**: three writers touch files the user owns — `cutver init` writes `cutver.toml`,
  `cutver gate add` mutates it, and `cutver-gh` writes `.github/workflows/release.yml`. Deterministic
  TOML editing that preserves comments, key order and formatting; refusal to overwrite an existing
  scaffold or workflow without an explicit flag; and `--dry-run` on every one of them are requirements
  of the implementation, not polish.
- **Kept honestly**: this record is the same idea as the draft with its boundary corrected. The draft's
  trade-off section named the cheap cost (YAML inspection complexity) and not the expensive one
  (the core taking on a vendor's file format), which is the single defect this revision exists to fix.

## Residual

- Not verified: whether `cutver plugin add` prints CI steps today. The draft asserts it does; the claim
  is treated as a proposal until the command is read.
- Not verified: that a lean binary is the right thing to ship `attest` from — PR #406 attests with the
  lean linux artifact, which follows from source, not from a run.
- Dropped from the draft: the "under two minutes, fewer than 15 lines of TOML and 35 of YAML" figures.
  Nothing measured them, and this repository does not keep a claim nobody ran.
- Not decided here: `cutver workflow add-channels` mutating YAML is rule 2's subject, not an exception —
  it belongs to the adapter for the same reason `gate add` belongs to the core.

## Reopen condition

This reopens with evidence that the split costs more than it saves — a report of someone running
`cutver gate add`, following the printed steps, and still shipping a drifted configuration — or with a
maintainer decision to put CI parsing in the core, recorded per `CONTRACT.md:222` in the pull request and
the changelog. It does not reopen on the observation that two artifacts are more work than one, which is
already recorded above.

## Traceability

- Raised by: [#360](https://github.com/cutver/cutver/issues/360) — the generated `cutver.toml` neither
  covers the shipped surface nor delimits scope.
- Blocked on: [#404](https://github.com/cutver/cutver/issues/404) — `publish`, `attest` and `init` have
  no machine surface, which rule 4 requires before either phase ships.
- Depends on: [#317](https://github.com/cutver/cutver/issues/317) — what `[plugins.<name>.config]` is
  for, which rule 4 must settle before the surface can claim honesty.
- Boundary: [`CONTRACT.md` §VII.1](../../CONTRACT.md), *What `cutver` May Become*.
- Cost evidence: PR [#406](https://github.com/cutver/cutver/pull/406) and its two verification findings.
- Related: [#326](https://github.com/cutver/cutver/issues/326), [#166](https://github.com/cutver/cutver/issues/166).
