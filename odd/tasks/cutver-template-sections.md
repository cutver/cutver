# A template receives release notes as sections

Feature: a `[changelog] mode = "template"` template can iterate grouped sections instead of grouping the
flat commit list itself and remembering to exclude breaking entries. Branch: `feat/template-sections`,
base `origin/main` at `fe88de1`.

## Specs

S1. The template context exposes `sections`: an ordered sequence of sections, each one carrying a
`heading`, its `entries`, and an `is_breaking` flag that is true only for the section holding the
breaking entries — so no template has to infer it from the entries or from the label text. Headings
come from the existing single authority — `SECTIONS` in `src/changelog/render/template.rs:77-89` —
never from a second copy of the label list. The authority stays where it is; reaching it from the
context module is one `pub(crate) use` line in the `src/changelog/render.rs` module root, which is why
that file is in the edit surfaces. Entries are the same commit objects `commits` already yields, with
the same keys and the same values.

S2. An entry whose `is_breaking` is true appears in the breaking section only, and in no other section.
This is the rule the flagship template hand-rolls today with
`commits | group_by_section` plus `rejectattr("is_breaking")`
(`.github/templates/cutver/RELEASE.md:12-17`); after this feature a template that iterates `sections`
gets it without knowing about it.

S3. Sections with no entries are omitted, so a template never renders an empty heading. With no commits
at all, `sections` is empty and `all_changes` keeps its current fallback behaviour.

S4. Nothing existing changes. `commits`, `features`, `fixes`, `breaking`, `perf`, `refactor`, `docs`,
`maintenance`, `other`, `all_changes` and the `group_by_type`, `group_by_section`, `group_by_scope`
filters keep their current shape, order and content.

S5. `.github/templates/cutver/RELEASE.md` — which is also the scaffolded default, embedded by
`src/init/scaffold/templates.rs:15` — stops hand-rolling the grouping and the breaking exclusion, and
renders the same body for the same input as before.

## Tasks

| ID | Spec | What | Route | Commit |
| --- | --- | --- | --- | --- |
| T1 | S1, S2, S3 | Add `sections` to `ReleaseContext` (serialize map, deserializer, construction) and populate it where the buckets already exist. | worker | `e759dd8` |
| T2 | S2, S3 | Pin the rules with tests: breaking appears once, canonical order, empty sections omitted, no commits yields an empty sequence. | worker | `e759dd8` |
| T3 | S4 | Prove the existing keys and filters are untouched (the context and renderer tests that pin them stay green). | worker | `e759dd8` |
| T4 | S5 | Rewrite the flagship template on `sections` and confirm the rendered body is unchanged for the current release set. | worker | `72ffee6` |
| T5 | S1 | Document the new key in `docs/reference/configuration.md` and `docs/templates/release-notes.md`. | worker | `e759dd8` |

## Log

L1. User, verbatim: «Sí, cuando marque verde mergea y sigue con el siguiente slic»

L2. Correction on the record. The premise stated earlier in conversation — "template mode receives the
flattened commit list, so it cannot group" — was wrong, and the evidence says so: `ReleaseContext` is
assembled before the mode branch (`src/bump/exec/changelog.rs:75-87`), so template mode already receives
populated `features`/`fixes`/`breaking`/… blocks (`src/changelog/context/assemble.rs:58-136`), the
`group_by_section` filter exists (`src/changelog/render/template.rs:130`), and the flagship template
already groups with it. The real gap is narrower and stated in S2: the grouping arrives with breaking
entries mixed in, so every template must remember `rejectattr("is_breaking")` or it renders them twice.

L3. Decision (parent): take the narrow, evidenced gap rather than the wrong one. Package-section grouping
for template mode (`src/changelog/sections.rs:16-26` explicitly excludes it) stays out of this document
and gets its own slice if the monorepo story needs it.

L4. Decision (parent), answering the worker's blocker: option A. `mod template;` in
`src/changelog/render.rs:2` is private, so the context module cannot reach `SECTIONS` without one
`pub(crate) use` line in that module root (observed: `error[E0603]: module template is private`). The
authority does not move — S1 keeps naming `src/changelog/render/template.rs` — and each section carries
explicit `heading`, `entries` and `is_breaking` instead of leaving the flagship template to infer the
breaking section from its entries. Rejected: moving `SECTIONS` into the context module (churns every
renderer for nothing) and rendering a throwaway expression to harvest labels (obscure).

L5. Slice boundary and delivery strategy, recorded before pushing. The two commits are `e759dd8` (domain: `sections` in the context, its authority reach, its docs and its tests) and `72ffee6` (consumer: the flagship and scaffolded template switches to `sections`, plus the `#![cfg(unix)]` assertion in `tests/e2e_init.rs` that pinned the old markers). Together they are 549 authored lines, past the ~400-line advisory: 384 of those are tests, the production change is roughly 120 lines, and the two commits cannot deliver green apart as separate PRs without the second re-pinning a body the first already proves. So this ships as one PR with the overrun declared in its body, not as a silent oversize.

L6. Independent verification (gentle-ai-verify) found a blocker the Windows gates could not: the rewrite removed the two markers `tests/e2e_init.rs` asserted, and that file is `#![cfg(unix)]`, so it runs zero tests on this host. Fixed in `72ffee6` by re-pinning the new markers. Recorded because the same blind spot has produced CI-only failures in this repository before.

L7. CI caught a second Windows-only failure the local gates could not: `flagship_template_renders_the_same_body_it_always_rendered` compared the rendered body against an LF literal, but `DEFAULT_RELEASE_TEMPLATE` is `include_str!` of `.github/templates/cutver/RELEASE.md`, so the CI checkout (autocrlf) bakes CRLF into the template. Reproduced locally by converting the template to CRLF (RED), fixed by normalising the rendered body, and proven green in both states (CRLF and LF, 142 tests each).
