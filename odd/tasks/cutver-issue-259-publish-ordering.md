# Feature: release ordering — verify before publishing (issue #259)

Large, high-risk (delivery). Branch `fix/issue-259-publish-ordering`. Specs quote the issue and the
human's decision verbatim; nothing below adds a requirement they did not state.

## Specs

- **S1** (issue #259 acceptance) "`tests/release_pipeline_contract.rs` pins the new order the way it
  already pins build → PATH → action, so a reordering fails a test instead of a release."
- **S2** (issue #259 acceptance) "A release run whose smoke job fails publishes neither the crate
  nor the tag (prove with a deliberately red smoke on a rehearsal ref, not a real version bump)."
- **S3** (issue #259 acceptance) "The container smoke runs against an unpushed digest or a
  non-`latest` tag."
- **S4** (issue #259 remedy) "Take the irreversible steps out of the version job and give them their
  own job with `needs: [build, build-plugins, smoke, smoke-plugins]`, or gate them behind a first
  `--dry-run` pass plus a second invocation after verification." · "`cutver.toml [publish] commands`
  executes inside the version job by design. If the tool cannot express "publish after smoke",
  record that constraint explicitly and enforce the order at workflow level. Do not add a new
  dependency (Pillar IV.3)." · "Keep `draft = true`."
- **S5** (human decision, 2026-11-13) "Dos fases en el CLI (recomendado)" — "Fase prepare diferida +
  comando de cierre; el workflow llama al CLI directo. Cumple los 3 criterios." Detail from the
  accepted option: "`release.yml`: nueva salida `sha`; `build`/`smoke` sobre el sha; job final que
  hace tag + push + `cargo publish` + draft + promoción, con `needs: [build, build-plugins, smoke,
  smoke-plugins]`." · "El step deja de usar `cutver/release@v1` y llama al CLI construido (ya está
  en PATH)."

## Tasks

- [x] T1 Extract the deferred half of the release (tag, tag push, floating tag, `[publish].commands`,
  `on_post_release`) into `src/bump/exec/publish.rs`, behavior unchanged. S4
- [x] T2 `cutver bump --defer-publish`: keep the bump commit and the branch push, skip the deferred
  half, and say so in the summary. S4, S5 — with `--json` and `--report <PATH>` for CI
- [x] T3 `cutver publish --tag <TAG> --commit <SHA> [--notes-file <PATH>] [--dry-run]`: parse the tag
  as a domain type, fail closed when the tag does not match the manifests or the commit is not HEAD
  or the tag already exists remotely, then run the deferred half. S4, S5
- [x] T4 `release.yml`: doctor + bump + changelog steps replace the `cutver/release@v1` step, a `commit`
  output feeds `build`/`build-plugins`/`smoke`/`smoke-plugins`, and the `publish` job runs
  `cutver publish` before uploading assets and promoting the draft. S1, S2, S5
- [x] T5 Contract test pinning the order: nothing irreversible in the version job, `publish` needs all
  four verification jobs, §S1's marker idiom. S1
- [x] T6 `container.yml`: smoke a non-`latest` rehearsal tag and promote the verified digest into the
  release tags only after it passes. S3
- [x] T7 Docs: `docs/reference/cli.md`, `docs/reference/configuration.md`, `docs/internals/pipeline.md`,
  `docs/internals/design.md`, README. S4, S5
- [ ] T8 Gates: build, test, clippy, fmt, the new e2e test; record the S2 dynamic-proof constraint.

## Log

- **L1** (user, verbatim) "Vamos con el 259"
- **L2** (user decision, verbatim label) "Dos fases en el CLI (recomendado)" — chosen over "Mínimo:
  diferir sólo cargo publish" and "Repartir el issue".
- **L3** (evidence, `release.yml:178-180`) `build` checks out `ref: ${{ needs.version.outputs.tag }}`,
  so deferring the tag requires build/smoke to consume a commit sha instead. This is why S2 needs a
  `sha` output and not only a new job.
- **L4** (evidence, `src/bump/exec/pipeline.rs:130-181`) commit → tag → branch push → tag push →
  floating tag → `[publish].commands` → `on_post_release` live in one process; `src/config/validation.rs:174`
  accepts only `push`, `commands`, `default_timeout`, so no config value can split them.
- **L5** (evidence, `cutver/release@v1`) the action passes only fixed inputs
  (`command/bump/first-release/config/dry-run/skip-preflight/check-changelog/notes-file/template`) and
  its `release` command is doctor + bump + `changelog latest`; `released` is "HEAD changed", `tag` is
  `git describe --tags --match=v*.*.*`. A local tag created by the prepare phase keeps `git describe`
  working, so the action's outputs stay reproducible without a new JSON contract.
- **L6** (constraint) `.github/workflows/release.yml` is `workflow_dispatch`-only and `cutver bump`
  refuses outside `main` (`[git] require_branch`), so an S2 rehearsal on a throwaway ref cannot run
  the version job; the dynamic proof needs either a main-based rehearsal (a real bump commit, then
  revert) or the two-phase e2e test. Escalated to the human at Close.
- **L7** (decision, supersedes the `sha` wording of S5) the release commit sha travels to the build and
  smoke jobs, and the machine-readable handoff is `cutver bump --report <PATH>` (one JSON line) rather
  than parsing stdout. `--json` also prints the object, but `[preflight]` and git subprocess output
  share stdout on a real run; suppressing child stdio would mean editing `src/git/ops.rs` and
  `src/preflight.rs` for no release benefit, so `--report` is the guaranteed path and the `--json`
  limitation is documented instead.
- **L8** (decision) the container half promotes the smoked digest with `docker buildx imagetools
  create` from a single `promote` job whose `needs:` names both builds and both smokes; all-or-nothing
  on purpose, and rehearsal tags are left in place rather than force-deleted (Pillar II.2).
- **L9** (residual, honest) the `publish` job publishes the crate before it uploads the assets and
  promotes the draft, because the draft release only exists once `cutver publish` dispatches the
  post-release plugin. A failing asset upload therefore leaves a crate with an unpublished draft,
  retryable by re-running the upload and promotion; the build/smoke failure the issue targets can no
  longer publish anything.
- **L10** (refactor, needed by the contract) `pipeline.rs` was at 436 lines after the extraction, above
  the 400-line hard ceiling: the `[hooks]` steps moved verbatim to `src/bump/exec/hooks.rs` and the
  phase-1 guards plus the interpolation context to `src/bump/exec/guard.rs`. A differential run against
  the base commit showed identical exit codes, tags, remote refs, `published.txt` and output shape.
