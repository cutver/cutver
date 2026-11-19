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
maintainer."* Those four phrases are the four checkboxes, in that order.

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
| T2 | `.github/PULL_REQUEST_TEMPLATE.md` carrying the four checklists of S2, the two required checks, and the label trap | S2, S4 | worker |
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
