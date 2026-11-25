# CLI Command Reference

Comprehensive command-line interface documentation for `cutver`.

Every command supports `--help`; each section below lists the same flags. `cutver --help` lists them.

## Exit Codes

`cutver` uses deterministic exit codes across all commands:

| Code | Meaning | Common Scenarios |
| :---: | :--- | :--- |
| `0` | **Success** | Release finished successfully, doctor checks passed, or notes emitted. |
| `1` | **General Error** | Configuration parse error, missing file, preflight test failure, a hook or publish command that outlives its timeout, `cutver publish` validation failure, or git safety violation. |
| `2` | **Drift Detected** | `cutver doctor` detected manifest version drift across packages or mismatch between tags and changelog. |

Every JSON surface reports the same outcome and opens with `schemaVersion` ([machine contract](../internals/machine-contract.md)).

`cutver doctor` reports more than drift, and none of it moves the exit code. Besides one row per
`[plugins.<name>.permissions] filesystem` grant — the declared spelling, the resolved guest path and
the canonical host directory behind it, with the plugin name and capability set — it prints one row per
`[plugins.<name>.permissions] env` grant stating whether the host can produce it, never its value. Its
`--json` object carries a `plugins` array (one entry per declared plugin: `name`, `runtime`,
`capabilities`, `grants`, `env_grants`) and a `plugin_load_error` field for a driver that could not be
built at all. Each `grants` entry carries `effect`: `applied` under the WASM runtime, `ignored` under
`runtime = "process"`, which applies only `permissions.env` (#367).
An unset environment grant is a row of its own rather than a load failure: grants are resolved when the
plugin is invoked (#300), so a host that exports none of them still gets a diagnosis. The exit code
stays `0` for a clean run, `2` for drift, and `1` only when a check could not run.

## Global Options

These flags are accepted by every subcommand, before or after its name.

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| | `--ci` / `--no-ci` | `--ci` turns the CI output channel on — `$GITHUB_OUTPUT`, `$GITHUB_STEP_SUMMARY` and a stderr failure annotation — without `GITHUB_ACTIONS` and without a named sink; `--no-ci` turns it off even on a detected CI. Without either, the CI output channel follows `GITHUB_ACTIONS` reading `true` (trimmed, case-insensitive) plus a named file. It never writes stdout, so `--json` is unaffected: [CI output channel](../internals/machine-contract-surfaces.md#ci-output-channel). |
| | `--verbose` | Forwards a `runtime = "process"` plugin child's stderr to cutver's own stderr, unchanged, on a call that exits `0`, so a plugin that logs while it works can be read at last. Without it the stderr of a successful call is drained and dropped, and a failed call's stderr is its error message either way, byte for byte. Nothing else in `cutver` becomes verbose today: this is the flag's one effect. Under `runtime = "wasm"` it changes nothing, because a WASM plugin has no child and no stderr. |
| `-c <PATH>` | `--config <PATH>` | Explicit path to `cutver.toml`; without it, discovery walks up from the current directory. |

## Release Commands

The five commands that cut a release, in the order the two-phase flow uses them. Diagnostics, the
changelog, `open` and the plugin commands have [their own page](cli-tools.md), because both pages are
bound by the same documentation ceiling and one page of nine commands is past it.

### `cutver init`

`cutver init [OPTIONS]` scans project files, detects package manifests, determines primary source of truth, and scaffolds `cutver.toml`, a starter `CHANGELOG.md`, and default release notes template.

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| `-u` | `--update` | Appends newly added package manifests to existing `cutver.toml` without overwriting settings. |
| `-f` | `--force` | Overwrites `cutver.toml` and default template if they already exist. |
| `-p <DIR>` | `--path <DIR>` | Target directory to inspect and initialize (default: current directory). |
| `-nt` | `--no-template` | Skips scaffolding `.github/templates/cutver/RELEASE.md`. |

### `cutver bump`

Calculates SemVer updates, runs preflights, applies atomic manifest changes, updates the changelog, commits, and tags. Add `--defer-publish` to stop after the branch push and close the release later with `cutver publish` ([Release Pipeline Order and Atomic Guarantees](../internals/pipeline.md)).

```text
cutver bump [LEVEL] [OPTIONS]
```

`[LEVEL]` is `patch`, `minor`, `major` or `auto` (default: `auto`).

`--channel <CHANNEL>` is the prerelease ladder: `alpha`, `beta` and `rc` append the channel and start its counter (`1.3.0` becomes `1.3.0-alpha.1`), and `stable` is the release itself. On a version that is already a prerelease the channel **promotes** it, and the level is ignored: the same channel moves only the counter (`1.2.0-alpha.1` → `1.2.0-alpha.2`, and any build metadata is dropped exactly as a level drops it, `1.2.0-alpha.1+build.7` → `1.2.0-alpha.2`), another channel starts its own counter on the same base (`1.2.0-alpha.3` with `--channel beta` → `1.2.0-beta.1`), and `stable` drops the suffix with no increment (`1.2.0-rc.1` → `1.2.0`) — no rung is validated, so `alpha` promotes exactly like `rc`. On a stable version a named level still cuts the version it names, so `plan minor --channel stable` on `1.2.3` reports `1.3.0`; a request that yields the version the repository already carries releases nothing ([No-op](#no-op)), which is why `--first-release`, whose tag carries the version the manifests declare, is never one. The bare word "channel" always means that ladder; the [CI output channel](../internals/machine-contract-surfaces.md#ci-output-channel) and the gate channel carry their qualifier every time they are written. The plan tree names the channel the run leaves and the one it lands on, so reading suffixes by eye is not part of approving a release.

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| | `--channel <CHANNEL>` | Publishes on this channel: `alpha`, `beta`, `rc` or `stable` — applied to the version the level cuts, or as a promotion when the version already carries a prerelease. Without the flag the version keeps no channel of its own, exactly as before. |
| | `--dry-run` | Simulates the release without writing to disk, committing, or pushing tags. |
| `-fr`, `--fr` | `--first-release` | Cuts an initial release tag without incrementing the manifest version. |
| | `--skip-preflight <STEP>` | Bypasses one or more named preflight checks (can be repeated). |
| | `--defer-publish` | Creates the release commit and pushes the branch, then stops: no tag, no tag push, no floating tag move, no `[publish]` commands, no `on_post_release` dispatch. `cutver publish` performs those. |
| | `--json` | Prints the release result as one JSON object on stdout. It is the last line cutver writes there, not the only one: `[preflight]` steps and git print above it. |
| | `--report <PATH>` | Writes that same object to this file, atomically, and nothing else. Use it when a machine must read the object alone. |
| `-c <PATH>` | `--config <PATH>` | Explicit path to `cutver.toml`. |

```bash
# Preview, run, or skip a preflight step; then commit and leave the tag and publishing to `cutver publish`
cutver bump auto --dry-run
cutver bump auto
cutver bump auto --skip-preflight lint
cutver bump minor --defer-publish
```

#### Machine-readable result

`--json` and `--report` are independent: either, both, or neither. Both describe a real and a `--dry-run` run alike, a failed run writes no report, and field order is part of the contract: the object opens with `schemaVersion`, then `released`, `deferred`, `version`, `tag`, `commit`, `floating_tag`, `publish_push_command` and `publish_commands`.

```json
{"released":true,"deferred":false,"version":"0.19.0","tag":"v0.19.0","commit":"abc1234","floating_tag":"v0","publish_push_command":null,"publish_commands":["cargo publish"]}
```

`released` is `false` on a `--dry-run` and on a [no-op](#no-op) — the one authority is `!dry_run && !no_op()` (`src/bump/types.rs:348`); `floating_tag` is `null` when the run never reached it; `publish_commands` lists the rendered `[publish].commands` and is always an array. `publish_push_command` is the *planned* push line, so it is filled on a `--dry-run` report and `null` on a real run, whose pushes have already happened: read it before the run and confirm the refs afterwards with `git ls-remote origin`. On a detected CI it also fills the runner's sinks ([CI output channel](../internals/machine-contract-surfaces.md#ci-output-channel)).

#### No-op

A request that yields the version the repository already carries releases nothing: `cutver bump --channel stable` on a stable version with no level named exits `0`, writes no file, makes no commit and creates no tag — no preflight step runs, no hook, no `[publish]` command and no lifecycle plugin dispatch, `on_pre_bump` included. The tree opens a `No-op` section and marks the commit and the tag `[SKIPPED]`, `released` is `false` in `--json`, `--report` and both CI sinks, and a versioning plugin that answers with the current version is reported the same way.

---

### `cutver plan`

Computes the release the workspace is about to cut and reports it without changing anything: no file, no git mutation, no `[preflight]`, no `[hooks]`, no `[publish] commands` and no plugin dispatch — not even `on_pre_bump`, which `bump --dry-run` still dispatches. A dirty tree on the wrong branch is the normal case, because that is the state CI asks the question in. A plan on a channel names it in the tree and adds no `channel` key to the JSON: `current_version`, `next_version` and `manifests[].from`/`to` already report the transition.

```text
cutver plan [LEVEL] [--json] [--skip-preflight <STEP>]... [--first-release] [--channel <CHANNEL>]
```

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| | `--channel <CHANNEL>` | Reports the version the run would cut on this prerelease channel: `alpha`, `beta`, `rc` or `stable`, computed exactly like `bump --channel`. A plan reports it and writes nothing. |
| | `--json` | Prints the plan as one JSON object on stdout instead of the plan tree. |
| | `--skip-preflight <STEP>` | Marks a named preflight step as skipped; a plan never runs one (can be repeated). |
| `-fr`, `--fr` | `--first-release` | Plans the initial release: the tag carries the version the manifests already declare. |
| `-c <PATH>` | `--config <PATH>` | Explicit path to `cutver.toml`. |

`notes` is the rendered changelog entry, produced without writing the file, and `phases` is the release in order (`validate`, `mutate`, `publish`) with the steps each phase would run. The guards that abort a real `bump` — dirty tree, required branch, an existing release tag, an invalid tag name — arrive as one-line `blockers`, and the command still exits `0`; exit `1` means no plan could be computed. Every key, including `gates`, is documented in the [machine surfaces reference](../internals/machine-contract-surfaces.md).

---

### `cutver publish`

Closes a release that `bump --defer-publish` prepared: it creates the annotated tag, pushes it, moves the floating major tag, runs `[publish].commands` and dispatches the `on_post_release` plugins. Validation fails closed before any of that, naming the command that resolves the problem and stating that nothing was written; an existing-tag error names the commit it points at (`src/bump/types.rs:85-88`). Every declared `[[artifact]]` glob is also verified after validation and before the tag: one that matches fewer files than its `at_least` fails closed naming the glob and the counts, and `--dry-run` reports that same verdict. Which inputs are validated, and why the split exists, live in [Release Pipeline Order and Atomic Guarantees](../internals/pipeline.md). On a detected CI it also fills the runner's sinks with what it published ([CI output channel](../internals/machine-contract-surfaces.md#ci-output-channel)).

```text
cutver publish --tag <TAG> [OPTIONS]
```

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| | `--tag <TAG>` | Release tag to publish. It must be a valid git ref, and it must equal the version the manifests declare at `HEAD`, so the tag is derived from the release commit instead of trusted from the command line. |
| | `--commit <SHA>` | The release commit the caller verified. When given it must be `HEAD`; omit it to publish whatever `HEAD` is. |
| | `--notes-file <PATH>` | Release notes for the `on_post_release` payload, which is what renders the draft release. The file is the command's input rather than a change in the tree, so a path that names a directory is refused before the clean-tree check; `-` reads the notes from standard input instead. An empty stream — or one carrying only whitespace — is refused before the tag, the pushes and the commands, because a release published with no notes cannot be corrected by re-running. |
| | `--require-gate <NAME>` | Refuses to publish unless this gate has an attestation for the commit being published (can be repeated). It adds to the gates `[gates] require` already declares, which `publish` enforces whether or not the flag is given; a gate named both ways is required once. The refusal names the gate and the commit, and a recorded `failure` refuses too. `[gates] dir` is exempt from the clean-tree check exactly like `--notes-file`: an attestation is an input, not a change in the tree. See [`cutver attest`](#cutver-attest). |
| | `--dry-run` | Validates and prints the plan without creating a tag, pushing anything or running a command. |
| | `--resume` | Continues a release an earlier `cutver publish` left unfinished, driven by the repository instead of a run record: it repeats only the steps that are harmless to repeat and refuses whatever the evidence contradicts. It requires `--commit`, it will not run a `[publish] commands` entry again unless `publish-<i>` — the entry's 1-based position in declaration order — holds an attestation with conclusion `success` for that commit, and it dispatches no `on_post_release` plugin. `--dry-run --resume` reports the verdict the real resume would reach and writes nothing. |
| `-c <PATH>` | `--config <PATH>` | Explicit path to `cutver.toml`. |

#### What `--resume` refuses

Nothing persists a run record, so a resume reads the repository for its plan: the release tag here and
on the remote, the floating major tag, and one `publish-<i>` attestation per `[publish] commands` entry.
Within that evidence it repeats only what is harmless to repeat — creating a tag that is absent, pushing
it, moving the floating major tag, verifying the gates read-only — and it refuses, naming the step and
the evidence it found, whenever the repository says this is not the release it continues:

- **no `--commit`** — a resume cannot re-identify the release it continues, which is why the flag
  requires the commit rather than guessing at `HEAD`;
- **`--commit` is not `HEAD`** — the release moved on, and the same rule a plain `cutver publish`
  applies stands here;
- **the version the manifests declare at `HEAD` does not produce `--tag`** — the same derivation a plain
  `cutver publish` performs;
- **the local or the remote tag `--tag` points at another commit** — a tag naming a different commit is
  a different release, so repeating a step over it would rewrite something this run did not do;
- **a `[publish] commands` entry has no `publish-<i>` attestation with conclusion `success` for
  `--commit`**, where `<i>` is the entry's 1-based position in declaration order. An arbitrary command
  is the one step a resume may not repeat on the tool's own judgement: record the verdict with
  `cutver attest publish-1 --commit <sha> --conclusion success` once it really succeeded, then resume.
- **an `on_post_release` plugin is not dispatched at all** — a plugin is arbitrary code too, and it may
  already have run in the attempt this resume continues, so repeating it is exactly the guess a resume
  refuses to make. The tree names the plugins it left alone. Inspect the release, and run `cutver publish`
  without `--resume` if the plugin still has to run.

A refusal names the entry's position and the command verbatim, and writes, tags, pushes and executes
nothing. A resume whose steps are all already in place exits `0`, reports itself as `Resumed` rather
than `Published`, and leaves the tag and the remote exactly as they were.

```bash
cutver bump minor --defer-publish --report .cutver-release.json
cutver publish --tag v1.3.0 --commit "$(git rev-parse HEAD)" --notes-file RELEASE_NOTES.md
cutver publish --tag v1.3.0 --commit "$(git rev-parse HEAD)" --resume
```

---

### `cutver attest`

Records a gate's verdict for a commit, so `publish` can require it: `cutver attest <GATE> --commit <SHA> --conclusion success|failure [--run <URL>]` writes one JSON object into `[gates] dir` (default `.cutver/attestations`). The verdict is recorded, never verified: `cutver` signs nothing and talks to no network, so an attestation is only as trustworthy as the machine that wrote it ([the trust boundary](configuration-tables.md#gates)).

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| `[GATE]` | | The gate this attestation covers. It becomes the file name inside `[gates] dir`, so only letters, digits, `-`, `_` and an inner `.` are accepted. |
| | `--commit <SHA>` | The commit the gate verified, as the full sha `git rev-parse HEAD` prints. A tag, a branch or a short form is refused: the attestation is matched against the release commit. |
| | `--conclusion <VERDICT>` | `success` or `failure`, and nothing else. A recorded `failure` refuses a release that requires the gate, in the same voice as a missing attestation. |
| | `--run <URL>` | The run that produced the verdict, stored with it and reported by `cutver plan --json`. |

---

## Related Documentation

- [Diagnostics, Changelog, Open and Plugins](cli-tools.md): the other four commands, with their flags.
- [Getting Started with cutver](../guides/getting-started.md): Walkthrough of your first release.
- [Machine Contract](../internals/machine-contract.md): `schemaVersion`, every JSON surface, and exit codes.
- [Complete cutver.toml Configuration Reference](configuration.md): Reference for all configuration keys.
- [Language Manifests and Surgical Diff Guarantees](manifests.md): How cutver updates each file kind.
