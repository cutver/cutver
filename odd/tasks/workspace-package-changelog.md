# Workspace package identity and per-package changelog sections

Implements the accepted scope of [ADR-0002](../../docs/adr/0002-shared-version-with-path-filtered-changelogs.md): one shared version, one tag, one commit, plus a schema-level `package` identity and changelog sections filtered by path. Design baseline: `design/per-package-changelog/` (boards 01-03).

## Specs

S1. One shared version per release: every package still bumps together under one tag, one commit and one publish. This change alters **where a commit is reported**, never when it ships.

S2. `package` is an **optional** key on a `[[manifest]]` entry in `cutver.toml`. A configuration that never declares it must behave exactly as today.

S3. The rendered layout is **package heading above categories**: `### <package>` followed by the existing `#### Added` / `#### Changed` / `#### Fixed` / `#### Documentation` buckets.

S4. Ownership is resolved **per changed file**: the file belongs to the package whose declared manifest path is the longest prefix of that file's path. A commit is then placed once in each package that owns at least one of its files, and never repeated inside the same package.

S5. Packages render in **declaration order** — the order their `[[manifest]]` entries appear in `cutver.toml`, not alphabetical and not by commit count.

S6. A package with no commit in the range renders **no heading at all**; empty sections are never emitted.

S7. Commits that touch no declared manifest path render under `### Other changes`, never dropped.

S8. Commit selection for a package uses one `git log <range> -- <path>` invocation per package.

S9. `package` must be unique across manifests. A duplicate is a hard configuration error naming the offending value.

S10. `package` must be accepted for **every** manifest kind: the closed `MANIFEST_KEYS` allowlist in `src/config/validation.rs` gains it, so an unknown-key rejection never fires on it.

S11. Documentation states the behavior: `docs/reference/configuration.md` documents the key, `docs/guides/monorepos.md` documents the rendered shape.

## Tasks

| ID | Spec | Task | Route | Commit |
|---|---|---|---|---|
| T1 | S2, S9, S10 | Add the optional `package` identity to the manifest schema, the closed key allowlist, duplicate validation and the reference docs | writer | pending |
| T2 | S4, S8 | Add path-filtered commit selection to the git layer: per-package ranges, unowned remainder, nested-path resolution | writer | pending |
| T3 | S3, S5, S6, S7, S11 | Group commits into package sections in the default changelog renderer and document the rendered shape, with the backward-compatibility invariant pinned | writer | pending |
| T4 | S1, S8 | Update ADR-0002's implementation status now that the accepted scope is delivered | inline | pending |

## Log

L1. User, verbatim: "refrescame que es lo que dice el adr002 hace tiempo ya que no entraba este proyecto y quise volver a él. También haz la propuesta para el nuevo corte."

L2. User, verbatim: "Me gusta lo que recomiendas vámonos con esas y de paso utiliza la herramienta de diseño para recrear cómo se verá esto y partir de algo concreto"

L3. Decisions approved by the user in L2, from the three open questions of the proposal: package heading above categories (S3); longest declared path wins for nested and multi-package commits (S4); one `git log -- <path>` per package rather than a single `--name-only` pass (S8).

L4. Design authored from L2: `design/per-package-changelog/` with `01-package-sections.html`, `02-edge-cases.html`, `03-before-after.html` and `design.json`. No artboard is handed off for implementation; the design is the rendering baseline for T1-T3.

L5. T1's schema addition forced test-only `package: None` additions to existing `Manifest` struct literals in `src/cli/doctor.rs` and `src/manifest/mod.rs`; both edits are test-only and behavior-free.
