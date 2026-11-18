# The reference plugin, documented, and the CLI page relieved

Feature: finish the plugin documentation with the implementation this repository actually runs, and take
`docs/reference/cli.md` off the 200-line ceiling so the next slice has room. Branch
`docs/plugin-github-publish`, base `origin/main` at `140b3d7`.

## Specs

S1. `docs/guides/plugin-github-publish.md` (new) documents the plugin this repository ships a release with,
end to end: what it does, the `[plugins.github-publish]` declaration and every key this repository sets,
the draft-then-promote flow as it appears in `.github/workflows/release.yml`, and how a release fails when
it does. Everything is scoped to what this repository can prove: the plugin's source lives in another
repository, so the page names the keys this repository uses and says where the rest lives instead of
guessing at its full key set.

S2. `docs/reference/cli.md` drops to at most 185 lines without losing a command or an option. It shrinks by
removing statements that have a canonical home elsewhere and linking to them: the publish validation
narrative that `docs/internals/pipeline.md` owns, the two-phase ordering that both pages restate, and the
notes-exemption sentence that `pipeline.md` and the `publish` section both spell out.

S3. No page is moved or renamed and no heading inside `cli.md` changes, so every existing anchor keeps
resolving.

S4. Every claim is traceable to a `path:line`, no page exceeds 200 lines, `cargo test --test
ci_workflow_contract` stays green, and every relative link in a touched file resolves.

## Tasks

| ID | Spec | What | Route | Commit |
| --- | --- | --- | --- | --- |
| T1 | S1 | Write the reference-plugin page. | worker | pending |
| T2 | S2 | Trim `cli.md` by linking to canonical pages instead of restating them. | worker | pending |
| T3 | S3, S4 | Prove anchors, links, budgets and the ceiling test. | worker | pending |

## Constraints for the worker

- Evidence-first, no hype, scope every absolute, state residuals.
- Do not describe behavior that is not on `origin/main`.
- Do not guess at another repository's surface. If a plugin key cannot be verified from this repository,
  say that it cannot and point at where it can.
- Do not delete a command or flag from `cli.md` to make the budget; trim prose and restatement only.

## Log

L1. User, verbatim: «abre el issue faltante, mira el diff y termina lo que falta de la docs para seguir en otra session con trabajo en paralelo»

L2. Evidence: `docs/reference/cli.md` is at 197 of the 200-line ceiling enforced by
`.github/workflows/ci.yml` and by `tests/ci_workflow_contract.rs`; the plugin documentation slice (`#315`)
left `github-publish` as the only plugin surface with no page, because its key set lives in the
`cutver/plugins` repository and only the declaration plus the workflow flow are provable here.
