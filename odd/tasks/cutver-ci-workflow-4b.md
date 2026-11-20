# CI output, phase 4B of #326 — the workflow consumes the step outputs instead of parsing the report

Slice 4B is workflow-side only: `.github/workflows/release.yml` stops extracting the five keys from
`--report .cutver-release.json` with `jq` and reads what `cutver bump` wrote to `$GITHUB_OUTPUT`
itself (4A, `e446632`). Requirement source: issue #326 acceptance criterion 6 ("the tool writes the
outputs, and the workflow stops parsing the report"), and `odd/tasks/cutver-ci-output.md` L7, which
cut the criterion in two so each half lands reviewable.

Risk: item 5 (delivery/environment). This changes the workflow that cuts every release, and no test
can execute a workflow — the contract test and one dispatch are the only evidence available.

## Specs

S1. The version job reads the bump result from the step's own `$GITHUB_OUTPUT`. The step that runs
`cutver bump` carries `id: cutver`; the separate `Read the release report` step and the
`--report .cutver-release.json` argument are gone from this workflow. The job outputs
(`tag`, `released`, `version`, `commit`) keep their names and their `steps.cutver.outputs.*` source,
so every downstream job is untouched.

S2. The bump step fails closed when the tool wrote no usable step outputs. The report-file existence
check was the old guard against a run that produced nothing and silently skipped every downstream job;
its replacement is an assertion on the same step that `released=` reached `$GITHUB_OUTPUT` **with one
of the two values the gated `if:` conditions compare against** — `released` is always written on
success and `false` on a `--dry-run` (`src/cli/ci/fields.rs::bump`), so presence alone would let an
empty value skip every gated job on a green run. It fails with an actionable `::error::` line that
says nothing was published.

S3. `tests/release_pipeline_contract.rs` pins the new shape and keeps every existing assertion in
`the_version_job_publishes_nothing_irreversible`: `id: cutver` sits on the step that runs
`cutver bump`, and neither `jq` nor `--report` appears anywhere in the version job. The old
assertion message ("the job outputs must come from the report-reading step, not from the bump step")
becomes false, so it is rewritten rather than deleted.

S4. `--report` stays a CLI feature with its documented behaviour and tests. 4B removes one caller,
not the surface: `docs/reference/cli.md`, `docs/integrations/migrating-from-the-release-action.md`
and the machine contract keep teaching it, because a local or script flow still reads it.

S5. The two integration pages that show the workflow stop showing `jq`:
`docs/integrations/github-actions.md` (the step and the claim that the report "is the only thing a
later step needs to read") and `docs/integrations/production-release-workflow.md` (the step).

S6. Out of scope, decided and stated rather than discovered later: the attestation section the
record conditions on 4B (`odd/tasks/deferred-findings.md:159`, gaps measured at zero occurrences of
`attest`/`--require-gate` on both pages) is a second subject with its own evidence line, and the
dispatch rehearsal is deferred by the maintainer ("lo del dispatch de ensayo lo vamos viendo"). Gaps
already declared for the closure report rather than carried silently.

## Tasks

| ID | Work | Specs | Route | Commit |
| :--- | :--- | :--- | :--- | :--- |
| T1 | Workflow: the bump step owns the outputs, the report step and its argument are deleted, the fail-closed guard lands | S1, S2 | inline | `e7406ae` |
| T2 | Contract test pins the new shape, existing assertions kept | S3 | inline | `e7406ae` |
| T3 | The two integration pages stop teaching `jq`; `--report` kept everywhere it is the documented surface | S4, S5 | inline | `e7406ae` |
| T4 | Local gate + independent verification (risk item 5) | S1-S5 | `gentle-ai-verify` | — |

| T5 | Close the four advisory gaps the verifier opened in the pins | S1, S2, S3 | `gentle-ai-worker` | `4014e56` |

## Log

L1. Maintainer, verbatim: «Vamos con 326 4B y lo del dispatch de ensayo lo vamos viendo».

L2. Prior question, verbatim: «Entonces, naturalmente va 326 4B?» — the answer given was procedural
(no open decision remains): the pre-4B queue (#335-#340) is merged, 4B is criterion 6's second half,
and the robustness issues (#295, #300, #307, #302) are not its precondition, only #302/#307 share
text with `release.yml`.

L3. Decision (parent), 4B scope: one PR, from a fresh `origin/main`, no attestation section and no
dispatch inside it (S6). The evidence for the workflow change is the contract test plus the gate; the
dispatch is a maintainer decision that stays open.

L4. Independent verification (`gentle-ai-verify`, read-only, 2026-11-20): S1, S2, S4, S5 met; S3 met
with a pin narrower than its wording; S6 out of scope. No blockers, six advisories. Four became T5:
(a) `--report` was forbidden only on the bump step while S3 says the whole job, and a *new* step
parsing `.cutver-release.json` with a different tool left every assertion green (reproduced);
(b) nothing pinned the S2 guard — deleting it kept the contract suite green (reproduced);
(c) the `--defer-publish` pin matched the step's text, so leaving the flag in the `args` array but
dropping `"${args[@]}"` from the invocation stayed green while the bump published (reproduced);
(d) the guard was presence-only, so a `released=` line with an empty value passed it (probe G5).
The verifier confirmed the positive side by probe: `--defer-publish` writes the five keys on the real
path and four on `--dry-run`, `on_bump_success` runs on both success paths, no `GITHUB_OUTPUT` means
no file written, and the guard exits 1 on an empty or `released`-less file.

L5. Advisory not taken into this PR, recorded instead: `.agents/skills/cutver-actions/SKILL.md:47-52`
still shows `--report .cutver-release.json` and "the commit the report names" as the workflow recipe.
That catalog is synced from the sibling `cutver/skills` repository (never edited by hand here), and S4
keeps `--report` a legitimate surface, so repointing it is a skills-catalog change with its own pin —
not a workflow change hidden in a release-pipeline PR.
