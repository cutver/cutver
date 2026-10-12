# Feature: Workspace Support RFC

Status: decision recorded
Branch: `docs/workspaces-rfc`
Deliverable: `docs/rfcs/0002-workspace-releases.md` — a **decision record**, not a proposal
Predecessor: `fix/reject-unknown-config-keys` merged as `11ce2aa` (closes #193)

## Outcome

The maintainer decision of 2026-10-11 was taken and is recorded in the RFC:

- **(c) accepted** — one shared version with path-filtered per-package changelog
  sections, together with the schema-only package identity it requires. This is
  the work to do next.
- **(a) rejected for now** — per-package release units with namespaced tags, with
  the reopen condition below. The RFC's previous "adopt (a) as the target"
  recommendation is reversed.
- **(b) dropped** — per-package versions under one release and tag. It breaks the
  tag's meaning without delivering independence; it is not the fallback.
- **(d) named as today's honest answer** — split into per-package repositories, or
  use another tool.

Recorded reasons for rejecting (a), in weight order: the dependency cascade is an
unaddressed prerequisite (`docs/internals/design.md:177`); no demonstrated demand;
(b) cannot serve as the fallback; (a) is a different product category — a release
*orchestrator* rather than one product across many manifests.

**Reopen condition:** this rejection is revisited only if a real user with a real
dependency graph asks to release one package without the others. Work then starts
from the dependency cascade, never from the tags.

The deliverable changed from "recommend a model" to "record a decided model": a
reasoned, written rejection with a reopen condition is what stops the next reader
from documenting `[workspace]` as if it existed.

## Goal

Design workspace support for real, in an RFC, before any of it exists. `[workspace]
mode/members` was documented as a supported feature for three verification rounds
while the loader never read it; it was removed from the documentation and is marked
`[Planned]` in `docs/internals/design.md`. The user decided to implement it, and to
let the RFC analyse the alternatives and recommend one rather than deciding the
shape up front. The maintainer then rejected the RFC's recommendation, so the
feature outcome is the recorded decision plus the accepted narrow slice.

## Why now, and why it is safe to write

#193 landed: unknown `cutver.toml` keys are now a hard error naming the table and
the key. A half-built `[workspace]` can no longer be mistaken for a working one,
which is precisely how the previous phantom survived. The documentation rule
stands: nothing enters `configuration.md` or the README until the feature exists.

## Why it is the hardest feature in this domain

Today the tool assumes **one release unit per repository**. That assumption is
load-bearing in places that are not obvious, and the RFC names each one:

- the clean-tree requirement and the selective staging set;
- the `MutationTransaction` rollback boundary;
- `--dry-run`'s meaning ("simulate the release" is unambiguous only when there is
  one release);
- tag creation, `tag_prefix`, the floating major tag, and `git::latest_tag` as the
  version baseline;
- the changelog: which commits belong to which release;
- preflight, `post_bump`, `[publish]` and the plugin lifecycle events, all of which
  fire once;
- what CI does with tags (`.github/workflows/release.yml`, `cutver/release@v1`).

## Tasks

- [x] 1. Evidence map (read-only): for each assumption above, the `path:line` that encodes it, so the RFC argues from the code rather than from what a monorepo tool usually does.
- [x] 2. Alternatives analysis: the release-unit models with their consequences on the assumptions in task 1, plus the current single-unit behaviour as the baseline.
- [x] 3. Decision record: the accepted model (c), the rejected model (a) with its four reasons and reopen condition, the dropped model (b) and the honest answer (d), with the evidence.
- [x] 4. The accepted configuration design: how a package is identified, and how path-filtered changelog sections are produced, labelled as proposed and not implemented.
- [ ] 5. Follow-up implementation (separate feature): schema-only package identity and path-filtered per-package changelog sections.
- [ ] 6. Sweep `docs/guides/monorepos.md:11`, which still calls independent releases "Phase 2 work" and points at the design document rather than the decision record.

## Non-goals

- Implementing any of the accepted slice in this feature. This feature produces the decision record.
- The plugin artifact cache/proxy RFC, which is the unit after this one.
- The `[hooks] pre_bump` (#194) and `--help` string (#195) fixes.

## Residual

- Until the accepted slice is implemented, `[workspace]`, `glob` in `[[manifest]]`,
  Cargo workspace inheritance and the dependency cascade stay `[Planned]`; the
  independent workspace mode row is now `[Deferred]` and links to the decision
  record.
- `docs/guides/monorepos.md:11` still carries the stale "Phase 2 work" sentence;
  task 6 tracks it.
