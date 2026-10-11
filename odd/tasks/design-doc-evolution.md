# Feature: Design Document Evolution

Status: in progress
Branch: `docs/design-evolution`
Predecessor: `docs/claim-accuracy-closure` merged as `3315bb5`

## Goal

`docs/internals/design.md` is the internals design statement, and today it does not mark status:
it puts the implemented specification, the module layout and the roadmap in the same voice. That
ambiguity is the mechanism that produced the `[workspace]` phantom — a Phase 2 roadmap bullet was
read as a supported configuration table and documented as real for three verification rounds.

This feature makes the document describe what exists, mark what does not, and stay inside its line
budget by decomposing instead of appending.

## Evidence of drift (measured, not asserted)

| Section | Problem |
| --- | --- |
| Module Layout | Describes the Phase 1 shape. The real tree has 22 directories; `src/plugin/{driver,error,install,manage,registry}`, `src/cli/{args,runner}`, `src/bump/exec`, `src/changelog/{context,extract,render}`, `src/init/scaffold` and `src/atomic.rs` are absent, so the plugin microkernel does not appear anywhere in the document. |
| Core Principles | Six principles, none about extensibility. The README leads with the microkernel and first-class plugins; the design doc does not mention the plugin engine at all. |
| CLI Surface | Four lines. Omits `plugin add|list|remove`, `changelog show|open`, `changelog latest --json`, `open`. |
| Full Declarative Specification | Omits `[plugins.<name>]`, `[version] plugin`, `[changelog] format = "plugin"` and `[project]`; still shows `default_timeout = 600` with no comment while the code applies no default. |
| Evolution Roadmap | Mixes done and not-done. `cutver init` and the `post_bump` hook are shipped but listed as future; the GitHub Actions step outputs are delivered in another repository (`cutver/release@v1`), so `$GITHUB_OUTPUT` cannot be found in this tree. `--channel`, `doctor --fix`, `glob` in `[[manifest]]` and `[workspace]` are genuinely absent. |

## Tasks

- [ ] 1. Map the implemented architecture read-only: real module tree with purposes, the complete CLI surface from `src/cli/args/`, the complete config surface from `src/config/types.rs` and `src/plugin/types.rs`, and the plugin subsystem's shape (drivers, capabilities, registry, the two digest gates, the cache).
- [ ] 2. Audit the Evolution Roadmap item by item and classify each as Implemented / Partial / Planned, with the evidence for the verdict.
- [ ] 3. Rewrite `design.md`: status-marked capabilities, the real module layout, the plugin microkernel at design level **linking RFC 0001** instead of duplicating it, the complete CLI and configuration surfaces, and the audited roadmap.
- [ ] 4. Keep the document inside the budget: 100-150 target, 200 hard ceiling. Decompose and link rather than append; `design.md` currently sits at 170.
- [ ] 5. Verify: every claim carries its `path:line`, links resolve, no hype wording, no false absolutes, and the roadmap cannot be misread as implemented.
- [ ] 6. Open the pull request.

## Non-goals

- Implementing `[workspace]` mode/members/`--package`. The user decided to do it for real, but as its own RFC followed by its own implementation; this feature marks it Planned and nothing more.
- Designing the plugin artifact cache/proxy. Separate RFC.
- The open findings #193 (unknown keys silently ignored), #194 (`pre_bump` inert) and #195 (two false `--help` strings). They are code changes.

## Sequencing decision

Three units were requested in the same conversation: this design evolution, a workspace RFC, and an
artifact cache/proxy. They are deliberately **not** run at once: two parallel writers in one
repository, plus a new service design, is the multi-area change the harness is supposed to flag
rather than absorb. Order: this feature first (it fixes the document that the other two will have to
update), then the workspace RFC, then the cache/proxy RFC.

## Evidence

| Task | Commit | Note |
| --- | --- | --- |
| — | `7fff633` | `docs(odd): open the design document evolution feature` |
| 1 | explorer | Architecture map plus the roadmap audit; it uncovered three false claims outside this document |
| 3 | `184e5d3` | `docs(internals): evolve the design document with explicit capability status` — `design.md` 170 -> 180, plus the two reference fixes |

## Claims the audit uncovered (fixed here because they are the same class)

| Claim | Truth | Where |
| --- | --- | --- |
| `require_branch` accepts a list | The type is `Option<String>` — one branch | `configuration.md:77`, `:163`, and the design spec |
| The Cargo editor preserves workspace inheritance and updates `[workspace.package] version` | `src/manifest/cargo_toml.rs:11,27` reads and writes `package.version` only; a Cargo workspace with `version.workspace = true` fails with `FieldNotFound("package.version")` | `manifests.md:15`, `:35`, `design.md:123` |

`design.md` contradicted itself on the second one — the kind table asserted the support while the roadmap listed it as Phase 2. The rewrite makes that impossible by construction: the roadmap states status and nothing else in the document asserts an unshipped capability.

## Roadmap classifications (the reviewable artifact)

Implemented: canonical `cutver.toml`; the primary-manifest convention; Conventional Commits and `bump auto`; structured changelog categorization; the `post_bump` and `[publish]` hooks; `cutver init`; `doctor --check-changelog`; the Pi release skill, shipped as `cutver-release` (the roadmap called it `release-creator`).

Partial: `doctor --fix` — the checks run, the flag does not exist.

Planned: `glob` in `[[manifest]]`; Cargo `[workspace.package]` and `version.workspace = true`; independent workspace mode; internal workspace dependency cascade; pre-release channels.

External: the GitHub Actions step outputs, emitted by the `cutver/release@v1` action rather than by this tree.

## Residual

- `design.md` is 180 lines: inside the 200 ceiling, above the 150 target. The rewrite traded length for the plugin section the document never had; trimming it further means moving the module layout to its own page, which is available as a follow-up but not done here.
- `[workspace]`, `glob` in `[[manifest]]` and Cargo workspace inheritance stay unimplemented and marked Planned until the RFC lands.
- Until #193 is fixed, a config declaring a key this tree does not know is still accepted silently, which is how the previous drift stayed invisible.
