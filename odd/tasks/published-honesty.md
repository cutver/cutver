# published-honesty — what a released version claims must be what it does

Three findings from auditing v0.21.0 against the tree, all of them published to the public before
they were true. The tool's own release notes, and the documentation that ships **inside** the crate
tarball, promise things the code does not do.

## Specs

- **S1 — `env("VAR", "default")` works, because we published it.** `docs/templates/release-notes.md`
  documents `env("VAR_NAME", "default")` and its example `{{ env("GITHUB_ACTOR", "CI") }}`, while
  `src/changelog/render/template.rs:160` registers `env` as a one-argument global
  (`env.add_function("env", lookup)`, `lookup: Fn(&str) -> Option<String>`). A user who copies the
  documented call gets a template error. `Cargo.toml:12` excludes only `odd/`, so that page ships in
  every crates.io tarball: this is not a stale website, it is a published artefact.
  The second argument is optional and additive: one argument keeps today's behaviour exactly
  (absent variable renders as undefined, not as an error; that is what the existing test pins at
  `src/changelog/render/template.rs:251-258`), two arguments return the default when the variable is
  absent. Zero, three or more arguments, or a non-string argument, is a template error naming the
  call — never a silent coercion.
- **S2 — `publish` speaks the CI channel like `bump` already does.** Today only `bump`
  (`src/cli/ci.rs:209` `on_bump_success`) and `plan` (`:224` `on_plan_success`) feed the channel;
  `runner/dispatch.rs:26` only annotates failures. So the **irreversible** step of a release writes
  no step outputs and no summary, and a consumer has to parse stdout to learn what it did — the
  exact work the v0.20.0 note says the tool took over.
  The outputs mirror `bump`'s names wherever they mean the same thing (`tag`, `commit`), and add
  what only `publish` can know: whether the tag was pushed, whether the floating major tag moved, and
  the number of `[publish] commands` that ran. The summary is one bounded markdown block like the
  others. Nothing in the channel writes stdout, so `--json` stays byte-identical with and without CI
  detection.
- **S3 — a step output that cannot be written degrades, it does not fail the release.**
  `ci::append` already takes that posture (`src/cli/ci.rs:204-222`), and `publish` must not become an
  exception: the release has already happened by then, so a bookkeeping failure warns and continues.
- **S4 — the released notes stop reading as capabilities that do not exist.** Two corrections, and
  both are corrections rather than rewrites, because a released section is a record:
  1. v0.21.0's `**release**: take the release body from notes in the tree` describes a change to
     `.github/workflows/release.yml`, not a surface of the binary: there is no `[release] notes_dir`
     in `src/config/types.rs` and the binary's only notes input is `publish --notes-file`. The entry
     gains the clause that says so.
  2. The v0.20.0 claim about the CI contract is true after S2 lands; if S2 is ever revisited, the
     note has to be narrowed in the same change. Nothing else in the five released sections is
     touched.
- **S5 — Rails.** `src/changelog/render/template.rs` is not allowlisted (ceiling 400); record its
  line count before and after. `src/cli/ci.rs` is likewise free. The release notes page
  `docs/templates/release-notes.md` documents a surface, so the correction is a documentation change
  that must say what happens when the variable is absent with and without a default.

## Tasks

| ID | Spec | Route | Commit |
| --- | --- | --- | --- |
| T1 | S1, S5 | Red first: a test that renders `{{ env("CUTVER_TEST_ENV_VAR", "fallback") }}` through the injected lookup and expects `fallback`. It fails today with an arity error. Then the optional second argument, keeping the one-argument behaviour byte-identical | 1 |
| T2 | S2, S3, S5 | `ci::fields::publish` + `ci::on_publish_success`, called from the publish runner on success; a test that the outputs reach a named `$GITHUB_OUTPUT` file and that stdout is untouched with the channel on | 2 |
| T3 | S4 | The two corrections to `CHANGELOG.md`, with the released sections otherwise byte-identical | 3 |

## Log

L1 (maintainer, verbatim): «Hemos invluido cosas a cutver que no hemos implementado pero que ya fueron publicadas al público en versiones, investiga que podemos aplicar al workflow de este repo y al cutver.toml»

L2 (maintainer, verbatim): «recuerda que lanzamos la V0.21.0 que de hecho, la tengo instalada en este equipo a traves de scoop, con un cutver -V lo vez»

L3 (decision, maintained by the maintainer's choice): «Primero la honestidad de lo publicado» — before wiring gates or artifacts into the release workflow, correct what the published versions claim.

L4 (evidence, this session, measured against the installed 0.21.0 binary obtained by scoop, in a scratch repository): `[[artifact]]` with `glob`/`at_least`/`description` is accepted and `plan --json` reports `"matched": N`; `publish` refuses with `Error: declared artifact '…' matched 0 file(s), 2 required (glob 'dist/*.txt')` and a `Where:`/`Fix:` pair. `[gates] require` is accepted and `plan --json` reports each gate's `conclusion`/`commit`/`verified_at`/`run`; `cutver attest smoke --commit … --conclusion success --run …` writes `.cutver/attestations/smoke.json`. `plan --json` emits `schemaVersion:1`, `phases[]` with `mutates` and `steps`, `artifacts[]`, `gates[]` and `blockers[]`.
  **Trap found by that probe and worth remembering:** a declared artifact whose files are untracked makes `publish` refuse on the clean-tree guard (`working tree has uncommitted changes: ?? dist/`). The only exemptions are `--notes-file` and the attestation directory, so any `[[artifact]]` over build output needs those paths ignored or outside the tree.

L5 (evidence, this session): `Cargo.toml:12` reads `exclude = ["odd/"]`, so `docs/` ships inside the crates.io tarball. That is what turns the `env()` example from a documentation bug into a published artefact defect.

L6 (evidence, this session, reported by an independent read of the tree): the CI channel is fed only by `bump` (`src/cli/ci.rs:209`) and `plan` (`:224`); `src/cli/runner/dispatch.rs:26` only annotates failures, and `runner/publish.rs`, `runner/doctor.rs` and `runner/changelog.rs` call nothing. The v0.21.0 entry `take the release body from notes in the tree` names a workflow change: `grep -rn "releases/" src/` finds nothing and `[release] notes_dir` is absent from `src/config/types.rs`.
