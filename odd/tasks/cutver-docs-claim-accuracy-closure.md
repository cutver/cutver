# Feature: Documentation Claim-Accuracy Closure

Status: in progress
Branch: `docs/claim-accuracy-closure`
Owner skill: `.pi/skills/cutver-docs-voice` (evidence-first, scope every absolute, state the residual)

## Goal

Close the documentation claim-accuracy class opened by the 2026-10-11 sweep. Four rounds found
14 false claims in 8 documents; `b74599d` fixed the ones known then. This feature fixes the 5
findings that arrived after that merge, checks the two areas that were read but never
cross-checked against reality, and re-sweeps every absolute claim to confirm closure.

## Non-goals

- `README.md` line count: the user decided on 2026-10-11 to leave it at the budget limit (176).
- `docs/internals/references.md` over the 200-line ceiling: pre-existing debt tracked in #159.
- No behaviour change. In every known case the code is right and the prose is wrong.

## Findings to fix

| # | Claim | Truth | Files |
| --- | --- | --- | --- |
| F1 | Elixir `mix.lock` is a staged lockfile | `KNOWN_LOCKFILES` (`src/bump/exec/command.rs:18-31`) has no `mix.lock`; staging filters by exact filename, so a hook-regenerated `mix.lock` stays unstaged | `docs/reference/manifests.md:102`, `docs/internals/references.md:301` |
| F2 | `plugin add --dry-run` downloads nothing | It fetches the registry entry (`plan.rs:60` → `resolve_entry_with` → `fetch_entry_with` → `Transport::fetch`); only the artifact is skipped | `docs/reference/plugins.md:26`, `:71` |
| F3 | "Every mutation-capable run can be simulated end to end" | Only `bump` and `plugin add` have `--dry-run`; `init` and `plugin remove` do not (`src/cli/args/commands.rs:20-33`, `src/cli/args/plugin.rs:20-28`) | `docs/internals/design.md:40` |
| F4 | Staging covers "only the files `cutver` itself modified" | `run_post_bump_hook` also stages recognized lockfiles modified by the user's hook (`src/bump/exec/pipeline.rs:302-311`); the "never `git add .`" half holds (`src/git/ops.rs:7-32`) | `docs/internals/design.md:36-37` |
| F5 | "Every manifest editor is format-preserving ... minimal, single-line diffs" | Gradle also increments `versionCode`; the regex editor replaces every match. Same over-claim already fixed in `docs/reference/manifests.md:7`; the table two lines below says so | `docs/internals/references.md:105` |

## Tasks

- [ ] 1. Fix F1–F5 in the five documented locations, then sweep each claim across the repository for repeats.
- [ ] 2. Cross-check the never-verified areas against the code and the real artifacts: the out-of-repo artifacts (`cutver/setup`, `cutver/release`, the `ghcr.io/cutver/cutver` images) in `docs/integrations/{github-actions,containers}.md`, and the template-variable claims in `docs/templates/release-notes.md`.
- [ ] 3. Re-run the full absolute-claim sweep over `README.md` and `docs/**` and confirm closure (expect zero, do not stretch to agree with the expectation).
- [ ] 4. Land the new `cutver-docs-voice` skill as a work unit.
- [ ] 5. Verify budgets, link resolution and hype wording; record the residual.
- [ ] 6. Open the pull request.

## Evidence

| Task | Commit | Note |
| --- | --- | --- |
| — | — | — |

## Residual

- `docs/internals/references.md` (424 lines) stays over the ceiling; tracked as #159.
- `README.md` stays at 176 lines by explicit user decision.
