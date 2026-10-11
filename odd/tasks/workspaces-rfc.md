# Feature: Workspace Support RFC

Status: in progress
Branch: `docs/workspaces-rfc`
Deliverable: `docs/rfcs/0002-workspace-releases.md`
Predecessor: `fix/reject-unknown-config-keys` merged as `11ce2aa` (closes #193)

## Goal

Design workspace support for real, in an RFC, before any of it exists. `[workspace] mode/members` was
documented as a supported feature for three verification rounds while the loader never read it; it was
removed from the documentation and is marked `[Planned]` in `docs/internals/design.md`. The user has
decided to implement it, and to let the RFC analyse the alternatives and recommend one rather than
deciding the shape up front.

**Decision carried from the user:** implement it for real, RFC first. The RFC must present the release
unit alternatives with their consequences and recommend one.

## Why now, and why it is safe to write

#193 landed: unknown `cutver.toml` keys are now a hard error naming the table and the key. A
half-built `[workspace]` can no longer be mistaken for a working one, which is precisely how the
previous phantom survived. The documentation rule stands: nothing enters `configuration.md` or the
README until the feature exists.

## Why it is the hardest feature in this domain

Today the tool assumes **one release unit per repository**. That assumption is load-bearing in places
that are not obvious, and the RFC must name each one:

- the clean-tree requirement and the selective staging set;
- the `MutationTransaction` rollback boundary;
- `--dry-run`'s meaning ("simulate the release" is unambiguous only when there is one release);
- tag creation, `tag_prefix`, the floating major tag, and `git::latest_tag` as the version baseline;
- the changelog: which commits belong to which release;
- preflight, `post_bump`, `[publish]` and the plugin lifecycle events, all of which fire once;
- what CI does with tags (`.github/workflows/release.yml`, `cutver/release@v1`).

## Tasks

- [ ] 1. Evidence map (read-only): for each assumption above, the `path:line` that encodes it, so the RFC argues from the code rather than from what a monorepo tool usually does.
- [ ] 2. Alternatives analysis: at least three release-unit models with their consequences on the assumptions in task 1 (per-package release unit with namespaced tags; per-package versions under one release; one version with per-package changelogs), plus the current single-unit behaviour as the baseline. Each alternative states what it breaks, what it costs, and what it cannot express.
- [ ] 3. Recommendation with the evidence, and the smallest incremental slice that delivers it.
- [ ] 4. The configuration design: how a package is identified, what `[workspace]` or its successor looks like, how `--package` interacts with `require_branch`, `--dry-run` and the clean-tree rule, and how `glob` in `[[manifest]]` and Cargo `[workspace.package]` awareness fit (both currently `[Planned]`).
- [ ] 5. Compatibility and migration: existing configs must not break, `deny_unknown_fields` means every new key is a deliberate schema addition, and the RFC states the versioning impact.
- [ ] 6. Verification: the RFC's claims against the code, links resolve, budget respected, and the alternatives table cannot be read as an implementation.
- [ ] 7. Open the pull request.

## Non-goals

- Implementing any of it. This feature produces the RFC.
- The plugin artifact cache/proxy RFC, which is the unit after this one.
- The `[hooks] pre_bump` (#194) and `--help` string (#195) fixes.

## Residual

- Until the RFC is accepted and implemented, `[workspace]`, `glob` in `[[manifest]]` and Cargo
  workspace inheritance stay `[Planned]`.
- If the recommendation requires a behaviour change to the single-unit path, that is a separate
  decision and must come back to the user explicitly.
