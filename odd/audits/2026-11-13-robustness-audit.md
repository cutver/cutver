# Robustness & Solidity Audit — cutver

**Date**: 2026-11-13 · **Source**: `main` @ `2f8805b` (v0.18.1) · **Contract**: `CONTRACT.md` (180 lines, 6 pillars)

**Method**: three parallel read-only delegations (Pillars I/II/V; Pillars III/IV/VI; CI/CD gate
integrity) plus a mechanical pass on the local machine. Every P0 and P1 finding below was
re-read at its exact line by the auditor after the delegation returned. P2 findings marked
`[reported]` come from the delegated read-only pass and were spot-checked, not re-read in full.

**Local gates at audit time** (all green): `cargo build --locked --workspace --all-targets`,
`cargo test --locked --workspace`, `cargo clippy --locked --workspace --all-targets --features plugins -- -D warnings`,
`cargo fmt --all --check`.

## Status at commit

This document is the audit as performed on `2f8805b` (2026-11-13). It is kept verbatim as the
record; this block says what has since changed, so nobody implements an already-fixed finding.

| Finding | State |
| --- | --- |
| F-01 `main` unprotected | **Fixed** — #257, ruleset `main: required gates` (id 27202161) requiring `CI gate` + `Exactly one type label`, one bypass actor: the `cutver-release` app |
| F-07 tag collision fails open | **Open** — #262 |
| F-08 force tag ops / `push --tags` | **Fixed** — #263, one named ref per push, force only on the floating tag after success |
| F-02 container image | **Open** — #258, still broken; v0.18.2 shipped without an image |
| F-03 publish ordering | **Fixed** — #259, merged `157ea25`: `bump --defer-publish` stops after the branch push and `cutver publish` creates the tag, publishes the crate and opens the draft only after every smoke job is green; the container build pushes a rehearsal tag and the verified digest is promoted afterwards |
| F-04 `release-assets.yml` | **Open** — #260 |
| F-05 dead `Dockerfile` | **Open** — #258 |
| F-06 rollback writes `""` | **Open** — #261 |
| F-09 point of no return | **Fixed** — #265, merged `7019e02`: the tag name is validated in Phase 1 before any write, and the five post-commit errors name the landed commit and assert no remote state at all |
| F-10 / F-11 plugin authority | **Open** — #264 |
| F-12…F-20, pre-registered 2 | **Open** — #266…#272, #268 |

Two facts from this audit were later corrected by measurement, and the corrections are worth keeping:
the release push runs **before** `cargo publish` (`pipeline.rs:116` vs `:117`), so a rejected push does
not reach the registry; and the bump commit was already authored by `cutver-release[bot]` — the fix
changed the **pusher** identity, not the author.

One half-fix is worth naming: F-15 says `TagPrefix::format_tag` hands out a name that was never
validated. #265 closes that on the release path — the name is checked before anything is written —
but the type still allows it, so the defect stays open in #267 until `format_tag` validates at
construction.

## Verdict

The contract is honored where the code is a library, and violated where the code meets the
outside world. Production `unwrap`/`expect`/`panic!` count is **0**; newtypes, surgical editors
and fail-closed command handling are real. What breaks is the boundary: the order in which
irreversible side effects are taken, the gates that are supposed to stop a bad artifact, the git
operations that destroy history, and the I/O reads whose errors are converted into empty values.

Severity counts: **5 P0**, **6 P1**, **9 P2**.

---

## P0 — release and delivery integrity

### F-01 `main` has no protection and no rulesets (pre-registered finding 1)

Evidence: `GET /repos/cutver/cutver/branches/main/protection` → 404 (`Branch not protected`);
`GET /repos/cutver/cutver/rulesets` → `[]`. All eight checks produced on a PR are advisory:
`Test (ubuntu-latest|macos-latest|windows-latest)`,
`Test with plugins feature (…)`, `Publish dry-run`, `Exactly one type label`.

Failure mode: every gate in `.github/workflows/ci.yml` can be merged red. Three PRs merged in a
single session with no enforced gate.

Note on the required-check set: `pr-labels.yml:7` deliberately omits `opened`, so a fresh PR never
reports `Exactly one type label`. Requiring it as-is deadlocks every new PR in *Expected*.
Required set for a ruleset, today, without restructuring:
`Test (ubuntu-latest)`, `Test (macos-latest)`, `Test (windows-latest)`,
`Test with plugins feature (ubuntu-latest)`, `Test with plugins feature (macos-latest)`,
`Test with plugins feature (windows-latest)`, `Publish dry-run`.
Preferred alternative: one umbrella `CI gate` job (`needs:` the three job ids, `if: always()`,
asserting each `needs.*.result == 'success'`), then require only `CI gate`. Per-matrix required
names rot: deleting a matrix entry leaves a permanently-expected check that blocks every merge.

### F-02 The container image has been broken for three consecutive releases (new)

`Container Image` failed on v0.17.0 (`43221065023`), v0.18.0 (`43350724644`) and v0.18.1
(`43358205310`); the last success is v0.16.0. Both jobs fail in the image build:

```
error: couldn't read src/init/scaffold/../../../.github/templates/cutver/RELEASE.md
       (os error 2)
```

Root cause is a build-context invariant, not a Docker bug:
`src/init/scaffold/templates.rs:15` has `include_str!("../../../.github/templates/cutver/RELEASE.md")`,
while `Containerfile:21-43` only `COPY`s `Cargo.toml`, `Cargo.lock`,
`crates/cutver-pdk/{Cargo.toml,src}`, `src` and `crates/cutver-pdk/src`. `.github/` never enters
the builder. `.dockerignore` compounds it twice: `.github/` (line 4) and `*.md` (line 7).

Recurrence is the point: the `Containerfile` already carries a comment describing the same class of
failure ("*an error that only a release event would have surfaced*", from the missing
`crates/cutver-pdk` manifest). The image build is exercised only on `release: published`, so every
new compile-time file dependency breaks it again and nobody sees it until the next tag.

Fix, both halves:
1. `Containerfile`, before the real source build, after the dependency cache layer:
   `COPY .github/templates/cutver/RELEASE.md ./.github/templates/cutver/RELEASE.md`
2. `.dockerignore`: drop `.github/` and add `!.github/templates/cutver/RELEASE.md` after `*.md`.
   Relying on a negation under an excluded parent directory alone is not reliable.
3. Prevention: a `pull_request` image build with `push: false`, `platforms: linux/amd64`,
   `cache-from: type=gha`. This is the only gate that would have caught all three failures.

### F-03 Irreversible publish happens before any artifact is built or verified (new)

`cutver.toml:47 push = true` and `cutver.toml:54 commands = ["cargo publish"]` execute inside
`release.yml`'s `version` job (`release.yml:101`, `cutver/release@v1`). `build`, `build-plugins`,
`smoke` and `smoke-plugins` are later jobs. A failing build or smoke therefore leaves: a tag pushed
to origin, and a crate on crates.io, with no release and no way to unpublish.

The repository already reasoned about this hazard in one place and not the other:
`cutver.toml`'s `[plugins.github-publish.config] draft = true` comment says a false default
"*publishes a public, asset-less release during the version job, before the binaries are built*".
The same inversion is still live for `cargo publish`.

Same shape in the image pipeline: `container.yml:63,117` push with `push: true`, while the
`smoke`/`smoke-plugins` jobs run afterwards (`needs: build-and-push`, `:127,177`). A failing smoke
leaves a broken `:latest` already published, with no retraction step.

### F-04 `release-assets.yml` backfill rewrites the release the plugin authored (new)

The backfill duplicates `release.yml:123-334` and `:495-565` but stops at `build`/`build-plugins`
(`release-assets.yml:229 needs: [build, build-plugins]`) — it runs no smoke jobs, no brew/scoop
steps and no install smoke. It also uses `softprops/action-gh-release@v3` with
`name: Release ${{ inputs.tag }}` (`:272-276`), while `release.yml:588-591` documents that
repeating title/body "*gave the release a second owner — that is how `title_template` once came out
dead*" and `cutver.toml` sets `title_template = "cutver {version}"`. Running the backfill after a
healthy release rewrites the title and body that the plugin rendered.

### F-05 Dead duplicate `Dockerfile` (new)

`Dockerfile` and `Containerfile` are both tracked; `container.yml:61,115` use `./Containerfile`.
`Dockerfile` predates the workspace member and the feature arg: it lacks the
`crates/cutver-pdk` manifests and `ARG CUTVER_FEATURES`. It is an active decoy — editing it changes
nothing. Pillar V.6.

---

## P1 — data loss and destructive git

### F-06 Rollback writes an empty string over a file it never read

`src/bump/exec/manifest.rs:138`:
`let original = std::fs::read_to_string(m.path.as_str()).unwrap_or_default();`
An existing-but-unreadable file (permissions, invalid UTF-8, transient `EIO`) becomes `""`, that
`""` is recorded as the backup (`:158/:167`), and `MutationTransaction::drop` writes it back
(`transaction.rs:41`). A later failure in stage, commit or a hook turns a real manifest into a
zero-byte file. An absent file becomes a created empty file on rollback.

The rollback itself discards its own failures: `transaction.rs:41` and `:50` are `let _ = …`, so a
failed restore leaves a half-mutated tree reported as only the original error. `Summary` has no
field to carry it. `doctor.rs:92` repeats the same read pattern.

### F-07 Tag-collision check fails open

`src/git/tags.rs:36-40`: after `ls-remote`, `if !output.status.success() { return Ok(None); }`.
An unreachable remote (DNS, expired token, offline CI) reads as "no remote tag". The release then
writes, commits and tags locally, `git push` fails, and the retry aborts with `TagExists` — the
wedge that Pillar II.2's fail-closed rule exists to prevent.

### F-08 Force tag operations are core features, not guarded escapes

`src/git/ops.rs:151` `+refs/tags/{tag}:refs/tags/{tag}` (force refspec) and `git/ops.rs:185`
`git tag -f -a` (blind overwrite), reached from `pipeline.rs:384` / `handle_floating_tag`.
`git/ops.rs:130` additionally pushes with `--tags`, publishing every local tag instead of the
release tag. Pillar II.2 forbids force operations on shared history; these are unconditional.

### F-09 The point of no return is mid-pipeline

`src/bump/exec/pipeline.rs:108` `transaction.commit()` runs before `git::tag` (`:109`), the floating
tag, both pushes, `run_publish_commands` and `run_plugin_post_release`. Each can fail (invalid tag
prefix, signing failure, `cargo publish` error, plugin timeout) and returns `Err` with the tree
committed and possibly tagged. No compensation, no message naming what is now half-applied.
The two-phase split is enforced only by statement order in a 100-line function: nothing in the type
system (`MutationTransaction` is a bool, `transaction.rs:10`) prevents a write from moving earlier.

### F-10 Plugin `permissions.env` is decorative on the process runtime

`src/plugin/driver/process.rs:70-86` uses `Command::envs(&self.env)` with no `.env_clear()`, and
`src/plugin/manager/factory.rs:74` passes an empty map. The child therefore inherits the **entire**
parent environment: `GITHUB_TOKEN`, `CARGO_REGISTRY_TOKEN`, everything. `release.yml:111` asserts
the opposite in a comment ("*these are the only environment keys the plugin sees*"); the WASM
runtime honors that sentence, the process runtime does not. Pillar VI.2, zero ambient authority.

### F-11 Process plugin stdin write blocks before the timeout can start

`src/plugin/driver/process.rs:110-118`: `write_stdin()` performs blocking `write_all`/`flush`
before `wait_with_timeout` begins polling. A plugin that never reads stdin stalls on the ~64 KB pipe
buffer indefinitely; `timeout_seconds` is never consulted. The release pipeline hangs instead of
failing closed. Pillar VI.3.

---

## P2 — swallowed errors, duplicated authority, structure

1. **F-12 Four `Result` reads become empty values** (Pillar III.4). `bump/exec/manifest.rs:138`,
   `bump/exec/doctor.rs:92`, `cli/runner/changelog.rs:39` and `:75`. The last two make
   `cutver changelog latest|show --json` emit an empty `raw_commits` with exit 0 when git fails;
   the doctor ones turn an I/O error into template drift. Also `process.rs:156-157`
   (`join().unwrap_or_default()` → truncated stdout reported as a payload error) and
   `driver/wasm.rs:130` (an unset declared env grant is injected as `""`).
2. **F-13 Three tag/version-prefix normalization authorities** (V.6). `git/types.rs:150`
   (prefix-aware) vs `changelog/extract/parser.rs:133` (hardcoded `v`/`V`) vs inline
   `trim_start_matches(['v','V'])` in `cli/runner/changelog.rs:56,60` and
   `cli/runner/changelog_context.rs:29,38,52,69,86`. With `tag_prefix = "release-"`,
   `changelog show`/`open` never match a heading.
3. **F-14 Config resolution duplicated 5×, all through `.ok()`** (V.6, verified 4 `.ok()` sites).
   `config::load(p).ok()` means `--config broken.toml` silently falls back to CWD discovery, and
   path resolution and context assembly can read different roots in one invocation.
4. **F-15 `changelog.path` and `tag_prefix` bypass their newtypes** (II.3/V.5).
   `config/types.rs:225` `Option<String>`, `:298` `String`; the raw path reaches `git add`
   (`bump/exec/changelog.rs:289`) unnormalized, and `TagPrefix::format_tag` builds a `TagName`
   without `validate_git_tag_name` — the invalid tag surfaces only after the commit exists (F-09).
5. **F-16 Hook spawning is triplicated and the hook copies have no timeout** (V.4/V.6).
   `preflight.rs:120-186` and `bump/exec/command.rs:96-186` carry the `setpgid`/`killpg` machinery;
   `pipeline.rs:300-311` and `:330-341` spawn shells with `.status()` and no timeout, no kill tree.
   A hanging `[hooks] pre_bump`/`post_bump` blocks cutver forever; `post_bump` additionally leaves
   the transaction half-applied with a live child.
6. **F-17 Wall clock inside the render core** (I.1). `changelog/update.rs:91,97`,
   `changelog/render/body.rs:66,99`, `changelog/context/assemble.rs:299`, `pipeline.rs:244`.
   The heading and `{{date}}` can disagree across a midnight crossing inside the same render.
7. **F-18 Documentation budget exceeded** (IV.5). `CONTRACT.md:114` sets a strict ceiling of
   ≤ 200 lines; `README.md` is **426**. `docs/**` is compliant. No gate enforces the budget.
8. **F-19 Test isolation** (IV.1). `tests/common/mod.rs:79-86` mutates the process-global CWD under
   a lock that only serializes other guards, and `Fixture::new` (`:25-31`) uses
   `std::env::temp_dir()` instead of `tempfile::tempdir()`. Date assertions compute
   `format_date(SystemTime::now())` after the binary rendered its own (local-midnight flake).
9. **F-20 Deprecation contract has no implementation** (IV.2). Zero `deprecat*` in `src/`;
   `reject_unknown_keys` (`config/validation.rs:196-215`) makes every renamed key a hard error;
   `doctor` has no warning channel.

---

## Pre-registered finding 2 — grant diagnostics (reframed)

`cutver doctor` prints four fixed rows (`cli/doctor.rs:13-32`: Configuration, Manifests, Preflight,
plus Changelog with `--check-changelog`). It prints **nothing** about plugins, capabilities or
`permissions.filesystem` grants.

The problem is larger than a missing print. `PluginConfig.permissions.filesystem: Vec<String>`
(`plugin/types.rs:166`) is **overwritten in place** with the resolved guest path by
`resolve_plugin_filesystem` (`config/discovery.rs:47-63`), so the declared spelling is destroyed
before anything downstream could print it. The canonical host path exists only as a local in
`WasmDriver::build_manifest` (`plugin/driver/wasm.rs:123-126`) and is discarded; `resolve_grant`
(`config/discovery.rs:70-85`) holds `declared`, `resolved` and `real` together and drops `real` on
success (`:80`). The only struct that ever carries both is the error variant
`ConfigError::GrantOutsideWorkspace` (`config/types.rs:18-25`) — error path only.

So finding 2 is a data-retention fix (keep declared alongside resolved, return the map from
`resolve_plugin_filesystem`) plus a `render_plugin_permissions` surface in `doctor`.

## Pre-registered finding 3 — native reviewer precision

No reviewer or judge model configuration exists inside this repository: `.pi/` holds only
`skills/cutver-docs-voice/`, `.github/` holds CI and release workflows, `.agents/` holds project
skills. The reviewer is configured at harness level, outside the project root, and reading or
editing it requires explicit per-path authorization.

Standing process rule until it improves (unchanged): never implement a `correction_required`
finding without first refuting it against the dependency source and the CI gates, and record the
tally in the PR. Tally from #251: 5 lineages, 22 model runs, 12 findings, 3 real, 9 false.

---

## Remediation plan — ordered work units

Order is risk reduced per line changed. WU1–WU3 need no product decision; WU4–WU7 are source
changes that each fit one reviewable commit.

| WU | Scope | Findings | Size |
| --- | --- | --- | --- |
| WU1 | Ruleset on `main` + umbrella `CI gate` job + restore `opened` on the label check | F-01 | small, no source |
| WU2 | PR-time image build (`push: false`), `Containerfile` COPY, `.dockerignore` negation, delete dead `Dockerfile` | F-02, F-05 | small |
| WU3 | Move tag push and `cargo publish` after the smoke jobs; push the image only after its smoke | F-03 | medium, config + workflow |
| WU4 | Rollback safety: fail closed on unreadable manifest, record rollback failures in `Summary` and stderr | F-06 | medium |
| WU5 | Destructive git: `ls-remote` fail-closed, force-tag behind an explicit flag, single-refspec push instead of `--tags` | F-07, F-08 | medium |
| WU6 | Plugin authority: `.env_clear()` + explicit grants, stdin write under the timeout | F-10, F-11 | medium |
| WU7 | `doctor` declared→real grant map (retain declared paths through resolution) | pre-registered 2 | medium |
| WU8 | Single-source version normalization + single config resolution | F-13, F-14, F-15 | large, refactor |
| WU9 | Retire `release-assets.yml` or re-derive it from `release.yml` | F-04 | medium |
| WU10 | Error-swallow sweep, hook timeout parity, test isolation, README decomposition | F-12, F-16…F-20 | large |

## What is healthy

Pillar III.2 (0 production `unwrap`/`expect`/`panic!`; the single `unreachable!` at
`config/validation.rs:213` is guarded by `ROOT_KEYS`), III.1/I newtypes parse at every boundary,
III.3 rich What/Where/Fix errors, II.1 surgical editors (`toml_edit`, span-scanned JSON, line
endings preserved), I.3 no git or write under `rayon`, IV.3 clippy/`-D warnings`/fmt with no
`continue-on-error`, V.2/V.3 exhaustive `ManifestKind` match plus trait-dispatched editors, VI.1 one
`PluginInvocation` DTO for both runtimes, VI.4 optional dependencies keeping the lean build real.
