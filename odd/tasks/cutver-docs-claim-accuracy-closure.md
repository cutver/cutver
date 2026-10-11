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

## Round 4 findings

Round 4 ran the table-vs-struct method as the primary instrument and found the class the keyword rounds could not:

- **Documented defaults the code never applies.** `[version] strategy` was documented with default `"conventional"`; `default_strategy()` returns `"manual"` (`types.rs:123-125`). The `json` `field` and the `gradle` `version_name_field`/`version_code_field` were given "Default: ..." text while the serde fields carry no `#[serde(default)]` — they are required. `[preflight] default_timeout` advertised `600`; `parse_preflight` starts at `None` and applies no default (`config/preflight.rs:6,27`).
- **An incomplete value union.** `strategy` accepts a third value, `"plugin"` (`pipeline.rs:148`), with a `[version] plugin` key that no document mentioned.
- **The JSONC claim.** `manifests.md` and `references.md` promised `*.jsonc` support with comments preserved; both readers pre-validate with `serde_json::from_str`, which rejects comments.
- **`cutver notes`** does not exist and **`toml` is an alias**, not a generic kind; the design kind table omitted `pyproject` and `plugin` too.
- **Inert keys discovered while fixing.** `[hooks] pre_bump` is parsed and never executed (only `hooks.post_bump` is read, `pipeline.rs:279`); `[project] name` is parsed and unused. Both are now documented as such.
- **Completeness.** Eight `[changelog]` keys, `[hooks] pre_bump`, `[publish] default_timeout`, the `[project]` table, the plugin `command` key, `--json` for `changelog latest/show`, and `composer.lock` were added.

## Method

Four rounds used two methods with disjoint blind spots. The keyword sweep (`never|always|every|only|...`) cannot see a **wrong vocabulary** (the `pre_bump` vs `on_pre_bump` event names contain no absolute) and cannot see an **omission or an inert key** (a documented table the loader never reads). The complementary method — cross-check every documented table row, flag, tag, output and default against the struct or artifact that defines it — found the round-3 and round-4 classes. Round 5 makes the regression check of each newly written sentence the primary job.

## Round 3 findings

The closure sweep's keyword method found the wrong-vocabulary and omission classes that three earlier rounds missed:

- **`[workspace]` does not exist.** `Config` (`src/config/types.rs:41-63`) has no `workspace` field and there is no `deny_unknown_fields`, so the table is silently ignored. `docs/guides/monorepos.md:11-21` nonetheless documented two release models with an *Independent* row promising path-filtered commits and package-prefixed tags; `configuration.md` documented `mode`/`members` as real keys with defaults; `design.md`'s "Full Declarative Specification" listed them plus a `[[manifest]] glob` key that `Manifest` (`types.rs:158-166`) never deserializes. User decision: document the reality. Intent stays recorded at `design.md:170` as Phase 2.
- **The `plugin` manifest kind existed undocumented.** `ManifestKind::Plugin` (`types.rs:149-152`) is fully implemented (`src/manifest/mod.rs:71-105`) and no document named it; `manifests.md` listed 5 of 6 kinds.
- **`-plugins` suffix claim (introduced by `1b9a6e3`).** The registry publishes `0.13.0-plugins`, `0.13-plugins` and the floating `plugins`; there is no `0-plugins` and no `latest-plugins`.
- **Found while reviewing, still open:** `configuration.md:112,114-115` attach "Default: ..." to the `json` `field` and the `gradle` `version_name_field`/`version_code_field`, but the serde enum fields have no `#[serde(default)]` — omitting them is a parse error. Round 4 targets this class explicitly.

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
| 3 | `dd39702` | `docs: document the real workspace behaviour and the plugin manifest kind` — 6 files, `[workspace]` removed, `plugin` kind documented, `-plugins` suffix corrected |
| — | `84c311d` | `docs(odd): record the round 3 findings and the method change` |
| 4 | `7c6d215` | `docs: correct the documented defaults and fill the reference gaps` — 7 files, 9 false claims plus the completeness gaps |
| — | `9e77b4e` | `docs(odd): record the round 4 findings` |
| 5 | `ec08155` | `docs: scope the versioning strategy claim to its one implemented value` — 4 files; `strategy` is a plugin switch, and the missing `plugin` kind row entered at the file's `main` size |
| 6 | `6f0581b` | `docs: correct the pyproject table key and scope the legacy reference title` — `configuration.md:123` required→optional, `references.md:3` drops "complete" |

## Round 5 findings

- **`[version] strategy` is a plugin switch, not a bump strategy.** `grep -rn "\.strategy" src/` returns exactly one production read: `src/bump/exec/pipeline.rs:148`, `== "plugin"`. `"manual"` and `"conventional"` are read nowhere; `src/config/validation.rs:28-29` only substitutes the default `"manual"` for an empty value. The bump level comes from the CLI argument. Documented wrongly in `configuration.md:25,99`, `design.md:65`, and implied by the monorepo example. Fixed in `ec08155`.
- **`versionCode` on `--first-release`.** The row "incremented on every bump" is TRUE even on a first release: `resolve_next_version` returns the current version, but `GradleEditor::write_version` (`src/manifest/gradle.rs:57-71`) writes `code + 1` unconditionally with no equality guard. The false string is the CLI help at `src/cli/args/commands.rs:46`, which promises "without incrementing the version in manifests".
- **The last false claim:** `configuration.md:123` marked the `pyproject` `table` key as required. It is `#[serde(default)] Option<String>` (`types.rs:144-148`) and the editor auto-detects `[project]` then `[tool.poetry]` (`src/manifest/pyproject.rs:43-65`).

## Verification

Six passes, the last two read-only over the frozen tree. Every claim fixed in this branch carries the `path:line` that decides it in the sections above. The final pass re-verified every sentence the branch itself introduced (a regression check is where the `strategy` class surfaced) and reported the surface free of known false claims after the final commit.

## Residual

**Not verified — inference from source, not execution:**
- No test, build, lint or binary was run. Every verdict is static (`grep`/read). Runtime behaviour of plugin dispatch, atomic writes and the CLI parser was read from source, not exercised.
- The `plugin`-enabled build path was not exercised.
- Out-of-repo artifacts: `cutver/setup@v1` and `cutver/release@v1` sources were inspected earlier in the branch (`action.yml` at `v1`, `release/src/main.ts`) but not the full action repositories; container tags were confirmed with `skopeo list-tags`, not by pulling an image.
- `cutver-pdk` publication state (HTTP 403 when checked) — the RFC Status says so explicitly.

**Accepted debt:**
- `docs/internals/references.md` at 424 lines, over the 200 ceiling — pre-existing, tracked as #159. It is 424 on `main` too: this branch moved the `plugin` kind row in without growth.
- `README.md` at 176 lines by explicit user decision.
- `docs/templates/release-notes.md` documents 8 of the ~35 `ReleaseContext` fields — an omission, not a false statement.

**Follow-ups outside this docs-only change (issues opened):**
- `cutver.toml` accepts unknown keys silently (no `deny_unknown_fields`) — the root cause behind both the `[workspace]` phantom and the inert `hooks.pre_bump`.
- `[hooks] pre_bump` is parsed and never executed; only `hooks.post_bump` is read (`pipeline.rs:279`).
- Two `--help` strings promise behaviour the code does not implement: `src/cli/args/commands.rs:46` (`--first-release`) and `src/cli/args/plugin.rs:48` (`plugin add --dry-run`).
