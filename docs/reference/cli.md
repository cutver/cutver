# CLI Command Reference

Comprehensive command-line interface documentation for `cutver`.

Every command supports `--help` for quick reference; each section below lists the same flags.

```bash
cutver --help
```

---

## Exit Codes

`cutver` uses deterministic exit codes across all commands:

| Code | Meaning | Common Scenarios |
| :---: | :--- | :--- |
| `0` | **Success** | Release finished successfully, doctor checks passed, or notes emitted. |
| `1` | **General Error** | Configuration parse error, missing file, preflight test failure, a hook or publish command that outlives its timeout, `cutver publish` validation failure, or git safety violation. |
| `2` | **Drift Detected** | `cutver doctor` detected manifest version drift across packages or mismatch between tags and changelog. |

Every JSON surface reports the same outcome and opens with `schemaVersion` ([machine contract](../internals/machine-contract.md)).

---

## Commands

### `cutver init`

`cutver init [OPTIONS]` scans project files, detects package manifests, determines primary source of truth, and scaffolds `cutver.toml`, a starter `CHANGELOG.md`, and default release notes template.

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| `-u` | `--update` | Appends newly added package manifests to existing `cutver.toml` without overwriting settings. |
| `-f` | `--force` | Overwrites `cutver.toml` and default template if they already exist. |
| `-p <DIR>` | `--path <DIR>` | Target directory to inspect and initialize (default: current directory). |
| `-nt` | `--no-template` | Skips scaffolding `.github/templates/cutver/RELEASE.md`. |

---

### `cutver bump`

Calculates SemVer updates, runs preflights, applies atomic manifest changes, updates the changelog, commits, and tags. Add `--defer-publish` to stop after the branch push and close the release later with `cutver publish` ([Release Pipeline Order and Atomic Guarantees](../internals/pipeline.md)).

```bash
cutver bump [LEVEL] [OPTIONS]
```

`[LEVEL]` is `patch`, `minor`, `major` or `auto` (default: `auto`).

| Flag | Long Option | Description |
| :--- | :--- | :--- |
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

`released` is `false` on a `--dry-run`; `floating_tag` is `null` when the run never reached it; `publish_commands` lists the rendered `[publish].commands` and is always an array. `publish_push_command` is the *planned* push line, so it is filled on a `--dry-run` report and `null` on a real run, whose pushes have already happened: read it before the run and confirm the refs afterwards with `git ls-remote origin`.

---

### `cutver plan`

Computes the release the workspace is about to cut and reports it without changing anything: no file, no git mutation, no `[preflight]`, no `[hooks]`, no `[publish] commands` and no plugin dispatch — not even `on_pre_bump`, which `bump --dry-run` still dispatches. A dirty tree on the wrong branch is the normal case, because that is the state CI asks the question in.

```bash
cutver plan [LEVEL] [--json] [--skip-preflight <STEP>]... [--first-release]
```

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| | `--json` | Prints the plan as one JSON object on stdout instead of the plan tree. |
| | `--skip-preflight <STEP>` | Marks a named preflight step as skipped; a plan never runs one (can be repeated). |
| `-fr`, `--fr` | `--first-release` | Plans the initial release: the tag carries the version the manifests already declare. |
| `-c <PATH>` | `--config <PATH>` | Explicit path to `cutver.toml`. |

`notes` is the rendered changelog entry, produced without writing the file, and `phases` is the release in order (`validate`, `mutate`, `publish`) with the steps each phase would run. The guards that abort a real `bump` — dirty tree, required branch, an existing release tag, an invalid tag name — arrive as one-line `blockers`, and the command still exits `0`; exit `1` means no plan could be computed. Every key, including `gates`, is documented in the [machine contract](../internals/machine-contract.md).

---

### `cutver publish`

Closes a release that `bump --defer-publish` prepared: it creates the annotated tag, pushes it, moves the floating major tag, runs `[publish].commands` and dispatches the `on_post_release` plugins. Validation fails closed before any of that, naming the command that resolves the problem and stating that nothing was written; an existing-tag error names the commit it points at (`src/bump/types.rs:85-88`). Every declared `[[artifact]]` glob is also verified after validation and before the tag: one that matches fewer files than its `at_least` fails closed naming the glob and the counts, and `--dry-run` reports that same verdict. Which inputs are validated, and why the split exists, live in [Release Pipeline Order and Atomic Guarantees](../internals/pipeline.md).

```bash
cutver publish --tag <TAG> [OPTIONS]
```

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| | `--tag <TAG>` | Release tag to publish. It must be a valid git ref, and it must equal the version the manifests declare at `HEAD`, so the tag is derived from the release commit instead of trusted from the command line. |
| | `--commit <SHA>` | The release commit the caller verified. When given it must be `HEAD`; omit it to publish whatever `HEAD` is. |
| | `--notes-file <PATH>` | Release notes for the `on_post_release` payload, which is what renders the draft release. The file is the command's input rather than a change in the tree, so a path that names a directory is refused before the clean-tree check; `-` reads the notes from standard input instead. An empty stream — or one carrying only whitespace — is refused before the tag, the pushes and the commands, because a release published with no notes cannot be corrected by re-running. |
| | `--require-gate <NAME>` | Refuses to publish unless this gate has an attestation for the commit being published (can be repeated). It adds to the gates `[gates] require` already declares, which `publish` enforces whether or not the flag is given; a gate named both ways is required once. The refusal names the gate and the commit, and a recorded `failure` refuses too. `[gates] dir` is exempt from the clean-tree check exactly like `--notes-file`: an attestation is an input, not a change in the tree. See [`cutver attest`](#cutver-attest). |
| | `--dry-run` | Validates and prints the plan without creating a tag, pushing anything or running a command. |
| `-c <PATH>` | `--config <PATH>` | Explicit path to `cutver.toml`. |

```bash
cutver bump minor --defer-publish --report .cutver-release.json
cutver publish --tag v1.3.0 --commit "$(git rev-parse HEAD)" --notes-file RELEASE_NOTES.md
```

---

### `cutver attest`

Records a gate's verdict for a commit, so `publish` can require it: `cutver attest <GATE> --commit <SHA> --conclusion success|failure [--run <URL>]` writes one JSON object into `[gates] dir` (default `.cutver/attestations`). The verdict is recorded, never verified: `cutver` signs nothing and talks to no network, so an attestation is only as trustworthy as the machine that wrote it ([the trust boundary](configuration.md#gates)).

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| `[GATE]` | | The gate this attestation covers. It becomes the file name inside `[gates] dir`, so only letters, digits, `-`, `_` and an inner `.` are accepted. |
| | `--commit <SHA>` | The commit the gate verified, as the full sha `git rev-parse HEAD` prints. A tag, a branch or a short form is refused: the attestation is matched against the release commit. |
| | `--conclusion <VERDICT>` | `success` or `failure`, and nothing else. A recorded `failure` refuses a release that requires the gate, in the same voice as a missing attestation. |
| | `--run <URL>` | The run that produced the verdict, stored with it and reported by `cutver plan --json`. |

---

### `cutver doctor`

Validates configuration syntax, verifies declared manifest files, and detects **version drift** across packages: `cutver doctor [--check-changelog] [--json]`.

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| | `--check-changelog` | Verifies that release entries in `CHANGELOG.md` match Git release tags. Headings are read through `[git] tag_prefix`, the same as `changelog latest` and `show`, so the commands agree: with `tag_prefix = "release-"`, `## release-1.2.0` is the entry for that tag. A tag whose entry is really absent is still drift, exit code `2`. |
| | `--json` | Prints the diagnosis as one JSON object on stdout — `ok`, a `checks` entry per check (`ok`/`drift`/`skipped`) and the drift arrays — and renders no dashboard or drift text. The exit code still reports the outcome: `0` clean, `2` drift, `1` a check could not run. See the [machine contract](../internals/machine-contract.md). |
| `-c <PATH>` | `--config <PATH>` | Explicit path to `cutver.toml`. |

---

### `cutver changelog`

Utilities to extract and inspect release notes from `CHANGELOG.md`; `latest` prints the newest entry to stdout and skips `[Unreleased]`:

```bash
cutver changelog latest [-H/--include-header] [--template <PATH>] [--json]
cutver changelog show <VERSION> [-H/--include-header] [--template <PATH>] [--json]
cutver changelog open [TARGET] [--print-url] [--browser <BROWSER>]
```

`changelog latest` and `show` read headings through `[git] tag_prefix`: with `tag_prefix = "release-"`,
`## release-1.2.0` is the entry for `1.2.0`. Exactly one marker is stripped, so a `v`/`V` heading still
matches under another prefix and a doubled marker is not read as a version. `changelog open` reads no
changelog: it builds the release URL from the same prefix.

`--json` builds its context from git and fails closed: an unreadable commit range (not a repository, an
unknown tag, an unusable `git`) goes to stderr with exit `1` instead of printing `"commits": []`, so a
consumer can tell an empty range from an unreadable one. `--template` keeps rendering the body with an
empty commit list. The object opens with `schemaVersion` and then the release context keys, including
`version` — the release version, not the schema one.

---

### `cutver plugin`

Installs, inspects, updates and removes plugins from the registry index. Requires a plugin-enabled build (`--features plugins`): a lean binary hides the subcommand and fails closed naming the flag.

```bash
cutver plugin add <NAME> [--version <VERSION>] [--registry <BASE>] [--event <EVENT>]... [--manifest-match <GLOB>]... [--dry-run] [--force]
cutver plugin list [--json]
cutver plugin restore [--registry <BASE>] [--dry-run]
cutver plugin update [NAME] [--version <VERSION>] [--registry <BASE>] [--dry-run]
cutver plugin info <NAME> [--registry <BASE>] [--json]
cutver plugin search [QUERY] [--registry <BASE>] [--json]
cutver plugin remove <NAME> [--keep-artifact]
```

`plugin` exits `1` on any failure: an unresolvable cache root, an unreachable registry, a digest mismatch, an unknown capability, or a declaration that already exists. The install flow, the digest trust model, the cache layout and the `[plugins.<name>]` keys live in the [plugin reference](plugins.md). Every `plugin` JSON surface opens with `schemaVersion`; `list` and `search` wrap their entries in `{"schemaVersion": 1, "plugins": [...]}`, because a bare array has no room for a version.

---

## Related Documentation

- [Getting Started with cutver](../guides/getting-started.md): Walkthrough of your first release.
- [Machine Contract](../internals/machine-contract.md): `schemaVersion`, every JSON surface, and exit codes.
- [Complete cutver.toml Configuration Reference](configuration.md): Reference for all configuration keys.
- [Language Manifests and Surgical Diff Guarantees](manifests.md): How cutver updates each file kind.
