# #353 — make the 1.0 enforcement condition decidable

#346 makes `1.0` reachable on three conditions. The second read "the enforcement table in `CONTRACT.md` has no
`judgement` row left in anything the 1.0 promises" — and nothing could decide it: the contract carried four judgement
obligations, none of them a surface a consumer parses, so the condition was satisfiable by declaring it satisfied. That
is the same shape as the four accidents the program exists to end (`permissions.env`, `[gates] require`, `--ci` without
a sink, the 400-line ceiling): a declaration nobody wired to a check.

The maintainer's verdict, recorded 2026-11-21: **every judgement obligation is outside the 1.0 freeze.** The decision
stays human; the mechanism does not.

## Specs

**S1 — the contract classifies every judgement obligation.** `CONTRACT.md` carries `## What the 1.0 Freeze Covers`: an
intro stating what the freeze actually freezes (machine surfaces, the configuration promise, the release transaction,
the plugin ABI) and one table row per obligation, each naming **Inside** or **Outside** and giving its reason.

**S2 — the classification is complete, and the pin can tell.** `tests/freeze_scope_list.rs` derives obligations from
the document in the two shapes the contract states them: the `Judgement` row of the limits table (keyed by the limit it
names) and every judgement-worded clause inside a pillar (keyed by the bullet's bold lead, or by the subsection title
when the clause stands alone). A listed obligation the document does not carry fails the pin, so the list cannot drift
into decoration.

**S3 — the pin cannot pass vacuously.** A floor of five obligations, current count six locations folding into five
obligations; and the level-vocabulary table at `CONTRACT.md:24` — which defines the word rather than stating an
obligation — is not counted, proven by a fixture where only the limits table may produce a finding.

**S4 — every row carries a verdict and a reason.** A verdict other than `Inside`/`Outside` fails, and a reason shorter
than 20 characters fails: a classification without its reason is the checklist this pin exists to prevent.

**S5 — the condition in #346 cites the mechanism.** Condition 2 is rewritten to point at the section and the test, and
#346 carries a comment recording the decision, so reachability stops being an opinion.

## Evidence

- The pin fails on the tree as it was, naming an unclassified clause added to a pillar in the real document:
  ``Unclassified: ["`a brand new judgement clause` at CONTRACT.md:264"]`` — then passes when the clause is removed.
- Two scratch documents prove the two directions plus the vocabulary-table exclusion, and a third proves a planted
  clause is reported with its `path:line`.
- `tests/freeze_scope_list.rs` 4/4; the full local gate and CI on the PR carry the rest.

## Notes

- The issue said "four judgement obligations"; the map found **six locations**, because `CONTRACT.md:154` and `:155`
  (Pillar IV.7) are two separate judgement obligations, and `:33` and `:180` are one obligation stated twice. The
  classification therefore has five rows, and the repetition is a fact of the matching, not an extra decision.
- `CONTRACT.md` is 259 lines and is **exempt** from the 200-line document ceiling (`tests/ci_workflow_contract.rs`
  `EXEMPT`, because it defines the ceiling and is consulted clause by clause). The new section adds 26 lines.
- Adding a `##` section at the end does not disturb anchors: the only external dependency on a heading is
  `docs/reference/configuration.md` → `#2-configuration-backward-compatibility`, validated by
  `tests/documentation_links.rs`.
