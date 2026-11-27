# Machine Contract: Release Tail

> Outcome first: `publish --json`, `attest --json` and `init --json` each answer with one object on one
> stream, so a wrapper reads what the release tail did instead of parsing a tree or a prose line. The
> protocol they obey — `schemaVersion` first, the exit codes, the error envelope — is the
> [machine contract](machine-contract.md), whose `## Surfaces` table is the index to this page. All
> three print one compact line, `schemaVersion` first, and their payload keys follow in the order
> below, because field order is part of the contract (`CONTRACT.md:231`). A refusal prints one
> envelope on **stderr** carrying its class — `publish`, `attest` or `init` — and adds nothing to
> stdout, so a consumer never reads half a document.

## `cutver publish --json`

What the closing phase did: what it tagged, what it pushed and what it ran. Without the flag the
command prints the publish tree, unchanged — `--json` replaces the tree rather than following it.

```json
{"schemaVersion":1,"tag":"v1.3.0","commit":"…","dry_run":false,"tag_skipped":false,"pushed_refs":["refs/tags/v1.3.0","+refs/tags/v1:refs/tags/v1"],"floating_tag":"v1","floating_tag_moved":true,"publish_commands":[{"position":1,"command":"cargo publish --locked","exit_code":0}],"post_release":{"dispatched":[],"left_alone":[]}}
```

The shape is `PublishJson` (`src/cli/runner/publish.rs:204`); the keys, in order:

- `tag` — the release tag this run closed, prefix included. A consumer matches it to the version it
  built, or reads it back from the remote.
- `commit` — the release commit the tag points at, as the full sha: the commit to compare against the
  artifacts that were smoked.
- `dry_run` — whether the invocation was a rehearsal. Every other key describes the same run, so a
  consumer that ignores this one reads a plan as a result.
- `tag_skipped` — `true` when the release tag already pointed at the commit, so this run created none.
  It is what separates "this run tagged" from "this run continued a release that was tagged".
- `pushed_refs` — the refspecs `git` received, in push order, each spelled as `git` received it, never
  the `&&`-joined command line a human reads. It is `[]` when nothing was pushed: a `--dry-run`, a
  `[publish] push = false` workspace, and every closing phase, which pushes no branch because the
  earlier `bump` did. A consumer relays the list, or confirms it with `git ls-remote origin`.
- `floating_tag` — the floating major tag this release names (`v1` for `v1.3.0`). It is `null` when
  `[git] floating_major_tag` is off, and on a prerelease, which moves none (ADR-0005).
- `floating_tag_moved` — `true` when this run moved that tag in the local repository; `false` on a
  `--dry-run` and whenever `floating_tag` is `null`. Whether the remote copy followed is a separate
  fact, and it appears in `pushed_refs` only when `[publish] push` is on.
- `publish_commands` — one entry per declared `[publish] commands`, in declaration order: `position`
  (1-based in the declaration, and the `publish-<n>` gate name a resume looks for), `command` (the
  template after interpolation) and `exit_code` (the code the child exited with — `null` for a command
  that did not run, which is a `--dry-run`, and for a child a signal ended, which has no code,
  `src/bump/exec/command.rs:71`). A consumer reads each code here instead of scraping the logs.
- `post_release` — the `on_post_release` outcome, as `dispatched` and `left_alone`. `dispatched` names
  the plugins the phase dispatched, and a `--dry-run` dispatches them with `dry_run: true` in the
  payload, so the list is what a real run names; the manager holds them in a map, so a consumer sorts
  the list rather than reading it as declaration order. `left_alone` is the sorted plugins a `--resume`
  refused to repeat — a plugin is arbitrary code that may already have run in the attempt the resume
  continues — and it is empty on a plain run.

`--dry-run` changes values and never keys: `pushed_refs` is `[]`, `floating_tag_moved` is `false`, every
`exit_code` is `null`, and the rest reads the same, so an adapter parses one schema for both runs. A
failed publish writes no object at all — the envelope is the whole stderr line, and stdout stays empty.

## `cutver attest --json`

The record the command just wrote, as data: the verdict and the file it landed in. The human channel is
one line naming the same facts in prose, and the flag changes the channel only — never the bytes of the
file.

```json
{"schemaVersion":1,"gate":"ci","commit":"0123456789abcdef0123456789abcdef01234567","conclusion":"success","verified_at":"2026-11-18","run":null,"path":".cutver/attestations/ci.json"}
```

The shape is `AttestJson` (`src/cli/runner/dispatch.rs:245`); the keys, in order:

- `gate` — the gate this verdict answers for: the name `publish --require-gate` and `[gates] require`
  use. A consumer matches it to the gate it asked for instead of trusting the order of the files.
- `commit` — the commit the verdict is for, as the full sha `git rev-parse HEAD` prints. The command
  refuses a tag, a branch and a short form, so the value is always comparable with the commit that
  `plan --json` and `publish --json` report.
- `conclusion` — `success` or `failure`, the two values the file spells. A `failure` is recorded, not
  discarded: `publish` refuses a gate it requires, and `plan` reports it as data.
- `verified_at` — the UTC day the verdict was recorded, an ISO date, from the clock the changelog
  heading uses. Cutver applies no freshness policy to it; a consumer decides whether a verdict is old.
- `run` — the run the gate named with `--run`, and the key is present and `null` when it named none, so
  an adapter parses one shape. A consumer links a human back to the job that verified the commit.
- `path` — the attestation file, spelled **relative to the workspace root** with forward slashes. It is
  the file `publish --require-gate` reads: `[gates] dir`, `.cutver/attestations` by default
  (`src/config/gates/table.rs:11`), joined with `<gate>.json`. A consumer resolves it against the
  workspace root it already knows — to archive or upload the file — instead of re-deriving the
  directory from the configuration.

### The file the record lands in

The file is the same record, versioned. It opens with `schemaVersion` as its first key, carrying the
integer `--json` printed, so the object and the file cannot disagree about the version. The keys that
follow keep the order above, with two differences that come from what the file is:

- `path` is absent, because it is the location of the file and cannot sit inside the file's own bytes.
- `run` is absent when the gate named none, where the printed object spells it `null`: the file omits a
  key it has no value for.

```json
{"schemaVersion":1,"gate":"ci","commit":"0123456789abcdef0123456789abcdef01234567","conclusion":"success","verified_at":"2026-11-26"}
```

A file with no `schemaVersion` predates the key, or came from another tool. It is the legacy shape and
it still reads as the attestation it holds. The reader is total on purpose
(`src/config/gates/store.rs`): a file that does not parse counts as no attestation, and
`publish --require-gate` refuses a gate it has no attestation for (`src/bump/exec/guard.rs`). Refusing
a file only because the key is missing would accordingly disable a gate that had vouched for the
commit, which is why the absence of the key is not an error.

A test enumerates a configured `[gates] dir` and asserts that each `.json` file there opens with the
key (`tests/attest_init_json.rs`, `every_file_in_the_gates_directory_opens_with_the_schema_version`), so
a writer that stopped adding it fails the suite. It covers the writers that exist: a new command
writing elsewhere in that directory, a file that is not `.json`, or a subdirectory would not be
examined.

## `cutver init --json`

What the scaffold found and wrote. A fresh run and `--update` answer with the same three keys, so an
adapter parses one shape for both.

```json
{"schemaVersion":1,"manifests":[{"path":"package.json","is_root":true,"kind":"json"},{"path":"android/build.gradle.kts","is_root":false,"kind":"gradle"}],"primary":"package.json","files_written":["cutver.toml","CHANGELOG.md",".github/templates/cutver/RELEASE.md"]}
```

The shape is `InitJson` and its `ManifestJson` entries (`src/init/scaffold/runner.rs:260`, `:269`); the
keys, in order:

- `manifests` — one entry per manifest discovery found, in the order the configuration declares them,
  and `[]` when it found none. Each entry carries `path` (relative to the target directory, forward
  slashes), `is_root` (whether it sits at that directory's root rather than in a subdirectory) and
  `kind`: the spelling the configuration uses, from the closed set `cargo-package`, `json`, `gradle`
  and `pyproject`. That is not the human summary's `json: version`, which names a field. A consumer
  maps `kind` to the file it has to edit.
- `primary` — the path of the manifest the configuration makes its source of truth: on a fresh scaffold
  the first one detected, and on `--update` what the existing document declares, read in this order:
  `[version] current_source` (or its deprecated alias `source`), then the manifest flagged
  `primary = true`, then the first declared manifest that is not a template sink — a manifest of
  `kind = "template"` generates its target from a template instead of holding a version, so it can
  never be the source. It is the order the configuration loader applies (`src/config/validation.rs`),
  and a test proves the document reading and the loader name the same manifest
  (`src/init/scaffold/tests.rs`). It is `null` when nothing was detected. A consumer reads the version
  from one file instead of guessing which one the release uses.
- `files_written` — the files this run wrote, relative to the target directory with forward slashes:
  the configuration, plus `CHANGELOG.md` when it did not exist, and the release template
  (`.github/templates/cutver/RELEASE.md`, `src/init/scaffold/templates.rs:17`) when it is missing or
  `--force` rewrites it. An `--update` writes no scaffold, so the list is the configuration when it
  appended something and `[]` when it had nothing to append: the object never claims a file the run did
  not write. A consumer reads it as the set to review or revert, and an empty list as "already
  configured".

An `init` that refuses an existing `cutver.toml` without `--force` writes nothing and prints the
envelope with the `init` class on stderr, with stdout untouched.

## Related Documentation

- [The Machine Contract](machine-contract.md): `schemaVersion` first, the exit-code table and the machine error envelope every surface here obeys.
- [Command Surfaces](machine-contract-surfaces.md): the CI output channel, `doctor --json`, `plan --json` and `changelog --json` as data.
- [CLI Commands and Exit Codes](../reference/cli.md): the three commands these objects belong to.
