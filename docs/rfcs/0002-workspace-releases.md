# RFC 0002: Workspace Releases

> **Status**: Proposed — analysis only. Nothing in this document is implemented.
> **Origin**: maintainer decision of 2026-10-11 — implement workspace support for real, RFC first, with the alternatives analysed and one recommended. No tracking issue exists yet; open one if this RFC is accepted.
> **Context**: the `[Planned]` rows at `docs/internals/design.md:174-176`. A `[workspace]` table was documented as supported while the loader never read it; that sweep was PR #196, and #193 (merged) now makes unknown keys a hard error, so a repeat cannot hide. This RFC exists to decide the shape before anything is written.

---

## 1. Context and problem

Cutver resolves one version, one changelog and one tag per repository. In a
monorepo where packages ship independently, that forces every package to share
a version and to wait for every other package's release. There is no package
selector in the pipeline ([design notes](../internals/design.md):176).

This is not a missing flag. It is the consequence of five assumptions encoded
in the pipeline, each of which is load-bearing in a place a package selector
would have to reach.

## 2. The single-release-unit assumptions

| # | Assumption | Where the code encodes it | Consequence in a monorepo |
| --- | --- | --- | --- |
| 1 | One global tag stream is the version baseline | `git::latest_tag(repo, Some(&config.git.tag_prefix))` (`src/bump/exec/pipeline.rs:146`); implementation `src/git/tags.rs:77-125`; tag is `"{prefix}{version}"` (`src/git/tags.rs:56`); one global prefix (`src/config/types.rs:258`, default `"v"` at `:280`) | Every package is versioned against every other package's tags |
| 2 | One commit per release | `paths_to_stage` is one merged vector (`src/bump/exec/manifest.rs:96-121`, `src/bump/exec/changelog.rs:146-166`, hook lockfiles `src/bump/exec/pipeline.rs:299-311`), staged once (`pipeline.rs:88`, `src/git/ops.rs:7`) and committed once (`pipeline.rs:92`); the clean-tree guard runs at entry (`pipeline.rs:39`, `src/git/status.rs:18`) | Cannot commit package A without package B's edits |
| 3 | The rollback boundary stops before the tag | `MutationTransaction` holds only file backups (`src/bump/exec/transaction.rs:5-11`), created `pipeline.rs:81`, committed `pipeline.rs:93`; the commit, tag, floating tag and publish sit outside (`pipeline.rs:92-104`) | A later package's failure cannot undo an earlier package's commit |
| 4 | The changelog is global and not path-aware | Range `{tag}..HEAD` (`src/git/log.rs:62-69`, `:124-127`); no pathspec argument in `src/git/log.rs`; filtering is type/scope only (`src/changelog/context/enrich.rs:95-142`); one entry prepended to one path (`src/config/types.rs:187`, `changelog.rs:166`) | Every package's commits land in one changelog |
| 5 | Lifecycle, publish and CI fire once per tag | Preflight `pipeline.rs:63`; `post_bump` `pipeline.rs:272-311`; publish push and commands `pipeline.rs:103-104,329-363`; the plugin events `pipeline.rs:383-459`; CI keys every downstream job off one tag (`.github/workflows/release.yml:21-28,500`) | One publish and one CI run for the whole repo |

The crux behind all five: there is **no notion of package identity**. `Config.manifest`
is a flat `Vec<Manifest>` (`src/config/types.rs:54`) and `Manifest { path, primary, kind }`
(`:162-166`) has no id, name or group; validation rejects duplicate paths only
(`src/config/validation.rs:36-46`). Version resolution picks one source (`:6-33`).

## 3. Alternatives

| Model | Delivers | Breaks | Cost | Cannot express |
| --- | --- | --- | --- | --- |
| **0. Baseline** (today) | One version, tag, changelog, commit, publish per repo | — | — | Any independent package release |
| **a. Per-package release unit, namespaced tags** (`core/v1.2.0`) | A real release boundary: its own baseline, tag, changelog, commit and publish | 1, 2, 3 (scope), 4, 5 | Package identity in the schema; per-package tag lookup, changelog range, staging set and CI fan-out | An atomic all-package release with one shared version |
| **b. Per-package versions, one release and tag** | Independent version numbers, one commit and one tag | 1 (tag no longer equals a package version); 4 still open | One tag number cannot baseline N packages; consumers cannot resolve a package from its tag | Releasing one package without touching the others; a per-package tag |
| **c. One version, per-package changelogs** | Path-filtered changelog sections per package | 4 (changelog gains a pathspec) | A commit-to-package path map and a changelog template | Independent versioning or cadence; every package still bumps together |
| **d. Split into per-package repositories** | Independent everything, with zero cutver change | nothing in code — leaves the tool | N repositories, N CI configs, duplicated releases | A monorepo at all |

Models 0–c all keep one release unit for the repository as a whole. Only (a)
makes the package itself the release unit, which is why only (a) can answer
"release `core` only". Model (b) gives packages independent version numbers but
still releases them together under one tag and commit.

## 4. Recommendation

**Adopt (a): per-package release units with namespaced tags**, as the target the
incremental slices build toward. The argument is the assumptions table: (b) and
(c) keep a single release unit, so (c) leaves 2, 3 and 5 untouched — every
package still releases together. Only (a) gives each package its own tag stream
(1), its own staging and commit (2), its own changelog range (4) and its own
publish and CI trigger (5). This is a decision that can be disagreed with: the
price of (a) is that the safety story changes, and §5 says that is the
maintainer's call, not a config detail.

## 5. The open product question: the safety story

The clean-tree guard (`pipeline.rs:39`) and the rollback boundary
(`transaction.rs:5-11`, `pipeline.rs:81-93`) are the product's safety story:
two-phase mutation, a `--dry-run` that simulates without writing, and an atomic
rollback of file edits **before the release commit**. The tag and the publish
are reported, not undone (`pipeline.rs:92-104`).

Models (b) and (c) leave that story intact: one tag, one commit, one transaction.
Model (a) changes its meaning. In a single run that releases N packages, "clean"
would have to mean "clean except the other packages' pending changes"; a rollback
that stops before the first commit cannot un-do a commit already made for the
first package when the second fails; and `--dry-run` would simulate N releases,
not one. Choosing (a) therefore requires the maintainer to decide whether the
release unit, not the repository, becomes the transaction boundary. That is a
product decision about the safety guarantee, not a configuration addition.

If the maintainer declines it, the endpoint is models (c) and (b) combined —
per-package changelog sections and per-package version numbers under one release
unit — so slices 3 and 4 in §7 are never built and a per-package release cadence
is not delivered at all.

## 6. Proposed configuration shape

> **Proposed, not implemented.** None of the keys below is read by any loader in
> this tree. Since #193, unknown keys are a hard error naming the table and the
> key (`src/config/validation.rs:52-75`), so every key here is a deliberate
> schema addition.

Package identity is proposed **on `[[manifest]]`**, not in a `[workspace]` table
and not implied by directory. `Config.manifest` is already the only per-package
list (`src/config/types.rs:54`); a `[workspace]` table with path globs would add a
second source of truth, and `glob` is separately `[Planned]` (`design.md`:174).
Directory-implied identity is ambiguous when one package owns two manifests.

```toml
# PROPOSED — not implemented, not read by any loader in this tree.
[project]
name = "acme"               # parsed today, unused pipeline-side (src/config/types.rs:72-74)

[[manifest]]
path = "crates/core/Cargo.toml"
package = "core"            # proposed identity anchor (new key)
primary = true

[[manifest]]
path = "crates/cli/Cargo.toml"
package = "cli"

# Proposed selector: cutver bump minor --package core
```

The selector would scope the release to every manifest whose `package` matches.
`--dry-run` simulates only that unit. `require_branch` is repo-wide
(`src/git/status.rs:38`) and would stay repo-wide. The clean-tree rule's scope is
the §5 decision, not settled here. The tag becomes `{tag_prefix}{package}/v{version}`:
`TagPrefix` already accepts a `core/v` shape (`src/git/types.rs:61-69`), while
`latest_tag` would need a per-package match (`src/git/tags.rs:77`). `ManifestPath`
already accepts nested relative paths (`src/config/path.rs:24-46`), and `cutver init`
already discovers nested manifests and skips virtual Cargo workspaces
(`src/init/discovery.rs:139-143`).

## 7. Incremental slices

| Slice | Delivers | Touches assumptions | Cost |
| --- | --- | --- | --- |
| 0. Schema-only identity | `package` parses, is validated unique, is read by `doctor`; no release behaviour changes | none | Schema key; rejection of duplicates |
| 1. Path-aware changelog | Per-package changelog sections under the current global version | 4 | Commit-to-package path map; changelog template |
| 2. Per-package version and tag | Namespaced tags and a per-package baseline | 1 | Per-package `latest_tag`; migration of the global tag stream |
| 3. Selector and per-package commit | `--package` scopes staging, commit and rollback | 2, 3 | The §5 safety decision; N-transaction bookkeeping |
| 4. Per-package lifecycle and CI | Plugin events, publish and CI fire per package | 5 | CI fan-out; `.github/workflows/release.yml` rewrite |

Slice 0 is the smallest first step that delivers value (declared, validated
package boundaries and a stable identity for later slices) without touching any
of the five assumptions. Slices 1–4 each pay for one assumption.

## 8. Non-goals and residual

- This RFC implements nothing; no code path or config key changed here.
- Cargo `[workspace.package]` awareness and `glob` in `[[manifest]]` are separate
  `[Planned]` items (`design.md`:174-175), not solved by this proposal.
- Backward compatibility: existing single-package configs are unaffected because
  `package` is optional and `[project] name` is already parsed (`src/config/types.rs:72-74`).
- Not verified, because no implementation exists: whether a per-package tag stream
  can coexist with the floating-major-tag lookup (`src/git/tags.rs:107`), and the
  exact CI fan-out. No behaviour was exercised.
