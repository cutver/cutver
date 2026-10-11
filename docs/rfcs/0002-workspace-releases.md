# RFC 0002: Workspace Releases

> **Status**: Decided, not implemented. The maintainer accepted **(c) one version
> with path-filtered changelog sections** together with the schema-only package
> identity it needs; rejected **(a) per-package release units with namespaced
> tags** for now, with the reopen condition in §8; and dropped **(b)**. Nothing in
> this document is implemented — no code path and no config key changed.
> **Origin**: maintainer decision of 2026-10-11 — implement workspace support for
> real, RFC first. This record is the decision that RFC produced.
> **Provenance**: the workspace roadmap rows at `docs/internals/design.md:174-177`.
> A `[workspace]` table was documented as supported while the loader never read it;
> that sweep was PR #196, and #193 (merged) now makes unknown keys a hard error, so
> a repeat cannot hide. The independent workspace mode row is `[Deferred]` and links
> to this record.

---

## 1. Context and problem

Cutver resolves one version, one changelog and one tag per repository. In a
monorepo where packages ship independently, that forces every package to share a
version and to wait for every other package's release. There is no package
selector in the pipeline (`docs/internals/design.md:176`).

This is not a missing flag. It is the consequence of five assumptions encoded in
the pipeline, each load-bearing in a place a package selector would have to reach.

## 2. The single-release-unit assumptions

| # | Assumption | Where the code encodes it | Consequence in a monorepo |
| --- | --- | --- | --- |
| 1 | One global tag stream is the version baseline | `git::latest_tag(repo, Some(&config.git.tag_prefix))` (`src/bump/exec/pipeline.rs:146`); implementation `src/git/tags.rs:77-125`; tag is `"{prefix}{version}"` (`src/git/tags.rs:56`); one global prefix (`src/config/types.rs:258`, default `"v"` at `:280`) | Every package is versioned against every other package's tags |
| 2 | One commit per release | `paths_to_stage` is one merged vector (`src/bump/exec/manifest.rs:96-121`, `src/bump/exec/changelog.rs:146-166`, hook lockfiles `src/bump/exec/pipeline.rs:299-311`), staged once (`pipeline.rs:88`, `src/git/ops.rs:7`) and committed once (`pipeline.rs:92`); the clean-tree guard runs at entry (`pipeline.rs:39`, `src/git/status.rs:18`) | Cannot commit package A without package B's edits |
| 3 | The rollback boundary stops before the tag | `MutationTransaction` holds only file backups (`src/bump/exec/transaction.rs:5-11`), created `pipeline.rs:81`, committed `pipeline.rs:93`; the commit, tag, floating tag and publish sit outside (`pipeline.rs:92-104`) | A later package's failure cannot undo an earlier package's commit |
| 4 | The changelog is global and not path-aware | Range `{tag}..HEAD` (`src/git/log.rs:58-66,123-127`); no pathspec argument in `src/git/log.rs`; filtering is type/scope only (`src/changelog/context/enrich.rs:96-140`); one entry prepended to one path (`src/config/types.rs:187`, `changelog.rs:166`) | Every package's commits land in one changelog |
| 5 | Lifecycle, publish and CI fire once per tag | Preflight `pipeline.rs:63`; `post_bump` `pipeline.rs:272-311`; publish push and commands `pipeline.rs:103-104,329-363`; the plugin events `pipeline.rs:383-458`; CI keys its downstream jobs off one tag (`.github/workflows/release.yml:21-28,500`) | One publish and one CI run for the whole repo |

The crux behind all five: there is no notion of package identity. `Config.manifest`
is a flat `Vec<Manifest>` (`src/config/types.rs:54`) and `Manifest { path, primary, kind }`
(`:162-166`) has no id, name or group; validation rejects duplicate paths and a
missing current source (`src/config/validation.rs:36-47`). Version resolution
picks one source (`:6-33`).

## 3. Alternatives

| Model | Delivers | Breaks | Cost | Cannot express | Verdict |
| --- | --- | --- | --- | --- | --- |
| **0. Baseline** (today) | One version, tag, changelog, commit, publish per repo | — | — | Any independent package release | — (baseline) |
| **a. Per-package release unit, namespaced tags** (`core/v1.2.0`) | A real release boundary: its own baseline, tag, changelog, commit and publish | 1, 2, 3 (scope), 4, 5 | Package identity in the schema; per-package tag lookup, changelog range, staging set and CI fan-out; **plus the dependency cascade, unaddressed** | An atomic all-package release with one shared version | **Rejected for now** (§5) |
| **b. Per-package versions, one release and tag** | Independent version numbers, one commit and one tag | 1 (tag no longer equals a package version); 4 still open | One tag number cannot baseline N packages; consumers cannot resolve a package from its tag | Releasing one package without touching the others; a per-package tag | **Dropped** — cannot serve as the fallback |
| **c. One version, per-package changelogs** | Path-filtered changelog sections per package | 4 (changelog gains a pathspec) | A commit-to-package path map and a changelog template | Independent versioning or cadence; every package still bumps together | **Accepted** (§7) |
| **d. Split into per-package repositories** | Independent everything, with zero cutver change | nothing in code — leaves the tool | N repositories, N CI configs, duplicated releases | A monorepo at all | **Named as today's honest answer** (§9) |

Models 0–c all keep one release unit for the repository as a whole. Only (a) makes
the package itself the release unit, which is why only (a) can answer "release
`core` only". Model (b) gives packages independent version numbers but still
releases them together under one tag and commit.

## 4. Decision

**Adopt (c): one shared version with path-filtered per-package changelog
sections, plus the schema-only package identity it requires.** Reject (a)
per-package release units with namespaced tags for now, with the reopen condition
in §8. Drop (b): it breaks the tag's meaning without delivering independence, and
a half-measure that breaks a contract and delivers nothing is worse than the
baseline. (d) is the honest answer today for anyone who genuinely needs
independent releases. This record implements none of it.

## 5. Why (a) is rejected

**1. The dependency cascade is an unaddressed prerequisite, and without it (a) is
unsafe.** `docs/internals/design.md:177` lists "Internal workspace dependency
cascade" as `[Planned]`; the original proposal never mentioned it. In a layered
monorepo, bumping `core` forces `cli`'s constraint to change, which forces `cli`
to be re-released: independence exists only between packages with **no dependency
edge**. Implementing the cascade means parsing and rewriting each ecosystem's
dependency syntax (Cargo, `package.json`, `pyproject`) — a new manifest editor per
ecosystem, i.e. a capability, not a config key. Editors are already dispatched
per manifest kind (`src/manifest/mod.rs:49-109`), so this is a new editor each,
not a switch. And (a) without the cascade degrades today's safety: today every
package bumps together, so a stale intra-repo constraint is impossible by
construction (assumptions 1 and 2); with (a) it becomes possible and silent.

**2. No demonstrated demand.** cutver's own repository is one crate plus
`crates/cutver-pdk` (`Cargo.toml:12-14`); this feature is for hypothetical users.
The risk is the mirror image of the original defect: aspiration designed as a
roadmap instead of aspiration documented as reality.

**3. (b) cannot serve as the fallback.** The alternatives table already gives the
reason: one tag number cannot baseline N packages, so (b) buys independent version
numbers while breaking the tag's meaning, and it still releases everything
together.

**4. (a) is a different product category.** Cutver's identity is one product, many
manifests: the origin is a single application across `package.json` +
`Cargo.toml` + `tauri.conf.json` (`docs/guides/monorepos.md:41-51`), and the core
principle is lockstep across ecosystems (`docs/internals/design.md:21-22`).
Independent per-package releases turn cutver into a release *orchestrator* — a
strategic change, not a feature.

## 6. Why this is a product decision and not a technical one

The clean-tree guard (`pipeline.rs:39`, `src/git/status.rs:18`) and the rollback
boundary (`transaction.rs:5-11`, `pipeline.rs:81-93`) are the product's safety
story: two-phase mutation, a `--dry-run` that simulates without writing, and an
atomic rollback of file edits **before the release commit**. The tag and the
publish are reported, not undone (`pipeline.rs:92-104`).

Model (a) changes that meaning. In one run that releases N packages, "clean" would
have to mean "clean except the other packages' pending changes"; a rollback that
stops before the first commit cannot undo a commit already made for the first
package when the second fails; and `--dry-run` would simulate N releases, not one.
Choosing (a) would therefore require the maintainer to decide whether the release
unit, not the repository, becomes the transaction boundary. **Because (a) is
rejected, that transaction-boundary decision is not being taken.** The analysis
stays because it is why the rejection is a strategy call rather than a
configuration addition.

## 7. What is accepted: identity plus path-filtered changelog sections

The accepted scope is the identity key and path-aware changelogs only. It does not
include per-package versions, tags, staging, commits, publish or CI.

A per-package changelog section is produced by passing the package's path to the
existing log range: `git log <range> -- <path>`. Each package's commits are grouped
under the current global version in the one changelog. That is the value for
exactly the users cutver targets: today every package's commits land in one
undifferentiated changelog (assumption 4).

Identity is needed because nothing in the schema names a package (assumption
crux). It is proposed on `[[manifest]]` as `package = "core"`, not in a
`[workspace]` table and not implied by directory: `Config.manifest` is already the
only per-package list (`src/config/types.rs:54`), and directory-implied identity is
ambiguous when one package owns two manifests.

```toml
# PROPOSED — not implemented, not read by any loader in this tree.
[[manifest]]
path = "crates/core/Cargo.toml"
package = "core"            # proposed identity anchor (new key)
primary = true
```

**Proposed, not implemented.** Since #193, unknown keys are a hard error naming the
table and the key (`src/config/validation.rs:52-75`), so every key here is a
deliberate schema addition. The configuration reference stays canonical only for
shipped keys, and the design convention bars a `[Planned]` item from appearing as a
key table (`docs/internals/design.md:11-12`); nothing enters the
[configuration reference](../reference/configuration.md) until the key exists.

## 8. What would reopen (a)

This rejection is revisited **only** if a real user with a real dependency graph
asks to release one package without the others. If that happens, the work **starts
from the dependency cascade, never from the tags** — the cascade is the
prerequisite that makes (a) safe, and tags are meaningless until it exists.

## 9. Non-goals and residual

- This record implements nothing; no code path or config key changed here.
- The remaining `[Planned]` roadmap items are unaffected: `glob` in `[[manifest]]`,
  Cargo `[workspace.package]` inheritance and the dependency cascade stay as they
  are (`docs/internals/design.md:174-175,177`).
- The honest answer today for a user who needs independent releases is (d): split
  into per-package repositories, or use another tool.
- `docs/guides/monorepos.md:11` still calls independent releases "Phase 2 work";
  that sentence predates this record and points at the design document, not here.
- Not verified, because no implementation exists: whether a per-package tag stream
  can coexist with the floating-major-tag lookup (`src/git/tags.rs:107`), and the
  exact CI fan-out. No behaviour was exercised.
