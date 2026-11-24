# The maintenance surface: a complete template set

Branch `docs/maintenance-templates`, stacked on `docs/contract-reevaluation` (PR #336) at `d15ef40`, because
the templates are the contract's operational copy and the contract must be settled first. Homonymous item of
the #326 pre-4B queue; sits before 4B and after the contract re-evaluation.

## Specs

**S1 — the request.** *"agregar una platilla completa para los issues, PRs y todo lo que se deba hacer para
mantener este proyecto como hasta ahora lo traigo, esto lo ahremos en otra session antes de 4B"*.

**S2 — what the pull request must carry.** *"a pull-request template that carries the checklists the project
already applies by hand — the gate was run, the docs moved with the behaviour, the slice boundary is stated,
every claim is verified — and whatever else makes the maintenance visible to someone who is not the
maintainer."* S2's four phrases are the four original checkboxes, in that order. A **fifth** — *a test failed
before the fix* — was added on 2026-11-24 at the maintainer's request, after a completeness review found
that a `fix` with no failing test passed all four; see L4 and S2's own consequence, that a checklist asks the
question the page answers, at the moment the answer is needed.

**S3 — the point is criterion, not ceremony.** *"The point is to transfer criterion, not to add ceremony:
`CONTRIBUTING.md` is the page, these are the prompts that sit in the path."* So a template may not duplicate a
page; it asks the one question that page answers, at the moment the answer is needed.

**S4 — the condition is already met.** *"Settle the contract changes before writing the templates, since the
templates are the operational copy of the contract and writing them first would freeze the current wording."*
PR #336 settled it, so the templates cite the contract as it now reads: the four enforcement levels, the
survivor rule, the freshness rule, and the 300–400 target with its evidence rule.

**S5 — the three issue templates.** *"issue templates (bug, feature, contract-debt)"*, named in that order.

**S6 — the meta-rule applies to this too (derived, not requested).** `CONTRACT.md`'s Enforcement section
forbids a declaration without its check, and a template nobody checks is a form that rots. Two mechanical
facts are testable and worth pinning: the pull request template names the two required checks **exactly**
(`CI gate`, `Exactly one type label` — the strings `tests/ci_workflow_contract.rs` already pins), and the issue
forms are structurally valid configuration rather than prose that GitHub will silently refuse.

## Tasks

| ID | What | Specs | Route |
| :--- | :--- | :--- | :--- |
| T1 | `.github/ISSUE_TEMPLATE/`: `bug.yml`, `feature.yml`, `contract-debt.yml`, and the `config.yml` that controls blank issues | S3, S5 | worker |
| T2 | `.github/PULL_REQUEST_TEMPLATE.md` carrying the checklists of S2 — the four the quote names plus the fifth added in L4 — the two required checks, and the label trap | S2, S4 | worker |
| T3 | `tests/maintenance_templates.rs`: the two required check names and the issue-form structure | S6 | worker |
| T4 | One pointer line in `CONTRIBUTING.md`, which must stay under the 200-line ceiling | S3 | worker |

## Log

- **L1** — Maintainer, on the maintenance surface: the S1 sentence, recorded in
  `odd/tasks/deferred-findings.md` while PR #334 was open.
- **L2** — Maintainer, 2026-11-19, on continuing now rather than after the merge: *"Podemos seguir, porque el
  ci de macos está en cola"* — the macOS leg is a known capacity queue, not a failure, so it does not hold the
  queue.
- **L3** — Evidence before the first write: `.github/` holds `scripts/`, `templates/` (the release-notes
  template) and `workflows/`, and **no** issue or pull-request template exists. `CONTRIBUTING.md` is at 142 of
  200 lines and already carries the criterion these templates ask for: the two required checks, the label
  trap, the working-record shape, and the test policy.
- **L4** — **a completeness review on 2026-11-24 found three gaps, and the maintainer asked for all three**
  («Arreglar los tres huecos»): a project that reads credentials had no private channel, a `fix` with no
  failing test passed all four boxes, and a contract change was discovered at release time instead of in
  review.
  - **The channel, and the setting that had to move first.** `SECURITY.md` cannot promise a form that does
    not exist, so the setting came before the page: `gh api --method PUT
    repos/cutver/cutver/private-vulnerability-reporting -F enabled=true` → `{"enabled":true}`, verified
    before any prose was written. `SECURITY.md` (59 lines) then names the five surfaces a report can live
    in, each with the code that shows it, the one channel, the "do not open a public issue" rule, and only
    promises that hold: a fix on `main` in the next release, no backports before 1.0, no response-time
    guarantee. `config.yml` carries that one `contact_links` entry and its comment block no longer argues
    from `SECURITY.md` not existing — the claim the change itself falsified.
  - **The fifth box.** `A test failed before the fix`, last in the list, asks for the test and the command
    that shows it red, and lets a change with no behaviour to test opt out only by saying why. One
    paragraph after the checklist asks about contract compatibility, because the release gate (#307,
    `src/bump/plan/guards.rs:89-101`) only fails *later*, at the release, which is the wrong place to learn it.
  - **The guard grew, it did not move.** `tests/maintenance_templates.rs`: `the_pull_request_template_keeps_the_four_checklists_in_order`
    → `..._the_five_checklists_in_order`, and `the_issue_config_decides_blank_issues_and_cites_no_invented_link`
    → `..._and_routes_only_the_private_advisory_form`, which now **requires** the advisory URL, requires a real
    `contact_links:` block with `name:` and `about:`, and still rejects any other `url:` — the invented-link
    rule survives, narrowed to the one channel that is real. RED first: the test edit alone failed 2 of 4 for
    the intended reasons before the template and config changed.
  - **The sweep this entry exists for.** Adding a checkbox falsified this document's own count (`:15`, the T2
    row) and `CONTRIBUTING.md:92-93` ("four boxes"), so both were corrected in the same commit. The test's doc
    comment cites "S2's five checklists", and a citation is a claim: six months of this repository's most
    expensive defects are that sentence, with a stale number in it.
  - **Checks**: `maintenance_templates` 4/4, `documentation_links` 1/1, `ci_workflow_contract` 8/8, and
    `sh scripts/check.sh` → `PASS (9 passed, 0 failed, 1 not run here)`, the not-run being the
    `x86_64-unknown-linux-gnu` + `plugins` clippy that needs a cross C compiler this host lacks.
  - **Unverified, stated as such**: the advisory URL is asserted as a live form on the strength of the API
    check above, and the test pins the string, not its reachability. `SECURITY.md` sits outside
    `GOVERNED_ROOT_FILES`, so its 59 lines are honoured by hand rather than by a check.
