# #307 — a non-patch release must declare what it does about compatibility

`#307`'s failure mode: `v0.19.0` shipped a breaking plugin-environment contract — `env_clear()` plus grants, an unset
grant now a hard error — and its notes say nothing, because no commit in `v0.18.2..v0.19.0` carried a `BREAKING CHANGE:`
footer. The notes template has the `> [!WARNING]` block ready; it rendered nothing because `is_breaking` was false for
every commit. Silence was read as "compatible".

The rule as the issue words it ("level is `minor` or `major` and no commit declares a breaking change → fail") would
block **every feature release**, because a normal `minor` is by definition a range with a `feat` and no declaration —
`0.19.0` and `0.20.0` are both that shape. The maintainer chose **option A** (2026-11-21): the declaration is required
in both directions.

## Specs

**S1 — the object is a version move over a commit range.** The check compares the release's level against the commits of
`<previous_tag>..HEAD`. It applies when the level is `minor` or `major`, whether `auto` derived it or `--level` named it
(the release workflow dispatches `bump=minor`, which is exactly how `0.19.0` was cut). A `patch` release is exempt: an
ordinary fix release never needs a declaration. A promotion is a version move with **no commits**, so it is not this
check's object and the check must not grow a promotion-shaped branch.

**S2 — two footers, symmetric.** The range has declared its intent when at least one commit carries either

- `BREAKING CHANGE: <migration>` (or the `BREAKING-CHANGE:` spelling, already parsed) or a `!` breaking header, or
- `NO-BREAKING-CHANGE: <why this release is compatible>` — the reviewed-none declaration.

Anything else is silence, and silence fails.

**S3 — the failure names what the author must add.** The error follows the crate's shape: first line, `Where:`, `Fix:`
(`src/bump/types.rs`'s existing variants are the model). It names the range it compared (`v0.18.2..HEAD`), the computed
level, and both footers to choose between, with one worked example. It runs before anything is written, staged,
committed, tagged or pushed.

**S4 — `plan` reports it as a blocker, not a crash.** `cutver plan` renders it as data with exit `0` and writes
nothing, like every other blocker (`src/bump/plan/guards.rs`, wired where `dirty_tree_blocker` is wired).

**S5 — the sentinel never reaches the notes.** `NO-BREAKING-CHANGE:` is review provenance, not release content: the
rendered body must not contain it, and a body for a range that declares `BREAKING CHANGE:` still carries the
`> [!WARNING]` block and the per-commit `> **Migration**:` line.

**S6 — a footer is a footer.** `NO-BREAKING-CHANGE:` counts only as a parsed commit footer, never as prose that happens
to contain the words: a commit body mentioning the footer's name must not satisfy the check.

**S7 — budgets and docs.** `src/bump/plan.rs` is at 389 of 400 and `src/bump/types.rs` at 358; neither is allowlisted,
so the wiring is a few lines and the regression risk is a new file or a guard, not a rewrite. The footer convention and
the failure are documented on the page that describes the release flow, in the commit that ships the behaviour.

## Tasks

- **T1** — S1/S2/S3: the guard and its error, with the two-direction rule and the patch exemption.
- **T2** — S4: the `plan` blocker path.
- **T3** — S5/S6: the notes and footer-shape tests.
- **T4** — S7: the docs, and the two budget checks green.

## Log

- **L1** (maintainer, verbatim): «continuemos con lo que tenemos pendiente mientras arreglo una clase de mi hermana,
  estudio para una exposición y un examen».
- **L2** — on the choice of rule: «Dime cual es la que más conviene de las opciones». Recommended and taken: **A**,
  requiring the declaration in both directions, because B fails every feature release and C would not have caught
  `0.19.0`.
- **L3** — the level a first release reports does not change: `plan --json --first-release minor` still prints
  `minor`, and the check exempts a first release on its missing previous tag instead of on the level's name.
  Two absences are outside the check for the same reason S1 gives for a promotion: no previous tag (nothing shipped
  to compare against) and an empty range (a version move with no commits).
- **L4** — `tests/common/mod.rs::initial_commit` carries a `NO-BREAKING-CHANGE:` footer so the ~40 fixtures that tag
  their first release keep releasing. That is fixture scaffolding, not coverage: the rule is exercised only by
  `tests/breaking_change_declaration.rs`, and a future test that wants to observe the gate must build its own range.
- **L5** — the same convention reaches the one in-crate fixture that releases over a tag:
  `bump::tests::dry_run_auto_bump_deduces_from_commits` commits `feat: add brilliant feature` with a
  `NO-BREAKING-CHANGE:` footer after `v1.2.3`, so it still witnesses `auto` deducing `1.3.0` from a `feat` through a
  guard that would otherwise refuse the range. The gate itself is still covered only by
  `tests/breaking_change_declaration.rs`.
