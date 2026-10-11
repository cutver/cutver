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

- [x] 1. Fix F1–F5 in the five documented locations, then sweep each claim across the repository for repeats.
- [x] 2. Cross-check the never-verified areas against the code and the real artifacts: the out-of-repo artifacts (`cutver/setup`, `cutver/release`, the `ghcr.io/cutver/cutver` images) in `docs/integrations/{github-actions,containers}.md`, and the template-variable claims in `docs/templates/release-notes.md`. Eight more user-facing false claims found; fixed in `1b9a6e3`.
- [x] 3. Re-run the full absolute-claim sweep over `README.md` and `docs/**` and confirm closure. Not closed on the first pass: four more false claims (three user-facing, one over-absolute introduced by this feature itself) plus two repeats. Fixed in `1b9a6e3` and `a82bd0e`.
- [x] 4. Land the new `cutver-docs-voice` skill as a work unit.
- [ ] 5. Verify budgets, link resolution and hype wording; record the residual.
- [ ] 6. Open the pull request.

## Round 2 findings (after the two verification passes)

### Integration docs — written against a newer action manifest than `@v1` serves
The real `cutver/release@v1` outputs are exactly `released`, `version`, `tag`, `notes-path`, `release-notes` (`action.yml` at `v1`; `release/src/main.ts:28-32`). The guide documented `notes` (wrong name, twice, including the example workflow), `previous_version` and `previous_tag` (both nonexistent), and omitted `notes-path`. It also called `cutver/setup@v1` "Cosign-verified": the action has no verification step and `src/services/installer.ts:63-74` only calls `tc.downloadTool`. The release assets are signed by the release workflow; the install action does not verify them — a different claim that was never made true together.

### Container tags
`skopeo list-tags ghcr.io/cutver/cutver` publishes bare tags only (`main, latest, sha-*, 0.13.0, 0.13, 0, 0.13.0-plugins, 0.13-plugins, plugins, sha-*`). The guide documented `:v0.10.0` and `:v0` (neither exists) and claimed `:latest` "tracks the latest stable release", while `.github/workflows/container.yml:51` enables `latest` on `refs/heads/main` too — at the time of review it pointed at a `main` build ahead of v0.13.0.

### Plugin event vocabulary — the highest-impact find
`README.md:55` listed the `lifecycle.v1` events as `pre_bump` / `post_bump` / `post_release`. The code dispatches on the literal strings `on_pre_bump` / `on_post_bump` / `on_post_release` (`src/bump/exec/pipeline.rs:400,429,454`) and `plugins_for_event` compares with an exact `e == event` (`src/plugin/manager.rs:65`) against an unvalidated `events: Vec<String>`. A plugin written with the documented spelling silently never fires. No keyword sweep can find this class: the sentence contains no absolute.

### The rest
`configuration.md:5` claimed every release behaviour lives in `cutver.toml` at the root (CLI flags, `CUTVER_PLUGIN_CACHE_DIR`, and ancestor discovery all contradict it); `configuration.md:134` gave `template_file` precedence it does not have (`template` wins, `src/changelog/render/body.rs:27-38`); `design.md:40` — rewritten by this feature — said `plugin add` could be simulated "end to end", when its dry-run returns before the download and therefore skips the digest check (`src/cli/runner/plugin/add.rs:57`); the RFC 0001 Status still listed a registry as remaining work while `cutver/registry` is live. The closing sweep also caught two repeats of already-fixed claims: `README.md:113` ("every load") and `references.md:9` (the "at the root" framing).

## Evidence

| Task | Commit | Note |
| --- | --- | --- |
| 4 | `e5d6179` | `feat(skills): add the docs-voice claim-sweep skill` |
| — | `f9dd3d2` | `docs(odd): open the documentation claim-accuracy closure feature` |
| 1 (F1) | `9c5b8c0` | `docs(bump): drop the mix.lock staging claim the code does not honour` — `manifests.md:102`, `references.md:301` |
| 1 (F2–F5) | `89c6546` | `docs: scope the dry-run, staging and format-preservation claims to the code` — `plugins.md:27,71`, `design.md:36-40`, `references.md:105` |
| — | `5287074` | `docs(odd): record the claim fixes and the cli help follow-up` |
| 2 + 3 | `1b9a6e3` | `docs: correct the integration guides and the remaining overstatements` — 6 files, 12 false claims |
| 3 | `a82bd0e` | `docs: scope the last two repeats of already-fixed claims` — `README.md:113`, `references.md:9` |

Repeat sweep by the writer found one extra instance **outside** the allowed surfaces: the clap help text at `src/cli/args/plugin.rs:48` repeats F2 (*"Resolve and print the plan without downloading or writing anything"*). The `cutver-docs-voice` skill excludes CLI help text from its activation contract, so it is recorded as a follow-up rather than fixed in this docs-only change.

## Residual

- `docs/internals/references.md` (424 lines) stays over the ceiling; tracked as #159.
- `README.md` stays at 176 lines by explicit user decision.
