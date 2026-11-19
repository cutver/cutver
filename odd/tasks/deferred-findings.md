# Deferred Findings

> **Purpose**: the index for defects found while building the plugin ecosystem that were
> deliberately **not** fixed in that work. One place to look, instead of guessing which
> feature document to open.
>
> The `todo` tool holds the **active plan only**. Deferred work lives here, with its
> evidence and the reason it was deferred, so it does not compete visually with what is
> actually being worked on.

---

## Open

All six are tracked as issues in the core repository, which is where they will actually be picked up.
This table is the index: the issue holds the full evidence and acceptance criteria, and this file
holds the reason each was deferred rather than fixed in place.

| Issue | Finding | Why it was deferred |
| --- | --- | --- |
| [#159](https://github.com/cutver/cutver/issues/159) | `docs/internals/references.md` is **419 lines** against the Pillar IV.5 hard ceiling of 200. `docs/internals/design.md` is 175, above the 100-150 target. | Pre-existing and unrelated to the plugin work. Decomposing a reference document is its own reviewable unit, and doing it inside a plugin change would bury both. The closed issue #131 claimed all documents were calibrated to 110-150 lines, so that criterion no longer holds. |
| [#160](https://github.com/cutver/cutver/issues/160) | No blank line between the `# Changelog` header and the newly inserted version heading. | Pre-existing cosmetic defect in the changelog prepend path. The control experiment against `format = "keep-a-changelog"` is what attributes it correctly; without that, the plugin would have taken the blame. |
| [#161](https://github.com/cutver/cutver/issues/161) | `src/plugin/manager.rs` production portion is **381 lines**, above the Pillar V.4 target of 300 and past the 350 mark where decomposition becomes mandatory. | Recorded as finding 7 in [Contract Compliance Hardening](contract-compliance-hardening.md). Inside the 400 hard ceiling, so a target miss rather than a breach — an earlier reading of "1038 lines" wrongly counted the test module. |
| [#162](https://github.com/cutver/cutver/issues/162) | Layering inversion: `use crate::cli::BumpLevel;` in `src/bump/exec/pipeline.rs:12` and `src/bump/tests.rs:2`. | Recorded in "Not in scope" in [Contract Compliance Hardening](contract-compliance-hardening.md). Same class as the defect that unit fixed, but resolving it moves a clap-derived enum across the boundary — a separate refactor with its own risk. |
| [#163](https://github.com/cutver/cutver/issues/163) | `cutver plugin schema` (layer 3): emit JSON Schema from the PDK types for polyglot plugin authors. | Planned future work, not a defect. See WU-D in [Plugin PDK Extraction](plugin-pdk-extraction.md). Layer 2 made it cheap: the contract now lives in a published crate of the core, so the CLI can emit it. |
| [#164](https://github.com/cutver/cutver/issues/164) | Public plugin authoring guide for third parties. | Premature until a plugin is actually released and the registry entry exists; writing it now would document an unreleased flow. |

---

## Next steps owned by the user

These are decisions, not work items, which is why they are not on the active plan.

- **Push the three repositories and open the pull requests.** Nothing in this effort has been
  pushed: `cutver` has `feat/extract-cutver-pdk`, `cutver/plugins` has `test/e2e-real-host`, and
  `cutver/registry` has its initial commit.
- **Tag `github-releases-v0.1.0`** and let the plugin release workflow run for the first time.
  The workflow parses and its tag derivation was verified against a real build, but it has
  **never executed**. This repository's release workflow has failed twice before, on the missing
  Rust toolchain and on a stale floating action tag.
- **Write the first `cutver/registry` entry** from that release's `.sha256` sidecar. It cannot be
  written before then: a local build's digest is not the published asset's digest.

---

## Closed during the plugin ecosystem work

For contrast, so the list above is not mistaken for "everything that was outstanding". These were
found and fixed: the capability-as-export ABI that the reference PDK could not satisfy, the absence
of any real WebAssembly coverage, the lack of an operation discriminant, silent error swallowing in
the first-time-contributor path, terminal I/O inside the domain layer, the duplicated wire contract,
and CI silently losing workspace coverage.

---

## Pending: the maintenance surface (do this **before** phase 4B of #326)

Requested verbatim: `luego de mergear esta pr si pasa verde, agregaremos como pendientes el agregar una platilla
completa para los issues, PRs y todo lo que se deba hacer para mantener este proyecto como hasta ahora lo traigo, esto
lo ahremos en otra session antes de 4B`.

- **A complete template set**: issue templates (bug, feature, contract-debt), a pull-request template that carries
  the checklists the project already applies by hand — the gate was run, the docs moved with the behaviour, the slice
  boundary is stated, every claim is verified — and whatever else makes the maintenance visible to someone who is not
  the maintainer. The point is to transfer criterion, not to add ceremony: `CONTRIBUTING.md` is the page, these are the
  prompts that sit in the path.
- **Settle the contract changes before writing the templates**, since the templates are the operational copy of the
  contract and writing them first would freeze the current wording (see the section below).
- **Then 4B**, from a fresh `origin/main`.

## Pending: CONTRACT.md re-evaluation after the size-budget decision

Decision verbatim: `Eso de asegurar pesos no lo veo viable ya, porque la tool está creciendo y lo vale todo`.

Agreed: the tool is growing and the growth is worth it. The distinction to keep: **the number goes, the measurement does
not.** A budget nobody enforces is what let the 400-line ceiling reach sixteen violators with one file at 1308 lines.

- **Replace Pillar VI.4's ~4 MB target** with a shape invariant that can be checked mechanically — the lean build's
  dependency graph must not contain `extism` or `wasmtime` — plus size *reported* per release rather than enforced. Size
  is a channel cost (container layers, download); startup latency is the user cost, and neither is currently measured.
- **Re-evaluate the five numbers the contract carries** and their inconsistent enforcement: 200 doc lines (enforced,
  works), 400 source lines (enforced as a ratchet), 350 facade (judgement, says so), 300-400 review advisory (violated by
  three of four phases and by this one's predecessor), 4 MB (fantasy). A contract with five numbers and three enforcement
  levels invites drift; either a number is enforced or it is stated as a target with the evidence rule attached.
- **Add the rule that every declaration ships with its check.** In one session four declarations were found unenforced:
  `permissions.env` (#300), `[gates] require`, `--ci` without a sink, and the 400 ceiling itself. That is a pattern, not
  four accidents.
- **Add a scope boundary, since growth is now welcome.** The contract governs code quality and says nothing about what
  the product is allowed to become. Candidates: GitHub is a channel and never an identity (every CI integration must
  degrade to stdio and exit codes); the core grows no HTTP client and no provider SDK; plugins extend the four typed
  slots and nothing else.
- **Add the compat and deprecation policy** that `schemaVersion` implies but nobody has written: additive within a
  version, a major bump needs a deprecation cycle with a warning channel (#272 is still open), and a surface cannot be
  removed in the same release that renames it.
- **Keep the doc ceiling and add its survivor rule.** 4A compacted `machine-contract.md` to 199 lines and silently
  deleted four statements of behaviour on the way. Compaction must move a fact, never drop it.

## Pending: documentation freshness, and the rule that keeps it

Requested verbatim: `la docs considero hay que actualizarla o rescribir parte como el inicio rápido, porque hemos
evolucionado vastante cutver como para tener docs desactualizada, que esa debe ser una regla que agregaremos, todo lo
que se añada, modifique, rescriba, etc, debe ir perfectamente documentado y con ejemplos claros de su aplicacion y
puesta en práctica`.

Measured 2026-11-19 — occurrences of the newer surfaces per page:

| Page | `cutver plan` | `attest` | `--require-gate` | `--defer-publish` | `--notes-file` | `GITHUB_OUTPUT` |
| :--- | --: | --: | --: | --: | --: | --: |
| `docs/guides/getting-started.md` | 0 | 0 | 0 | 0 | 0 | 0 |
| `docs/guides/visual-tour.md` | 0 | 0 | 0 | 0 | 0 | 0 |
| `docs/guides/faq.md` | 0 | 0 | 0 | 0 | 0 | 0 |
| `docs/integrations/github-actions.md` | 0 | 0 | 0 | 3 | 1 | 1 |
| `docs/integrations/production-release-workflow.md` | 0 | 0 | 0 | 3 | 2 | 1 |

The three onboarding pages — one of which the README sends a stranger to first — teach a cutver from before phase 2
(`plan`), phase 3B (attestations) and phase 4A (the CI channel). The two integration pages know the two-phase release but
not the gates.

**The rule, in three parts** (the current Pillar IV.5 already asks for docs in the same commit and was clearly not enough,
because it asks for *presence* and the failure is *freshness*):

1. **Same commit** — a new or changed surface ships its reference row and at least one worked example in the commit that
   changes it. Restated, not new.
2. **Freshness** — shipping a surface obliges you to revisit the pages that describe the flow *around* it. A new flag is
   not a row in the reference; it changes what the first-run page recommends, and leaving that page teaching the old flow
   is the defect this section exists to stop.
3. **Examples that were run** — an example is a transcript or it is a claim. The README's two-phase block was executed in
   a scratch repository before it was written down, and that is the standard.

**Enforcement, and where it must stop:**

- **Surface parity, mechanically.** Every subcommand and flag in the clap tree appears in `docs/reference/cli.md`, and
  every documented flag exists. Deterministic, cheap, and the exact analogue of the YAML contract tests. This catches
  "we shipped `attest` and the reference never learned" without a human reading anything.
- **Documented commands must parse.** Extract the `cutver …` lines from fenced blocks and check them against the CLI
  (parse only, no execution). Turns examples from prose into a checked claim, at the cost of one test and a convention
  that examples are fenced and start with the binary name.
- **Narrative pages stay a review obligation, and the rule must say so.** `design.md`, `pipeline.md` and the guides explain
  *why*; no test can verify that a sentence is still true. Pretending otherwise produces prose that passes and lies.
- **A `last-verified: vX.Y.Z` stamp is vanity.** Easy to add, easier to fake. Skip it.

**Consequence for the 200-line ceiling:** a rule that demands documentation per surface increases pressure on every page,
and compression is what deleted four behaviour statements in 4A. Decompose into focused pages — which `CONTRACT.md:114`
already says — and never buy a line by deleting a fact.

**The rewrite itself, when it happens:** `getting-started.md` must teach `plan` first (read-only, mutates nothing) and then
`bump --defer-publish` → rewrite notes → `publish --notes-file -`, because that is now the recommended flow and the page
currently teaches the single-command one. The two integration pages need the attestation section and the `$GITHUB_OUTPUT`
consumption once 4B lands. The reference pages are already current — 3B and 4A updated them.

## Pending: what to do with the 200-line documentation ceiling (decided 2026-11-19)

**Keep it. Do not raise it. Do not remove it. Move the trigger.**

- **Evidence the pressure is live**: `docs/reference/configuration.md` and `docs/internals/machine-contract.md` sit at
  **199** lines, four pages are over 180 and eleven over 150. That is not a healthy distribution; that is the whole set
  pressed against the wall. And the cost is already measured: 4A compacted `machine-contract.md` to 199 and silently
  deleted four statements of behaviour. The cheapest way to satisfy a line ceiling is deletion, and the cheapest words to
  delete are the qualifiers — which are the semantics.
- **Why not 300**: it does not change the incentive, it moves the wall. The same pathology reappears at 299 with more room
  to hide.
- **Why not remove it**: it is the only budget in this repository that never drifted, and only because it is checked in two
  places. Removing it would teach the opposite of what the 400-line ratchet just taught.
- **Change 1 — trigger at 150, ceiling stays 200.** Crossing 150 obliges a decision: split the page, or record in the PR
  why it is one thing. Growth must meet decomposition *before* it meets deletion, which today is backwards.
- **Change 2 — the survivor rule.** Compaction moves a fact to another page; it never drops one. This is the sentence that
  would have prevented 4A's loss.
- **Change 3 — close the hole.** `CONTRIBUTING.md` is **outside** the walk: `tests/ci_workflow_contract.rs:289-290`
  collects `README.md` and `docs/**` only, so the page that documents the ceiling is exempt from it "on honour". Either
  include it or exempt it with a written reason.
- **Tension to keep explicit**: the documentation-freshness rule above increases total volume, so the two rules pull
  against each other. The ceiling is a **per-page** limit and never a total one; the answer to growth is more pages, well
  indexed, which `CONTRACT.md:114` already mandates.
- **Legitimate exception if it ever exists**: a reference page is scanned, not read linearly, so the 3–5 minute rationale
  does not apply the same way. Prefer splitting into two reference pages; if that is genuinely impossible, an explicit
  named exemption with a reason — the same shape as the ratchet's allowlist.
