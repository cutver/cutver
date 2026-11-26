# Shipping cutver's own prereleases — the channel in this repository's pipeline

Status: **prepared, not started.** Queued behind `odd/tasks/phase5-debt-payment.md` and gated on four
maintainer decisions (D2, D3, D4 and D6 below). Branch to create when it starts: `feat/own-channel`.
Base: `main` at `cec818e`. Scope: make this repository's pipeline pass `--channel` and survive a
prerelease end to end, so `0.22.0-alpha.1` is the first non-stable version cutver ever ships.

## Specs

- **S1.** «Para cuando vayamos a simplificar el workflow de este repo como lo dejamos guardado también
  implementaremos los nuevos canales para ahora sí poder lanzar versiones alpha, betas, rc y cuando esté
  todo pulido, las stables» — one workstream, not two: the workflow simplification and the real channels
  are the same move, because a channel is only proven when this repository's own pipeline uses it.
- **S2.** «hasta la v0.21.0 que lanzamos hace unos días todas son stables» — verified: `Cargo.toml:3` is
  `0.21.0`, `CHANGELOG.md:6` is `v0.21.0`, and no prerelease exists anywhere in the history. The first
  alpha is therefore the first exercise of every downstream consumer at once, which is why S3's ordering
  matters more than the version numbers.
- **S3.** «cuando esté todo pulido, las stables» — the ladder ends in a promotion, not a new verb:
  `cutver bump --channel stable` on `0.22.0-rc.1` yields `0.22.0` (ADR-0005, phase 5's S1). No task here
  invents a `promote` command, a state machine, or a channel that outlives the version it produced.
- **S4.** The recorded plan is issue #326, «feat(ci): make the release pipeline an artifact instead of a
  workflow», and its own disclaimer is binding on how "simplify" may be read — `.git/issue-ci-native.md:98`
  (untracked local copy of the body): «**Making this repository's workflow short.** The workflow stays the
  reference consumer; shortening it is evidence that the phases landed, not a deliverable in itself.»
  So the workflow may shrink only where a landed phase took over a step, never to reach a line count.
- **S5.** The size of the thing being simplified: `.github/workflows/release.yml` is **1041 lines with 64
  `- name:` steps** (issue #326, `## What`), and the pipeline facts it encodes — artifact matrix, `needs:`
  gate order, promotion, channel tags, retry — are exactly what #326 wants inside the tool. Gaps 1, 3, 4
  shipped; 6 and 7 shipped as phase 5; **2, 5, 8 and 9 are still "not verified in this pass"** (#326's own
  status table), so no task below may assume them.
- **S6.** Irreversibility is named before it is touched: a crates.io version can be yanked but never
  deleted, `v0` and container `latest` are force-moved refs, and the GitHub release draft becomes the
  repository's *Latest* when promoted. Every task that touches one of those three says what happens on a
  prerelease before it runs.
- **S7.** Non-goals, unchanged from phase 5's S5: no CI provider SDK, no HTTP client in the lean build,
  stdio/exit codes/files only. Nothing here adds a dependency, and nothing here implements gaps 2, 5, 8 or
  9 beyond what a real prerelease forces.

- **S8.** «Otra cosa, de los gestores de paquete, scoop, homebrew y winget que tenemos que planificar el
  lanzamiento de cutver allá, creo que hace más sentido solo tener la versión con plugins habilitado» — and
  the follow-up answer «`cutver` = plugins, con shim deprecado una release». One build per package manager
  (the plugins one), under the name `cutver`; `cutver-plugins` stays one release as a deprecated shim and
  then goes; the lean build keeps shipping as a release asset and keeps its smoke; winget is a new surface
  and inherits the same rule.

## Tasks

Order is deliberate: the decisions gate the code, the tool-level rules come before the workflow consumes
them, and the first real prerelease is observed before anything downstream is simplified.

- **T1 — Correct #326's status table before planning from it.** (S5) Gaps 6 and 7 are shipped as of
  `cec818e` (`--channel` + promotion: `src/cli/args/commands.rs:35-47` is where the flag now lives; resume:
  `src/bump/exec/publish/resume.rs`), and phase 5 is the chain #391-#396 plus #397. Rewrite the table rows
  for 6 and 7 with that evidence, keep the reversal as the record (the body already does this for ADR-0004
  → ADR-0005), and leave gaps 2, 5, 8 and 9 marked unverified. Ask before editing the issue; the corrected
  text is what makes the rest of this document plannable by someone who only reads #326.
- **T2 — Decision batch, before any code.** (S3, S6) Four answers, each with a recommendation in
  `## Decisions`. If the maintainer accepts all four, T3 onward runs without another question.
- **T3 — `channel` reaches the bump step.** *(done, L9)* (S1) `release.yml:4-20` declares only `bump` and `dry_run`;
  the call is hardcoded at `release.yml:146` with `args` built at `:141-145` (only `--defer-publish` /
  `--dry-run`). Add a `channel` choice input (empty = current behaviour, then `alpha|beta|rc|stable`) and
  append `--channel "$CHANNEL"` only when it is set, so an untouched dispatch is byte-identical to today.
  The contract tests pin the old shape literally and must move with it:
  `tests/release_pipeline_contract.rs:184,387` (the exact `cutver bump "${{ inputs.bump || 'auto' }}"`),
  `:381-395` (requires `args=(--defer-publish)` verbatim and `"${args[@]}"` on the call line), `:249-255`
  (`inputs.dry_run` guard), `:331-372` (the `released=` grep and its byte-exact error text). Test-first:
  a new assertion that a dispatch with `channel: alpha` produces `--channel alpha`, and one that an empty
  channel produces the current argv unchanged.
- **T4 — The tool reports the channel it produced.** (S6) `cutver bump` writes exactly
  `released, deferred, version, tag, commit` (`src/cli/ci/fields.rs:17-25`; `plan` at `:28-34`) and no
  `channel` key, so every downstream job would have to re-parse the semver suffix. Add `channel` to the two
  field lists and to the contract's expected set at `tests/release_pipeline_contract.rs:408`
  (`WRITTEN: [released, deferred, version, tag, commit]`), which is the test that keeps the workflow from
  reading a key the tool never writes. No `--json` change beyond an additive key (phase 5's D3 stays
  intact: `plan --json` keeps `touched[]` as the transition's authority).
- **T5 — A prerelease is never Latest, and the promotion says so itself.** (S6) The plan first claimed that
  "a published alpha would become Latest". **That premise is false, measured on 2026-11-25**, and T5 survives
  in a better shape. GitHub guarantees the property at the API: "The latest release is the most recent
  non-prerelease, non-draft release" and "Drafts and prereleases cannot be set as latest"
  (`docs.github.com/en/rest/releases/releases`). And the plugin already marks the draft from the version:
  `plugins/github-publish/src/post_release.rs:78-80` reads
  `config.prerelease.unwrap_or_else(\|\| is_prerelease(&payload.version))`, pinned by the plugin's own
  `automatically_marks_prerelease` test at `:322-339`. What remains is a **coupling**, not a certainty: the
  property holds only while nobody sets `prerelease = false` under `[plugins.github-publish.config]` in
  `cutver.toml` and while the plugin keeps deriving the flag. T5 makes the guarantee independent of both —
  the promotion step passes the channel explicitly (`--draft=false --prerelease=false --latest=true` for
  stable, `--draft=false --prerelease --latest=false` otherwise), the contract test pins that exact line and
  the branch that chooses it, and a test asserts that `cutver.toml` declares no `prerelease` key, with the
  reason in the failure message. Constraints to respect: exactly two `GH_TOKEN:` and two `TAG:` lines, and
  the existing `assert_eq!` on the promotion string.

- **T6 — The floating major tag does not follow a prerelease.** (S6) `cutver.toml:22` sets
  `floating_major_tag = true`, `src/bump/plan.rs:301` decides it with no channel condition, and
  `src/bump/plan/resolve.rs:54-56` force-moves `v0`. On `0.22.0-alpha.1` that re-points `v0` at a version
  nobody should get by default. Make the floating major a **stable-only** effect for a prerelease target,
  report the skip in the plan tree the way the no-op is reported (phase 5's `No-op` section is the
  precedent), and pin it: `plan minor --channel alpha` on `0.21.0` must not print a floating tag line and
  must not move `v0`. This is a behaviour change to a shipped flag → ADR-0005 gains one bullet, or a new
  ADR if the maintainer prefers; do not decide it in a commit message.
- **T7 — Container tags stop lying about `latest`.** (S6) `container.yml:4-5` triggers on
  `release: published` with no prerelease filter and the rules at `:303-308` emit `{{version}}`,
  `{{major}}.{{minor}}`, `{{major}}` and `type=raw,value=latest` for any release (the plugins image opts
  out of `latest` at `:328-331`). A prerelease must publish `{{version}}` only. Gate the rules on the
  release's prerelease flag, and pin it in whatever contract test covers `container.yml` (check
  `tests/ci_workflow_contract.rs:24-25` before assuming there is one — it currently pins the three CI legs,
  not the container tags).
- **T8 — Homebrew and Scoop wait for stable, and publish one package.** (S6, S8) `release.yml:802-808` builds formula URLs from
  `${TAG#v}` and `:957` does the same for Scoop, both gated only on a token being present. An alpha would
  be published as the tap/bucket version. Gate both on the channel, and resolve the unknown first: how the
  tap derives `version` from `cutver-0.22.0-alpha.1-macos-arm64.tar.gz` (not observable from this repo;
  `odd/tasks/cutver-homebrew-tap-automation.md:19-21` records the steps as tag-driven). Then, per D8 and S8:
  the plain `cutver` formula and the Scoop manifest install the **plugins** asset, `cutver-plugins` becomes a
  deprecated shim for exactly one release and is removed in the next, and the four pages that describe the
  two builds (`docs/guides/installation.md` and the three others that name `cutver-plugins`) say one name
  and how to get the lean binary if that is what you want.

- **T12 — winget, from nothing.** (S8) No file in this repository mentions winget: it is a new delivery
  surface, not a modification. Ship a single `cutver` package built from the plugins asset (portable zip),
  stable-only by D3, with the version, installer URL and SHA-256 taken from the same `SHA256SUMS` the tap
  reads. The manifest lives in `microsoft/winget-pkgs`, so the update is a pull request with its own
  validation pipeline and is **not** synchronous with a release: decide and record whether it is a workflow
  job with a token (the same shape as the tap) or a documented per-release command, and pin the manifest
  fields with a test the way `tests/release_pipeline_contract.rs` pins the workflow.
- **T9 — crates.io: decide, then execute.** (S6) `cutver.toml:57` runs `cargo publish` inside
  `cutver publish`, so **every** prerelease run burns a version permanently. The two honest options are in
  D4. Whichever wins, `publish-crate.yml:38` needs no change (it seds any version string and lets the API
  refuse a duplicate), and the refusal path must be observed once so a re-run of a doomed alpha fails with
  a readable message rather than a mystery.
- **T10 — Cut `0.22.0-alpha.1` for real, then `beta.1`, then `rc.1`, then promote.** (S2, S3) Four
  dispatches: `dry_run=true` first to read the plan for `bump=minor channel=alpha`, then the real one, then
  a patch-level alpha, then `--channel beta`, then `--channel rc`, then `--channel stable` to promote
  `0.22.0-rc.N` to `0.22.0`. Record each run verbatim in `## Log`, the way phase 5's L11 recorded the
  promotion in a scratch repository — except this time on the real repository, and say plainly what each run
  published to crates.io, GitHub, GHCR and the tap.
- **T11 — Simplify only what a landed phase took over.** (S4) After T10, delete a job or a step only when
  the tool now owns it, and cite the phase that landed it in the commit message. Each deletion is its own
  commit and its own test update, because `tests/release_pipeline_contract.rs:20` (`JOBS`),
  `:22` (`VERIFIED_BEFORE_PUBLISHING`), `:571-585`, `:617-650` and `:710-846` pin the current shape
  literally. If nothing can be deleted honestly, this task reports "nothing yet" — that is a valid outcome
  and preferable to a shorter YAML that proves nothing.

## Decisions

Recommendations, not accepted decisions. T2 exists because four of them are genuinely the maintainer's.

- **D1 — Order.** Phase-5 debts first (`odd/tasks/phase5-debt-payment.md`, T1-T3 are cheap and independent),
  then this document from T1. T6 and T5 are the two that change shipped behaviour; they come before the
  workflow consumes the channel, because a workflow that passes `--channel` while `v0` still follows alphas
  would publish a broken default on the first run.
- **D2 — Version choice: `0.22.0`, prereleased as `alpha → beta → rc → stable`.** Not a patch on `0.21`;
  the pipeline change is a feature (`.github` surface plus a tool behaviour change), so `minor` is the
  honest level, and the ladder is what proves promotion. **Accepted 2026-11-25** (:«de ahora en adelante
  las versiones las lanzaremos con escaleras»).
- **D3 — The floating major tag, container `latest`, the Homebrew tap and the Scoop bucket are
  stable-only.** A prerelease publishes exactly its own version everywhere. This is the only rule that keeps
  `cutver/setup`'s `version: latest` default from installing an alpha, and it is why T5-T8 are one decision
  rather than four. **Accepted in full on 2026-11-25**: Scoop first («a scoop solo estables»), then all
  four surfaces — a prerelease publishes exactly its own version everywhere. See L8.
- **D4 — crates.io: publish the prereleases.** Recommended, for one reason: the crate is the surface under
  test, and a channel that skips it would leave `cargo publish` inside `cutver publish` unexercised exactly
  where `--resume`'s attestation logic lives. The cost is named in S6 — a broken alpha can be yanked, never
  deleted. The alternative (skip the crate step on prerelease, ship binaries only) is cheaper to undo and
  proves less; if the maintainer picks it, T9 becomes a gate in `cutver.toml` and D4 is rewritten.
  **Measured viable on 2026-11-25** — the evidence is L8 — and **accepted the same day**: T9 publishes
  `cargo publish` output on every channel, permanence cost included.
- **D5 — Simplification is evidence, never a line-count target.** S4 quotes #326's own words; T11 is
  therefore allowed to end with "nothing to delete yet". A workflow shrunk by guessing is how a release
  loses a gate.
- **D6 — The issue body is corrected before the plan is planned from.** T1 first: right now #326 says gaps
  6 and 7 are open, which is false, and a plan built on a false table is the defect phase 5 spent L3
  fixing once already.

## Unknowns to resolve before T5, T8 and T9

Each is one measured answer away, and none needs a decision:

1. **Does crates.io treat `0.22.0-alpha.1` as the crate's newest version?** Resolve by publishing a
   prerelease of a throwaway crate, or by reading the registry's documented semantics — do not assume.
2. **Does `github-publish-0.2.1.wasm` accept a prerelease/latest parameter?** Its source lives in
   `cutver/plugins`, outside this repository (`cutver.toml:66-94` pins the digest). Resolve by reading the
   plugin's manifest/PDK surface, not by guessing from this side.
3. **How does the tap derive `version` from a prerelease tarball name?** Not observable here; the answer
   decides whether T8 is a gate or a rename.

## Log

**L1.** «Para cuando vayamos a simplificar el workflow de este repo como lo dejamos guardado también
implementaremos los nuevos canales para ahora sí poder lanzar versiones alpha, betas, rc y cuando esté todo
pulido, las stables, porque hasta la v0.21.0 que lanzamos hace unos días todas son stables»

**L2. What "como lo dejamos guardado" resolves to.** Not a document in `odd/tasks/`, not an ADR, not a
memory entry: the recorded plan is **issue #326's own body**, «feat(ci): make the release pipeline an
artifact instead of a workflow», whose local copy is the untracked `.git/issue-ci-native.md`. Its `## What`
carries the measurement this document's S5 repeats (1041 lines, 64 steps) and its `## Sequencing` is the
phase list. `odd/tasks/cutver-issue-258-260-release-surface.md:41-43` records the opposite, older decision —
delete a second workflow rather than fold one into a reusable one — so nothing in the tree ever promised to
shrink `release.yml` by itself.

**L3. The tension this document had to resolve, and how.** #326 explicitly says the workflow "stays the
reference consumer" and that shortening it "is evidence that the phases landed, not a deliverable in
itself" (`.git/issue-ci-native.md:98`), while the maintainer's request names simplification as a goal. Both
are kept: the request sets the destination, #326's disclaimer constrains the method, and D5 makes T11
report "nothing yet" as a valid result. Nothing here deletes a gate to look tidy.

**L4. The nine measured blockers that separate `--channel` from a real alpha** (explorer sweep against
`cec818e`, each with the line that proves it): no `channel` dispatch input (`release.yml:4-20,141-146`); no
`channel` step output (`src/cli/ci/fields.rs:17-25`); the promotion step marks nothing as prerelease and
nothing as not-latest (`release.yml:787`, pinned by `tests/release_pipeline_contract.rs:679-697`); the
floating major follows any target (`cutver.toml:22`, `src/bump/plan.rs:301`, `src/bump/plan/resolve.rs:54-56`);
container rules emit `latest`/`major` for any release (`container.yml:4-5,303-308,328-331`); Homebrew and
Scoop are unconditional (`release.yml:802-808,957`); `cargo publish` runs on every publish (`cutver.toml:57`);
and two contract test files pin the workflow's current shape literally, so any change lands with them
(`tests/release_pipeline_contract.rs`, `tests/release_metadata_ownership.rs:50-77,114`).

**L5. What is already channel-agnostic, so no task wastes a line on it.** Changelog headings survive a
prerelease (`src/changelog/update.rs:97`, parser prefix-strip at `src/changelog/extract/parser.rs:136-153`,
`changelog latest` at `src/cli/runner/changelog.rs:80-90`); `doctor --check-changelog` is string-based
(`src/bump/exec/doctor.rs:139-152`); the notes path is tag-driven (`release.yml:184`, `.github/templates/`);
`scripts/check.sh` parses no version; `ci.yml` has no tag trigger; `publish-crate.yml` and `publish-pdk.yml`
need no change; `tests/source_line_budget.rs` governs `src/**`, not YAML.

**L6. What «continuemos» must be read as, for this document.** «continuemos» still means the phase-5 debts
first (`odd/tasks/phase5-debt-payment.md`). This document starts at T1 only when the maintainer says so
explicitly, or after the debts close — and T2's four answers are owed before T3 writes a line of workflow.

**L7.** T1 done on 2026-11-25, by maintainer authorization («crea el issue» → "Corregir la tabla de estado
del #326"), and it went further than the task asked because the evidence arrived with the phase-5 debt
branch (#398): the table now marks gaps **2 and 5 shipped** (they were "not verified in this pass"), **6 and
7 shipped in phase 5**, and **8 and 9 open**, with `path:line` evidence in each row; the three
`NOT RE-CHECKED` headings became `SHIPPED`/`OPEN`; gap 2 and gap 5 got a `Correction (2026-11-25)` block
like gap 6's; gap 7's correction records the finding that a `publish-<n>` attestation **authorizes** a repeat
rather than recording one, so a crate that already went out is finished by the upload and the promotion, not
by attesting a resume; and `## Sequencing` is a record with phases 1-5 done and phase 6 as the only open
work. The reversal ADR-0004 → ADR-0005 stays as the record. Nothing else in the body changed.

`gh issue view 326` is the state to plan from; T2 (the four decisions) is now the next step, and no code in
this document has been written.

**L8. D2 accepted, D3 accepted for Scoop, and unknown 1 measured: prereleases on crates.io are viable.**
Maintainer, verbatim: «Sí, de ahora en adelante las versiones las lanzaremos con escaleras, a scoop solo
estables y dime si es viable prereleases a crates.io?»

**Verified, not assumed (2026-11-25).**

- **Cargo never selects a prerelease unless the requirement asks for one.** The Cargo Book's pre-release
  section states both halves: a prerelease is not matched by a default requirement, and "`cargo install`
  will avoid pre-releases unless explicitly asked to install one"
  (`doc.rust-lang.org/cargo/reference/specifying-dependencies.html`); the resolver chapter repeats
  "Cargo doesn't automatically select pre-release versions"
  (`/cargo/reference/resolver.html#pre-releases`), and RFC 3493 states the rule as "always favours selecting
  stable versions over pre-release versions". The same page adds the property the ladder needs: "Cargo will
  also upgrade automatically to semver-compatible released versions from prereleases" — so `0.22.0`
  arriving lifts every `0.22.0-alpha.1` consumer on its next resolve.
- **crates.io already carries prereleases and distinguishes them.** Live API, `GET /api/v1/crates/<name>`:
  `leptos` returns `max_version = 0.9.0-rc1` with `max_stable_version = 0.8.23`; `dioxus`
  `0.8.0-alpha.3` / `0.7.10`; `rocket` `0.6.0-rc.1` / `0.5.4`. A consumer that wants stable reads the
  stable field; a consumer that wants the edge names the prerelease. Nothing about a prerelease hides the
  stable version from a default requirement.
- **docs.rs does not follow the prerelease.** `docs.rs/leptos/latest` renders **0.8.23** while `0.9.0-rc1`
  is the newest version; the prerelease keeps its own version path. So a `cutver-pdk` prerelease does not
  hijack the documented surface.
- **`cargo install cutver` is unaffected**, per the same Cargo Book sentence, so a published alpha does not
  become what `cargo install` gives a stranger.
- **The cost is permanence, and it is the one we already named.** A crates.io publish is permanent and the
  code cannot be deleted; `cargo yank` removes a version from resolution for new projects but leaves it
  downloadable (`doc.rust-lang.org/cargo/commands/cargo-yank.html`). Consequence for the ladder: a broken
  alpha costs a permanent artifact under a version number nobody will reuse — not a yank of anything a
  consumer already resolved, because no default requirement ever picked it.

**What this means for D4.** Viable, and the reason to do it stands on its own: the crate is the surface
under test, and skipping crates.io on a prerelease would leave `[publish].commands` — the `cargo publish`
that `--resume` must not repeat without a `publish-<n>` attestation — unexercised on exactly the channel
that is supposed to prove the ladder. One consequence to write into the guide when T9 lands: a
`cutver-pdk` prerelease is opt-in by exact requirement (`cutver-pdk = "= 0.3.0-alpha.1"`), because `"0.3"`
will not match it — which is the property that keeps plugin authors stable.

**D3 closed the same day, in full.** Asked which of the other three surfaces follow the same rule, the
maintainer chose all four, so the floating major tag, container `latest`, the Homebrew tap and the Scoop
bucket are stable-only and a prerelease publishes exactly its own version everywhere. That is the only
shape in which `uses: cutver/setup@v0` and its `version: latest` default cannot receive an alpha, which was
the hazard D3 existed for.

**D4 accepted too**, so all four decisions are answered and T2 is complete: per T2's own text, T3 onward
runs without another question. What that acceptance obliges is worth restating once, because it is
irreversible: the crate half now publishes on every channel, so the first alpha leaves a permanent
`0.22.0-alpha.1` on crates.io that can be yanked and never deleted. It is the price of exercising the
`publish-<n>` attestation path on the channel that is meant to prove the ladder.

- **D7 — The first real run is `0.22.0-alpha.1` and the ladder runs in full: `alpha.1 → beta.1 → rc.1 →
  0.22.0`.** Accepted 2026-11-25, superseding the short ladder decided the same day (L10 records both). The
  rungs are about the release *process*, and each is a **different transition** of the arithmetic phase 5
  shipped: `alpha → beta` and `beta → rc` are the restart-on-another-channel case (the counter starts at 1
  again), and `rc → 0.22.0` is promotion, which must not increment anything. Running the full ladder
  observes all three on the real pipeline; the short one left the two restarts unproven. What it still does
  **not** prove is the counter-only case (`alpha.1 → alpha.2`), which needs a second cut on the same
  channel — a rung re-run would prove it, and T10's record should say whether it happened. Cost, per D4:
  three permanent prerelease versions on crates.io, not two.

**L9. T3 merged into the branch, and the rung order decided.** Maintainer asked «Como todo está lo
suficientemente estable, podemos lanzarla en rc, que dices?» and, on the answer that the label is not what
makes a prerelease safe — the missing `--prerelease`/`--latest=false`, the floating tag, the container and
the two package managers are T5-T8, none of them written — chose **D7**: `0.22.0-alpha.1` as the rehearsal,
then `rc.1`, then `0.22.0`.

T3 is done in `be14e67` and verified by the parent: `release.yml` declares a `channel` `choice` input whose
first option is empty (so a dispatch nobody re-filled submits "no channel"), and the step appends
`--channel "<value>"` only under `if [ -n ... ]`, so an untouched dispatch runs the byte-identical argv it
ran before. `tests/release_pipeline_contract.rs` gained three tests plus a fail-closed argv reader that
executes the step's own shell constructs and panics on any construct it does not model. RED observed first —
`a_dispatch_channel_reaches_the_bump_arguments` failed with `left: ["bump", "minor", "--defer-publish"]`
against `right: [..., "--channel", "alpha"]`, and the input pin panicked with "the dispatch must declare a
`channel` input"; GREEN 31/31 with **zero deletions**, so no existing pin was edited or weakened. Two
mutations prove the new pins have teeth: removing the guard fails the empty-channel test, and moving the
channel branch after the call line fails the reach test. Parent re-ran the suite: 31 passed, 0 failed.

**Citation map, because T3 moved the numbers the rest of this document quotes.** In
`tests/release_pipeline_contract.rs` (now 1438 lines): the exact bump string `:184 → :384` and `:387 → :628`;
`args=(--defer-publish)` `:381 → :622`; the `inputs.dry_run` pins `:249 → :449`; the `released=` grep
`:332 → :573`; `WRITTEN` `:408 → :709`. In `release.yml` (now 1141 lines): the input map ends at `:32`, the
`channel` block is `:15-32`, the guard `:168-170`, the call `:171`. T3's own citation `:141-145` was right for
the `args` array but the call it qualifies is `:146`, outside that range. The plan's "1041 lines" for
`release.yml` was already stale at 1116 before this change.

**L10. `beta` comes back: the ladder runs in full.** Maintainer, verbatim: «mejor no saltemos beta, hagamos
la escalera completa, alpha, beta, rc y stable». This supersedes the short ladder accepted minutes earlier
(D7, L9), and it is the better call for a reason the short one got wrong: the rungs are not three samples of
the same behaviour, they are the three distinct transitions phase 5 claims to implement — restart on another
channel twice (`alpha → beta`, `beta → rc`) and promotion without an increment (`rc → 0.22.0`). Skipping
`beta` would have left the second restart, and promotion from a beta, unobserved on the real pipeline.

No task changes: T10 was written for the full ladder (`0.22.0-alpha.1 → beta → rc → stable`, observed
dispatch by dispatch). One thing to keep honest in T10's record: the **counter-only** case — two cuts on the
same channel, `alpha.1 → alpha.2` — is not part of a ladder that visits each channel once, so it stays
unproven on the real pipeline unless a rung is deliberately re-cut. The unit tests cover it; the pipeline
record should say so rather than imply four transitions were watched.

**L11. One build per package manager.** Maintainer: «de los gestores de paquete, scoop, homebrew y winget …
creo que hace más sentido solo tener la versión con plugins habilitado», and on the naming question chose the
deprecation shim over a hard switch. Measured state before the decision: the tap carries **two** formulas
(`Formula/cutver.rb` at `release.yml:827`, `Formula/cutver-plugins.rb` at `:904`) with Scoop mirroring them,
the release publishes both assets per target (`cutver-*` and `cutver-plugins-*` in `SHA256SUMS`), and
**nothing anywhere mentions winget** — `grep -rn winget` over `*.yml`, `*.toml` and `*.md` returns no match.

Why the single build is right: a package manager is the "just make it work" channel, and asking a user to
know what a WASM plugin is before choosing a formula is a support question we invented. The plugin build is
a functional superset; the lean build keeps shipping as a release asset for the people who need it, and its
smoke stays exactly as it is — Pillar VI.4 is about the build existing and staying small, not about what a
formula installs.

What the decision costs, named: a package-manager install grows by the WASM runtime (unmeasured here —
measure at T8, do not guess), a deprecated formula is one more file for one release, and winget is a new
review-gated surface whose manifests reach users a release behind, because `microsoft/winget-pkgs` validates
by pull request.

**L12. T4 done: the run reports its channel.** `ab2a44e`. `bump` writes `released, deferred, version,
channel, tag, commit` and `plan` writes `next_version, channel, tag, commit`, both with
`semver_bump::channel_of` applied to the version the run produced — **not** to the flag, so `1.3.0-rc.1`
reports `rc` whether or not `--channel rc` was the request, and a stable cut reports `stable`. No new field
on `Summary` or `ReleasePlan`: `src/bump/types.rs` is still 399 of 400, so T7's extraction is not on the path
after all. `--json` and `--report` are untouched because `fields.rs` feeds only the CI channel (two call
sites, `src/cli/ci.rs:182,196`), which is phase 5's D3 intact.

RED at both ends before the fix: the unit failed with the five-key rendering
(`version=1.3.0\ntag=v1.3.0` where `channel=stable` belonged), and `ci_output` failed on three sink
assertions including the end-to-end `--channel alpha` case. Six exact comparisons were **widened, never
relaxed to `contains`**; both sinks (file and step summary) are pinned for `stable` and for `alpha`.
`docs/internals/machine-contract-surfaces.md` held its 200-line ceiling at 199 with no trim, because the
CI-output bullet re-wrapped inside its 8 lines — the estimate was checked by the ceiling test, not asserted.

Two judgment calls worth keeping:
- **The worker left `docs/integrations/migrating-from-the-release-action.md:67` alone on purpose**: it lists
  the `--report` keys, which are JSON and gain no `channel`, so the one-word "fix" there would have
  documented a key the report never writes. That is the difference between a doc that is stale and a doc
  that is wrong.
- **The parent had to finish the formatting**: the worker is not authorized to run `cargo fmt`, so its own
  test and T3's contract helper were not rustfmt-canonical. `cargo fmt --all` ran before the commit, so
  `ab2a44e` is fmt-clean; the deltas fell inside files T4 already owned.

My handoff carried a defect worth naming: the continue prompt contained two `## Allowed edit surfaces`
blocks, a stale 4-path copy trailing the 8-path replacement. The worker read the replacement (correctly) and
reported the ambiguity instead of guessing quietly. One list, one block, next time.

**L13. Unknown 2 resolved, and T5's premise with it.** Measured on 2026-11-25 against `cutver/plugins`
(`main`) through the API, without reading the sibling clone at `d:/development/workspaces/plugins`, which is
outside this project:

- **The plugin accepts the parameter, and defaults better than the plan assumed.**
  `plugins/github-publish/src/post_release.rs:78-80` sets `prerelease` from
  `config.prerelease.unwrap_or_else(|| is_prerelease(&payload.version))`, so the draft for
  `0.22.0-alpha.1` is created **already marked prerelease** from the payload's version, with no config and
  no workflow flag. Its own test `automatically_marks_prerelease` (`:322-339`) pins that. `config.rs:178`
  also lets `GITHUB_PUBLISH_DRAFT` override `draft`, an environment variable this repository does not set.
- **GitHub cannot make a prerelease Latest**, so the failure the plan feared cannot happen while the release
  is marked: "The latest release is the most recent non-prerelease, non-draft release, sorted by the
  created_at attribute" and "Drafts and prereleases cannot be set as latest"
  (`docs.github.com/en/rest/releases/releases`). The `cutver/setup` `version: latest` hazard is real only for
  a release that is *not* prereleased.
- **So T5 is not "stop the alpha becoming Latest"; it is "stop depending on two things nobody here pins"**:
  the plugin's auto-derivation, and the absence of `prerelease = false` in `cutver.toml`. The explicit
  `--prerelease`/`--latest` flags and the config pin cost three lines and remove both dependencies.
- Two pieces of the plan's own reasoning survive untouched: the `Where` citations for the promotion step and
  the two `GH_TOKEN:`/`TAG:` pins are still the constraints to respect, and `release.yml`'s line number has
  moved because T3 added the `channel` input — locate it by the `gh release edit "$TAG" --draft=false` string.
