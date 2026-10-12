# ADR-0002: Use one shared version with path-filtered per-package changelog sections for workspace releases

**Status**: accepted
**Date**: 2026-10-11
**Owner**: repository maintainer
**Review date**: 2027-10-11 (proposal; not yet committed by the maintainer)
**Implementation**: not-started
**Supersedes**: none

## Context

Cutver resolves one version, one changelog and one tag per repository. In a monorepo whose packages would ship independently, that forces lockstep. The limitation is not a missing flag: the pipeline encodes five load-bearing release-unit assumptions.

1. One global tag stream is the version baseline — `src/git/tags.rs:77-125`; tag name is `"{prefix}{version}"` (`src/git/tags.rs:56`); one global prefix (`src/config/types.rs:257-258`, default `"v"` at `:280`).
2. One commit per release — the staged set is merged and staged once, then committed once (`src/bump/exec/pipeline.rs:88,92`).
3. The rollback boundary stops before the tag — `MutationTransaction` holds only file backups (`src/bump/exec/transaction.rs:5-10`).
4. The changelog range is not path-aware — `{tag}..HEAD` is built in `raw_commits_between` (`src/git/log.rs:11-20`) with no pathspec, and one changelog path is configured (`src/config/types.rs:187-188`).
5. Publish and lifecycle fire once per tag — publish push and commands (`src/bump/exec/pipeline.rs:103-104`).

The crux behind all five: there is no package identity. `Config.manifest` is a flat `Vec<Manifest>` (`src/config/types.rs:54`) and `Manifest { path, primary, kind }` (`src/config/types.rs:162-167`) has no id, name or group; validation rejects duplicate paths and a missing current source (`src/config/validation.rs:36-47`).

Prior art sets the honesty frame: the dependency cascade is **not** unsolved in general — Nx ships "Update dependents", Changesets keeps interdependent packages in sync, and cargo-release offers `--dependent-version` (RFC §3). It is unsolved *for cutver*, where each ecosystem is a new manifest editor (`src/manifest/mod.rs:49-111`). Per-package release units are therefore a scope-and-demand question, not an impossibility.

## Decision

We will adopt option (c): one shared version with path-filtered per-package changelog sections, plus the schema-only package identity it requires. We will reject option (a) for now under the reopen condition below, drop option (b), and name option (d) as today's honest answer for anyone who needs independent releases.

### Implementation status (separate axis: not-started)

The `package` key does not exist in the configuration schema: the closed `MANIFEST_KEYS` allowlist is `path`, `primary`, `kind`, `field`, … (`src/config/validation.rs:68-74`), and `grep '"package"' src/config` returns nothing. No path filtering exists in the changelog's commit range: `src/git/log.rs:11-20` pushes only `log`, the range and `--format`, with no `-- <path>` pathspec.

## Alternatives

| Option | Trade-off | Verdict |
| :--- | :--- | :--- |
| **Null case** — do nothing, keep one release unit per repository | Zero schema and pipeline change; the undifferentiated changelog and missing package identity stay | **rejected** — the identity-plus-changelog slice is small and answers the concrete complaint; reopen if it proves not worth its schema cost |
| **(a)** Per-package release unit with namespaced tags | A real per-package boundary — its own baseline, tag, changelog, commit and publish; adds per-package tag lookup, changelog range, staging set, CI fan-out, **and the unaddressed dependency cascade** | **rejected for now** — the four reasons below and the reopen condition |
| **(b)** Per-package versions under one release and tag | Independent version numbers; one tag number cannot baseline N packages, so the tag no longer names a package version | **dropped** — breaks the tag's meaning without delivering independence |
| **(c)** One shared version with path-filtered per-package changelog sections | Each package's commits grouped under the shared version; every package still bumps together, and a commit-to-package path map and changelog template are added | **accepted** |
| **(d)** Split into per-package repositories | Independent everything with zero cutver change; N repositories, N CI configs, duplicated releases | **named as today's answer** |

Four recorded reasons to reject (a): the dependency cascade is an unaddressed prerequisite and (a) without it is unsafe; no demonstrated demand; (b) cannot serve as the fallback; (a) turns cutver into a release *orchestrator*, a different product category.

## Consequences

- Easier: each package's commits group under the shared version, and packages gain a nameable schema identity.
- Harder: the accepted scope adds a schema key and a changelog template without delivering independent release cadence — every package still bumps together under one tag, one commit and one publish.
- Downside: the path map is a new commit-to-package mapping that can drift from reality.
- Downside: packages that today cannot go stale together will still not be independently releasable after this change.
- Downside: a package identity no other code path consumes is schema weight with no payoff beyond the changelog section; unknown keys are a hard error (`src/config/validation.rs:123-128`), so the key is a deliberate permanent addition.
- Kept honestly: choosing (c) defers the cascade as out of scope for now; it does not claim the cascade is impossible (Context above).

## Reopen condition

The rejection of (a) is revisited **only** if a real user with a real dependency graph asks to release one package without the others. If that happens, the work **starts from the dependency cascade, never from the tags**.

The null case reopens if the identity-plus-changelog slice proves not worth its schema cost.

## Traceability

- Proposal: [docs/rfcs/0002-workspace-releases.md](../rfcs/0002-workspace-releases.md)
- Sibling decision: [ADR-0001: Adopt the dual-runtime plugin microkernel](../adr/0001-dual-runtime-plugin-microkernel.md), proposed by [RFC 0001](../rfcs/0001-wasm-plugin-architecture.md).
- Roadmap rows: [docs/internals/design.md:174-177](../internals/design.md)
- Guide: [docs/guides/monorepos.md:11](../guides/monorepos.md)
- No issue or pull request number is recorded for this decision.
