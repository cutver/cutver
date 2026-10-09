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
