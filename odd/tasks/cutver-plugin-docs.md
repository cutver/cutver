# Plugin documentation for someone starting from zero

Feature: the plugin documentation says what a plugin is before it says how to configure one, covers the
process runtime and its grants, and stops hiding `config` and `timeout_seconds` in a table cell. Branch
`docs/plugins-foundation`, base `origin/main` at `c63b635`.

## Specs

S1. `docs/guides/plugin-overview.md` (new) answers, in this order and without assuming prior knowledge: what
a plugin is and what problem it solves; the three lifecycle slots and when each runs; the invocation
envelope with one worked example of a request and its answer; WASM versus the process runtime in one table;
what a plugin cannot do; and how a plugin fails. No prerequisites, one concept per block, and a quick path
first for the reader who only wants to configure one.

S2. `docs/guides/plugin-process-runtime.md` (new) documents the process runtime completely: how `command` and
its arguments are resolved, the per-call stdin/stdout exchange, `permissions.env` — including that the child
gets a cleared environment and only the granted keys, and that a declared but unset grant aborts rather than
passing an empty string — and `timeout_seconds`, what it bounds and that the kill reaches the process tree.
It states plainly that a process plugin has **no sandbox**, which is the reason the runtime exists and the
reason to prefer WASM.

S3. The `[plugins.<name>]` key table in `docs/reference/plugins.md` lists every key the code accepts,
including `config` (a table, today used by `github-publish` in this repository's own `cutver.toml`) and
`timeout_seconds` with its unit, default and bound. The zero-authority rule is stated as one canonical
paragraph there; `faq.md` and `plugin-authoring.md` link to it instead of repeating it.

S4. `docs/reference/configuration.md` documents the `[plugins.<name>.config]` sub-table as the pass-through
it is, with the reference implementation named.

S5. Nothing that already works breaks: no page is moved or deleted, existing links resolve, every touched
document stays at or under 200 lines, and `cargo test --test ci_workflow_contract` stays green.

## Tasks

| ID | Spec | What | Route | Commit |
| --- | --- | --- | --- | --- |
| T1 | S1 | Write the overview page. | worker | pending |
| T2 | S2 | Write the process-runtime page. | worker | pending |
| T3 | S3 | Complete the key table and make the zero-authority rule single-sourced. | worker | pending |
| T4 | S4 | Document the `config` sub-table. | worker | pending |
| T5 | S5 | Prove budgets, links and the doc-ceiling test. | worker | pending |

## Constraints for the worker

- Evidence-first: every claim traceable to a `path:line` in this repository. No hype wording.
- Scope every absolute (`never`, `always`, `only`). The process runtime having no sandbox is one that must
  stay absolute and visible.
- State residuals: what is not documented yet, and which behavior is not released.
- Do not describe `[hooks] default_timeout` or any other behavior that is not on `origin/main`.
- The reader is a newcomer: define a term the first time it appears, and prefer a table or a worked example
  over a paragraph.

## Log

L1. User, verbatim: «Podemos ir modernizando la documentación de lo que ya está implementado que tiene varias versiones sin moverce la docs y creo que hay que documentar más cosas. La cuestion de los plugins hay que documentarla bastante bien y perfectamente entendible hasta por alguien que esté iniciando con baja carga cognitiva»

L2. Evidence (inventory): the recent features are already documented; the gap is plugins. Undocumented today:
`permissions.env` resolution, `timeout_seconds` semantics, process-runtime authoring, the
`[plugins.<name>.config]` sub-table (present in code and in `cutver.toml`), `github-publish` as reference
implementation, and the timeout failure text. Duplication to resolve: the zero-authority rule appears in
three pages, the two digest gates in three, the build-size pairing in three.

L3. Decision (parent): additive pages, no moves. `docs/reference/plugins.md` (176 lines) and
`docs/guides/plugin-authoring.md` (174) have no room left under the 200-line ceiling, and moving pages would
break external links and the tests that pin document content. New pages plus deduplication gets the content in
without either cost.
