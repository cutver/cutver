# Release notes in the tree (option B) and the `release-notes-creator` skill

The release body stops being whatever the template renders from commit subjects. A release's notes live
in the repository, are written for a reader deciding whether to upgrade, and are reviewed like any other
file. The template keeps its job: it still renders `CHANGELOG.md` and remains the fallback when no notes
file exists for a tag.

Risk: item 5 (delivery/environment) — this changes the step that produces the body of every release.

## Specs

S1. A release's notes live at `.github/releases/<tag>.md` in English, committed before the dispatch. The
file name is the tag (`v0.20.0.md`), so the mapping is mechanical and the file is found by the tag the
tool reports, never by a convention someone has to remember.

S2. `release.yml`'s render step prefers that file for the version being released and falls back to
`cutver changelog latest --template .github/templates/cutver/RELEASE.md` when it is absent. The tag
comes from `steps.cutver.outputs.tag` — a step output since phase 4B of #326. The fallback reproduces
today's behaviour byte-for-byte, so a release without notes files is not a different release.

S3. `tests/release_pipeline_contract.rs` pins both halves: the step names the per-tag path and tests it
for existence, and the template render survives in the same step as the fallback. A pin that only
checked the preferred path would let the fallback be deleted and would turn "optional with fallback"
into "required", which is the disagreement S2 exists to prevent.

S4. The body applied to `cutver 0.20.0` on 2026-11-20 is backfilled as `.github/releases/v0.20.0.md`:
today that text lives only on the release page, which makes it the one published artifact this
repository cannot diff, review or roll back from git. The backfill also proves the path and name shape
against a release that already exists, at zero risk.

S5. A global skill `release-notes-creator` carries the craft — inputs to gather, rules, decision gates,
anti-patterns, self-audit, output contract — and configures itself per project instead of assuming
cutver's paths, tag convention or voice skill. It refuses to write notes it cannot source: a breaking
change with no recorded migration is a stop, not something to paraphrase.

S6. Out of scope, declared rather than dropped: the linter that refuses a tag when a breaking PR of the
range is unnamed (the mechanical half that keeps the freedom from becoming sloppiness), and any check
that every published tag has a notes file. Both are follow-ups with their own evidence line.

## Tasks

| ID | Work | Specs | Route | Commit |
| :--- | :--- | :--- | :--- | :--- |
| T1 | Render step prefers `.github/releases/<tag>.md`, falls back to the template | S1, S2 | `gentle-ai-worker` | pending |
| T2 | Contract test pins the preference and the fallback together | S3 | `gentle-ai-worker` | pending |
| T3 | `CONTRIBUTING.md` names the convention where maintenance is described | S1 | `gentle-ai-worker` | pending |
| T4 | Backfill `.github/releases/v0.20.0.md` from the body applied to the release | S4 | inline | pending |
| T5 | Global skill `release-notes-creator` at `C:\Users\Admin\.agents\skills\` | S5 | inline (outside the repo) | — |
| T6 | Local gate + independent verification (risk item 5) | S1-S5 | `gentle-ai-verify` | — |

## Log

L1. Maintainer, verbatim: «Bien Iremos con la B para las release notes, por lo que tendremos que crear
nuestra skill de release-notes-creator para tener la mejor skill para esto, que dices y que propones?»

L2. Maintainer's answers, verbatim: «Global (~/.agents/skills), reusable en los otros repos» and «U1 + U2
en una sola PR». Consequence recorded: a global skill cannot encode this repository's voice as law, so it
carries the craft plus a per-project configuration step, and the repo-specific decisions (path, tag
mapping, voice skill) stay in this repository.

L3. Context the decision came from, maintainer verbatim: «en el repo todo es en ingles, español solo para
mi en el chat» and the evaluation that started it — whether the template is viable for release notes or
the repo's policies have to be relaxed. The analysis found no policy to relax: the recorded guardrail
already allows an enriched body, and `CHANGELOG.md` stays the deterministic render, so the only thing
forcing the template into the body was one line of `release.yml`. Evidence from 0.20.0: the render
produced 46 lines of commit-level bullets, the enriched body 134 lines, and the intro, both migrations in
prose, the "no plugin is dispatched" fact and the not-included section existed nowhere in the render.

L4. Known cost, accepted: this adds an authoring step per release. With one maintainer plus an agent that
is cheap; with many contributors it would not be, which is why the convention is optional-with-fallback
and never mandatory.
