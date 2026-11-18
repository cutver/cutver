# Deprecate `cutver/release` in this repository's documentation

Feature: issue #321. The documentation stops presenting `cutver/release` as the way to release, keeps
`cutver/setup` for installation, and gains a migration page that a consumer can follow today. Branch
`docs/deprecate-release-action`, base `origin/main` at `f748416`.

## Specs

S1. `docs/integrations/github-actions.md` leads with the CLI recipe this repository runs. The action appears
only in a deprecation note that links the migration page, and the note states it is kept for existing users,
not removed.

S2. `docs/integrations/production-release-workflow.md` does the same. Its step list stops being built around
the `notes-path` output and shows the commands instead.

S3. `docs/integrations/migrating-from-the-release-action.md` (new, at most 150 lines) explains why the action
cannot cut the release this tool requires — it runs the published binary, and its command set has no
`publish` and no `--defer-publish` — gives a before/after table from the action's inputs to the CLI
commands, and shows the full recipe. It states plainly that the action never created the GitHub release
either, which is why every consumer already needed a follow-up step.

S4. `README.md` and `docs/internals/design.md` stop presenting both actions as one "official" path. The design
row that says step outputs are emitted externally by the action goes away with the action.

S5. Every claim about the action is read from its own `action.yml` in `D:/development/workspaces/release`, not
from this repository's prose. Where the two disagree, the page documents the action and says the old prose
was wrong. Nothing promises the action's removal.

## Tasks

| ID | Spec | What | Route | Commit |
| --- | --- | --- | --- | --- |
| T1 | S3 | Write the migration page. | worker | pending |
| T2 | S1 | Rewrite the GitHub Actions integration page around the CLI. | worker | pending |
| T3 | S2 | Rewrite the production release workflow page the same way. | worker | pending |
| T4 | S4 | Fix the README row and the design table row. | worker | pending |
| T5 | S5 | Prove budgets, links and that each action claim matches `action.yml`. | worker | pending |

## Constraints for the worker

- Evidence-first, no hype, every absolute scoped, residuals stated.
- Do not say the action is removed, unsupported or broken. It is deprecated: still published, still usable,
  no longer the recommended path.
- Do not change `.github/workflows/release.yml` or any test. The contract test that forbids this repository
  from running the action (`tests/release_pipeline_contract.rs:86-90`) must keep passing untouched.
- No page moved or renamed; every touched page at or under 200 lines; the new page at or under 150.
- The skills catalogs and the action repository are outside this slice; they are handled separately.

## Log

L1. User, verbatim: «Quiero evaluar si la action cutver/release sigue siendo viable tenerla, porque mismo ci de lanzamiento de este repo ya no la untiliza y las cosas las hemos ido implementando directamente en cutver, como el comando publish, etc.»

L2. User, verbatim: «abre el issue y la vamos a marcar como deprecada, nos quedaremos con cutver/setup. Tanto cutver/release como cutver/setup están en en el workspace al nivel del repo cutver»

L3. User, verbatim: «dale con el 321 que podemos hacerlo acá todavía»

L4. Evidence gathered before deciding, from the action's own files: `action.yml` accepts
`command: release|bump|changelog|doctor` and the inputs `bump`, `first-release`, `config`, `dry-run`,
`skip-preflight`, `check-changelog`, `notes-file`, `template`; it emits `released`, `version`, `tag`,
`notes-path`, `release-notes`; `src/runner/cutver-runner.ts` shells out to the `cutver` on `PATH`; and
`src/services/orchestrator.ts` runs doctor → bump → changelog and stops. So it runs a published binary and
has no way to express `--defer-publish` or `publish`.
