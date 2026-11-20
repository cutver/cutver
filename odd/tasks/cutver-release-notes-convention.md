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

S2a. A notes file that exists but carries no text is a failure **at the render step**, with an
`::error::` naming the file, not a zero-byte body that fails nineteen minutes later inside
`cutver publish` after the bump push and the build and smoke legs. Standing next to it: when the file
for this tag is absent while `.github/releases` holds notes for other tags, the run emits a
`::warning::` that names them, because a mis-named file (`0.21.0.md` for `v0.21.0`) must be visible
rather than silently replaced by the template body.

S3. `tests/release_pipeline_contract.rs` pins both halves: the step names the per-tag path and tests it
for existence, and the template render survives in the same step as the fallback. A pin that only
checked the preferred path would let the fallback be deleted and would turn "optional with fallback"
into "required", which is the disagreement S2 exists to prevent.

S3a. The pin must also prove the two halves are **branches and not neighbours**: the template render
sits inside the `else` of `if [ -f "$notes" ]`, between `else` and `fi`. Moving it to a line after `fi`
keeps every string the pin looks for, passes the suite, and makes the preference dead — the render
overwrites the committed notes and every release ships the commit-level body. And `RELEASE_NOTES.md`
must have exactly one writer, the render step, because a second one would let the artifact's body come
from two places.

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
| T1 | Render step prefers `.github/releases/<tag>.md`, falls back to the template | S1, S2 | `gentle-ai-worker` | `237b87b` |
| T2 | Contract test pins the preference and the fallback together | S3 | `gentle-ai-worker` | `237b87b` |
| T3 | `CONTRIBUTING.md` names the convention where maintenance is described | S1 | `gentle-ai-worker` | `237b87b` |
| T4 | Backfill `.github/releases/v0.20.0.md` from the body applied to the release | S4 | inline | `237b87b` |
| T5 | Global skill `release-notes-creator` at `C:\Users\Admin\.agents\skills\` | S5 | inline (outside the repo) | — |
| T6 | Local gate + independent verification (risk item 5) | S1-S5 | `gentle-ai-verify` | — |
| T7 | Close A1: pin the fallback as the `else` branch and `RELEASE_NOTES.md` as single-writer | S3a | `gentle-ai-worker` | `c2d9575` |
| T8 | Close A2/A3: an empty notes file fails at the render step; a mis-named one warns | S2a | `gentle-ai-worker` | `c2d9575` |

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

L5. Independent verification (`gentle-ai-verify`, read-only, 2026-11-20): S1, S2, S5 met; S3 met but
insufficient; S6 absent, not half-built; **no reproduced blocker**. It re-derived the pin's mutations and
found A1: moving the template render out of the `else` to a line after `fi` leaves all 23 tests green and
makes the preference dead — the hole S2 exists to prevent. A2: an empty notes file yields a 0-byte
artifact and fails only inside `cutver publish`, after the bump push and 14 build and smoke legs (its own
step-script probes: present → byte-identical, absent → fallback, wrong tag → silent fallback, directory →
fallback, empty → exit 0 with a 0-byte file). A3: a mis-named file is silently ignored. A4 pre-existing
(the tag is interpolated into bash while `TagPrefix` is unvalidated) and A5 (the skills catalog still
teaches only the template) are recorded, not taken here. It could not verify S4 for lack of network; the
parent closed that by diffing the live body against the tree — identical but for GitHub's trailing
newline — and corrected its own "134 lines" to 133.

L6. Decision (parent): A1, A2 and A3 are closed before the merge, not deferred. A1 because a pin that
cannot see a dead preference is worse than no pin — it certifies the wrong thing. A2 and A3 because both
are silences that cost a release attempt: nineteen minutes to discover an empty file, and a body nobody
chose because of a file name. A4 stays a follow-up (pre-existing, and its real fix is validating
`TagPrefix`, which is #267's territory); A5 belongs to the skills catalog repository.
