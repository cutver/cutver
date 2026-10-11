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
| — | — | — |

## Residual

- `[workspace]`, `glob` in `[[manifest]]` and Cargo workspace inheritance stay unimplemented and marked Planned until the RFC lands.
- Until #193 is fixed, a config declaring a key this tree does not know is still accepted silently, which is how the previous drift stayed invisible.
