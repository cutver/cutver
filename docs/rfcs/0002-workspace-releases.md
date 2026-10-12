# RFC 0002: Workspace Releases

> **Status**: Decided, not implemented. The maintainer accepted the recommendation in §5 on 2026-10-11; the resulting decision is recorded in §9 for the ADR named there. No code path and no config key changed.
> **Origin**: maintainer decision of 2026-10-11 — implement workspace support for real, RFC first.
> **Provenance**: the workspace roadmap rows at `docs/internals/design.md:174-177`. A `[workspace]` table was documented as supported while the loader never read it; that sweep was PR #196, and #193 (merged) now makes unknown keys a hard error, so a repeat cannot hide. The independent workspace mode row is `[Deferred]` and links to this record.

---

## 1. Context and problem

Cutver resolves one version, one changelog and one tag per repository. In a monorepo where packages ship independently, that forces every package to share a version and to wait for every other package's release; there is no package selector in the pipeline (`docs/internals/design.md:176`). This is not a missing flag: it is the consequence of five assumptions encoded in the pipeline, each load-bearing in a place a package selector would have to reach.

## 2. The single-release-unit assumptions

| # | Assumption | Where the code encodes it | Consequence in a monorepo |
| --- | --- | --- | --- |
| 1 | One global tag stream is the version baseline | `git::latest_tag(repo, Some(&config.git.tag_prefix))` (`src/bump/exec/pipeline.rs:146`); implementation `src/git/tags.rs:77-125`; tag is `"{prefix}{version}"` (`src/git/tags.rs:56`); one global prefix (`src/config/types.rs:257-258`, default `"v"` at `:280`) | Every package is versioned against every other package's tags |
| 2 | One commit per release | `paths_to_stage` is one merged vector (`src/bump/exec/manifest.rs:96-121`, `src/bump/exec/changelog.rs:146-166`), hook lockfiles (`src/bump/exec/pipeline.rs:299-311`), staged once (`src/bump/exec/pipeline.rs:88`, `src/git/ops.rs:7`) and committed once (`src/bump/exec/pipeline.rs:92`); the clean-tree guard runs at entry (`src/bump/exec/pipeline.rs:39`, `src/git/status.rs:18`) | Cannot commit package A without package B's edits |
| 3 | The rollback boundary stops before the tag | `MutationTransaction` holds only file backups (`src/bump/exec/transaction.rs:5-10`), created `src/bump/exec/pipeline.rs:81`, committed `src/bump/exec/pipeline.rs:93`; the commit, tag, floating tag and publish sit outside (`src/bump/exec/pipeline.rs:92-104`) | A later package's failure cannot undo an earlier package's commit |
| 4 | The changelog is global and not path-aware | Range `{tag}..HEAD` built in `raw_commits_between` (`src/git/log.rs:11-20`), callers pass `HEAD` (`src/git/log.rs:58-65`); no pathspec argument anywhere in `src/git/log.rs`; filtering is release/scope only (`src/changelog/context/enrich.rs:96-140`); one entry prepended to one path (`src/config/types.rs:187-188`, `src/bump/exec/changelog.rs:166`) | Every package's commits land in one changelog |
| 5 | Lifecycle, publish and CI fire once per tag | Preflight `src/bump/exec/pipeline.rs:63`; `post_bump` `src/bump/exec/pipeline.rs:272-311`; publish push and commands `src/bump/exec/pipeline.rs:103-104,329-363`; the plugin events `src/bump/exec/pipeline.rs:383-458`; CI keys its downstream jobs off one tag (`.github/workflows/release.yml:21-28,500`) | One publish and one CI run for the whole repo |

The crux behind all five: there is no notion of package identity. `Config.manifest` is a flat `Vec<Manifest>` (`src/config/types.rs:54`) and `Manifest { path, primary, kind }` (`src/config/types.rs:162-167`) has no id, name or group; validation rejects duplicate paths and a missing current source (`src/config/validation.rs:36-47`), and version resolution picks one source (`src/config/validation.rs:7-33`).

## 3. Prior art

Mandatory research: how established tools model the release unit, tag namespacing and the dependency cascade for per-package releases in a monorepo.

| Source (version) | Release unit and tags | Dependency cascade | Relevance |
| --- | --- | --- | --- |
| release-plz 0.3.171 ([release-plz.dev/docs](https://release-plz.dev/docs)) | Per package in a Cargo workspace; a git tag per released package named `<package>-v<version>` (e.g. `tokio-v1.8.1`) | Cargo workspace and dependency-graph handling | Shows per-package tags and a cascade already shipped in Rust |
| Changesets `@changesets/cli` 3.0.3 ([changesets.dev](https://changesets.dev)) | Per package, driven by committed changeset files; independent versions | Keeps interdependent packages in sync | Namespaced versioning plus cascade as a first-class feature |
| Lerna 10.1.0 ([lerna.js.org](https://lerna.js.org/docs/features/version-and-publish)) | Fixed or independent versioning; `lerna version` writes versions, commits and one tag per released version | N/A — JS workspace | The two release-unit models side by side |
| semantic-release 25.0.9 ([github.com/semantic-release](https://github.com/semantic-release/semantic-release)) + `semantic-release-monorepo` 8.0.2 | One package by design; monorepo needs the external plugin | Plugin-provided | One-release-per-repo is a deliberate default, not an oversight |
| cargo-release 1.1.6 ([github.com/crate-ci/cargo-release](https://github.com/crate-ci/cargo-release)) | Workspace-aware; `-p/--package` selects packages, `--tag-prefix`/`--tag-name` set tags | `--dependent-version <ACTION>` rewrites dependent constraints | Cascade implemented for Cargo only |
| Cargo workspace inheritance ([doc.rust-lang.org/cargo/reference/workspaces.html](https://doc.rust-lang.org/cargo/reference/workspaces.html)) | `[workspace.package]`; `version.workspace = true` | Same-workspace dependencies inherit | Today's lockstep model is Cargo's own |
| Nx 23.3.0 ([nx.dev/docs/features/manage-releases](https://nx.dev/docs/features/manage-releases)) | `nx release`: per-project versioning, changelogs and publishing; release groups | "Release projects independently" and "Update dependents" | An explicit, shipped independent release unit and cascade |
| Turborepo 2.11.7 ([turborepo.dev/docs/guides/publishing-libraries](https://turborepo.dev/docs/guides/publishing-libraries)) | Delegates versioning and publishing to Changesets | Inherited from Changesets | A task runner that declines to build its own release unit |

The cascade is therefore not unsolved in general — Nx ships "Update dependents", Changesets keeps interdependent packages in sync, and cargo-release offers `--dependent-version`. It is unsolved *for cutver*: editors are dispatched per manifest kind (`src/manifest/mod.rs:49-111`), so each ecosystem is a new editor. That keeps the rejection about scope and demand, not impossibility.

## 4. Alternatives

| Option | Trade-off | Verdict |
| --- | --- | --- |
| **Do nothing** — keep one release unit per repository | Benefit: zero schema and pipeline change, nothing to verify. Cost: the undifferentiated changelog and the missing package identity stay | **rejected** — the identity-plus-changelog slice is small and answers the concrete complaint; reopen if it proves not to be worth its schema cost |
| **a. Per-package release unit with namespaced tags** (`core/v1.2.0`) | Benefit: a real per-package boundary — its own baseline, tag, changelog, commit and publish. Cost: package identity plus per-package tag lookup, changelog range, staging set and CI fan-out, **and the dependency cascade**, unaddressed | **rejected for now** — the four reasons in §9; reopen only if a real user with a real dependency graph asks to release one package without the others |
| **b. Per-package versions, one release and one tag** | Benefit: independent version numbers. Cost: one tag number cannot baseline N packages, so the tag no longer names a package version and consumers cannot resolve a package from its tag | **dropped** — cannot serve as the fallback; it breaks a contract and delivers no independence |
| **c. One shared version with per-package changelog sections** | Benefit: each package's commits grouped under the shared version. Cost: a commit-to-package path map and a changelog template; every package still bumps together | **accepted** — recommended in §5 |
| **d. Split into per-package repositories** | Benefit: independent everything with zero cutver change. Cost: N repositories, N CI configs, duplicated releases | **named as today's answer** — the honest answer for anyone who needs independent releases today (§8) |

Models 0–c keep one release unit for the repository as a whole; only (a) makes the package the release unit, which is why only (a) can answer "release `core` only". Model (b) gives independent version numbers but still releases everything together under one tag and commit.

## 5. Recommendation and its cost

**Recommend (c): one shared version with path-filtered per-package changelog sections, plus the schema-only package identity it requires.**

**Cost.** This adds no independence in versioning, cadence, tags or publishing — only a package identity key and a path-filtered view of the shared changelog; every package still bumps together under one tag, one commit and one publish. The schema gains a permanent key on `[[manifest]]`, and since unknown keys are a hard error (`src/config/validation.rs:123-128`) the addition is deliberate; the design convention bars a `[Planned]` item from appearing as a key table (`docs/internals/design.md:11-12`). **Residual risk:** a package identity that no other code path consumes yet is schema weight with no payoff beyond the changelog section.

## 6. Proposed shape (unimplemented)

A per-package changelog section is produced by passing the package's path to the existing log range — `git log <range> -- <path>` — a pathspec `src/git/log.rs` does not accept today; each package's commits are grouped under the current global version in the one changelog.

Identity is needed because nothing in the schema names a package (§2 crux). It is proposed on `[[manifest]]` as `package = "core"`, not a `[workspace]` table and not implied by directory: `Config.manifest` is already the only per-package list (`src/config/types.rs:54`), and directory-implied identity is ambiguous when one package owns two manifests.

```toml
# PROPOSED — not implemented, not read by any loader in this tree.
[[manifest]]
path = "crates/core/Cargo.toml"
package = "core"            # proposed identity anchor (new key)
primary = true
```

Nothing enters the [configuration reference](../reference/configuration.md) until the key exists. Nothing in this RFC is implemented.

## 7. Incremental slices

1. **Schema identity.** Add optional `package` to `[[manifest]]` and validate it (non-empty, unique). Verifiable: a fixture `cutver.toml` with the key loads, and a focused test rejects duplicate package names.
2. **Path-filtered changelog sections.** Extend the log range with the package's path so each package's commits group under the shared version. Verifiable: a fixture repo with commits in two package paths renders two labelled sections under one version.
3. **Documentation.** Promote the key into the configuration reference once it ships. Verifiable: the reference lists the key and the `[Planned]` note is removed.

## 8. Non-goals and residual

- This RFC and its decision implement nothing; no code path or config key changed.
- The remaining `[Planned]` roadmap items are unaffected: `glob` in `[[manifest]]`, Cargo `[workspace.package]` inheritance and the dependency cascade stay as they are (`docs/internals/design.md:174-175,177`).
- The honest answer today for anyone who needs independent releases is (d): split into per-package repositories, or use another tool.
- The guide already reflects the deferral: `docs/guides/monorepos.md:11` states there is no independent per-package release mode and links here.
- Not verified, because no implementation exists: whether a per-package tag stream can coexist with the floating-major-tag lookup (`src/git/tags.rs:107`), and the exact CI fan-out. No behaviour was exercised.

## 9. Decision this RFC produced

**Decision recorded in [ADR-0002](../adr/0002-shared-version-with-path-filtered-changelogs.md); authoritative there, summarized here.** This RFC produces ADR-0002 (Use one shared version with path-filtered per-package changelog sections for workspace releases). It does not update an earlier ADR; [ADR-0001](../adr/0001-dual-runtime-plugin-microkernel.md) is the separate plugin-microkernel decision taken by [RFC 0001](0001-wasm-plugin-architecture.md).

**Decision.** Adopt (c): one shared version with path-filtered per-package changelog sections, plus the schema-only package identity it requires. Reject (a) per-package release units with namespaced tags for now, with the reopen condition below. Drop (b): it breaks the tag's meaning without delivering independence, and a half-measure that breaks a contract and delivers nothing is worse than the baseline. (d) is the honest answer today for anyone who genuinely needs independent releases.

**Why (a) is rejected.**

1. **The dependency cascade is an unaddressed prerequisite, and without it (a) is unsafe.** `docs/internals/design.md:177` lists "Internal workspace dependency cascade" as `[Planned]`. In a layered monorepo, bumping `core` forces `cli`'s constraint to change, which forces `cli` to be re-released: independence exists only between packages with **no dependency edge**. Implementing the cascade means parsing and rewriting each ecosystem's dependency syntax (Cargo, `package.json`, `pyproject`) — a new manifest editor per ecosystem, a capability, not a config key (`src/manifest/mod.rs:49-111`). And (a) without the cascade degrades today's safety: today every package bumps together, so a stale intra-repo constraint is impossible by construction (assumptions 1 and 2); with (a) it becomes possible and silent.
2. **No demonstrated demand.** cutver's own repository is one crate plus `crates/cutver-pdk` (`Cargo.toml:14-16`); this feature is for hypothetical users. The risk is the mirror image of the original defect: aspiration designed as a roadmap instead of aspiration documented as reality.
3. **(b) cannot serve as the fallback.** One tag number cannot baseline N packages, so (b) buys independent version numbers while breaking the tag's meaning, and it still releases everything together.
4. **(a) is a different product category.** Cutver's identity is one product, many manifests: the origin is a single application across `package.json` + `Cargo.toml` + `tauri.conf.json` (`docs/guides/monorepos.md:25-50`), and the core principle is lockstep across ecosystems (`docs/internals/design.md:21-24`). Independent per-package releases turn cutver into a release *orchestrator* — a strategic change, not a feature.

**Transaction boundary (why this is a product decision, not a technical one).** The clean-tree guard (`src/bump/exec/pipeline.rs:39`, `src/git/status.rs:18`) and the rollback boundary (`src/bump/exec/transaction.rs:5-10`, `src/bump/exec/pipeline.rs:81-93`) are the product's safety story: two-phase mutation, a `--dry-run` that simulates without writing, and an atomic rollback of file edits **before the release commit**. In a run releasing N packages, "clean" would mean "clean except the other packages' pending changes"; a rollback that stops before the first commit cannot undo a commit already made for an earlier package; and `--dry-run` would simulate N releases, not one. Choosing (a) would require deciding whether the release unit, not the repository, becomes the transaction boundary. Because (a) is rejected, that decision is not being taken.

**Reopen condition.** This rejection is revisited **only** if a real user with a real dependency graph asks to release one package without the others. If that happens, the work **starts from the dependency cascade, never from the tags** — the cascade is the prerequisite that makes (a) safe, and tags are meaningless until it exists.
