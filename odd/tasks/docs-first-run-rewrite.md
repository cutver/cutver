# First-run documentation: teach the two-phase release

Branch `docs/first-run-two-phase`, stacked on `docs/pillar-iv-7-checks-name-their-object` so the
deferral-record fix and Pillar IV.7 ride along as its first two commits. Item 1 of the pre-4B queue that
`CONTRACT.md`'s freshness rule now makes enforceable.

## Specs

**S1 — the request.** *"la docs considero hay que actualizarla o rescribir parte como el inicio rápido,
porque hemos evolucionado vastante cutver como para tener docs desactualizada, que esa debe ser una regla
que agregaremos, todo lo que se añada, modifique, rescriba, etc, debe ir perfectamente documentado y con
ejemplos claros de su aplicacion y puesta en práctica"*.

**S2 — the flow the page must teach.** *"`getting-started.md` must teach `plan` first (read-only, mutates
nothing) and then `bump --defer-publish` → rewrite notes → `publish --notes-file -`, because that is now the
recommended flow and the page currently teaches the single-command one."* The single-command release stays
documented as the short path, not as the recommendation.

**S3 — freshness, not presence.** Shipping this rewrite means revisiting the pages that describe the flow
*around* the surface (`CONTRACT.md:142`), which is why the two-phase release and the read-only plan are the
subject and not a new row somewhere.

**S4 — the survivor rule applies to the rewrite itself.** *"Compaction moves a fact to another page; it
never drops one"* (`CONTRACT.md:141`). Every fact on the page today must still exist afterwards, in this page
or in the canonical one that owns it. The inventory to preserve, checked one by one in the review:

- what `cutver init` detects and scaffolds, including `--update` for packages added later;
- `cutver doctor` and `doctor --check-changelog`, and that version drift exits `2`;
- the SemVer deduction rules **and their example commits**;
- that an explicit level can override deduction (`patch` / `minor` / `major`);
- the six-step atomic pipeline and where it really lives;
- the rollback guarantee's boundary: nothing before the release commit is left half-applied, and the commit,
  the tag and the publish come after that point and are not rolled back;
- the six Next Steps links.

**S5 — an example is a transcript or it is a claim.** *"an example is a transcript or it is a claim"*. The two
`text` blocks on the page today (`cutver doctor` and `bump --dry-run` output) are invented: nothing in them
came from a run. They are replaced by real output captured from a scratch repository, and every `cutver …`
line inside a `bash` fence is parsed against the CLI by `tests/documentation_examples.rs`, so a flag that does
not exist fails the suite instead of shipping.

**S6 — the ceiling.** Hard 200 lines, trigger 150 (`CONTRACT.md:140`). Crossing 150 obliges either a split or
a stated reason in the pull request note. The page is 127 today and the two-phase flow adds steps; the honest
response is to remove duplication, not to compress prose.

**S7 — out of scope on purpose.** Gates, `--require-gate`, attestations and the `$GITHUB_OUTPUT` channel belong
to `docs/integrations/github-actions.md` and `production-release-workflow.md`, and the record defers them until
phase 4B lands. This rewrite does not teach them from the first-run page.

## Shape decided before writing

- The six-step pipeline diagram leaves `getting-started.md` and becomes a one-line pointer to
  `docs/internals/pipeline.md`, which already carries all six phases in more detail (`pipeline.md:24-27`). The
  fact is not dropped: it is already stated there, better.
- The SemVer table splits along its own seam: the **rules** already live in
  `docs/internals/references.md:59-66`, so they stay there; the **example commits** exist only in
  `getting-started.md` today and move into that same section, which keeps them without duplicating the page.
- `getting-started.md` keeps the journey: init → doctor → plan → `bump --defer-publish` → rewrite the notes →
  `publish --notes-file -`, plus the short single-command path and the rollback boundary.

## Tasks

| ID | What | Specs | Route |
| :--- | :--- | :--- | :--- |
| T1 | Replace the fabricated transcripts with output captured from a scratch repository | S5 | worker |
| T2 | Rewrite the journey around `plan` and the two-phase release, keeping the short path | S2, S3 | worker |
| T3 | Move the pipeline diagram to a pointer and the SemVer examples into `references.md` | S4, S6 | worker |
| T4 | Verify the fact inventory of S4 survives, and run the doc checks | S4, S5, S6 | worker + parent |

## Log

- **L1** — The request, recorded in `odd/tasks/deferred-findings.md` while the CI contract was being built.
- **L2** — Maintainer, 2026-11-19: *"Dale con esto"*, after the queue was stated in order.
- **L3** — Evidence before the first write: the page is 127 lines; its two `text` blocks are invented
  transcripts; `bump` flags `--defer-publish`, `--report`, `--require-gate` and `--notes-file` appear nowhere
  on it; `plan` appears nowhere; the six phases already live in `pipeline.md:24-27`; and the SemVer rules live
  in `references.md:59-66` without the example commits.
