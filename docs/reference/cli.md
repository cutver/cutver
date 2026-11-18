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

---

## Commands

### `cutver init`

Scans project files, detects package manifests, determines primary source of truth, and scaffolds `cutver.toml`, a starter `CHANGELOG.md`, and default release notes template.

```bash
cutver init [OPTIONS]
```

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

- `[LEVEL]`: Target SemVer level: `patch`, `minor`, `major`, or `auto` (default: `auto`).

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
# Preview automated release calculated from conventional commits
cutver bump auto --dry-run

# Run automated release
cutver bump auto

# Skip a specific preflight step
cutver bump auto --skip-preflight lint

# Commit and push the branch, and leave the tag and publishing to a later step
cutver bump minor --defer-publish
```

#### Machine-readable result

`--json` and `--report` are independent: either, both, or neither. Both describe a real and a `--dry-run` run alike, and a failed run writes no report. The object holds `released`, `deferred`, `version`, `tag`, `commit`, `floating_tag`, `publish_push_command` and `publish_commands`; field order is part of the contract.

```json
{"released":true,"deferred":false,"version":"0.19.0","tag":"v0.19.0","commit":"abc1234","floating_tag":"v0","publish_push_command":null,"publish_commands":["cargo publish"]}
```

`released` is `false` on a `--dry-run`; `deferred` is `true` when the run stopped after the branch push; `floating_tag` is `null` when the run never reached it; `publish_commands` lists the rendered `[publish].commands` and is always an array.

`publish_push_command` is the *planned* push command line, so it is filled on a `--dry-run` report and `null` on a real run: by the time a real run writes the object the pushes have already happened, and `publish_commands` then names the commands it ran. Read the push line before the run, and confirm the refs afterwards with `git ls-remote origin`.

---

### `cutver publish`

Closes a release that `bump --defer-publish` prepared: it creates the annotated tag, pushes it, moves the floating major tag, runs `[publish].commands` and dispatches the `on_post_release` plugins. Validation fails closed before any of that, naming the command that resolves the problem and stating that nothing was written; an existing-tag error names the commit it points at (`src/bump/types.rs:85-88`). Which inputs are validated, and why the split exists, live in [Release Pipeline Order and Atomic Guarantees](../internals/pipeline.md).

```bash
cutver publish --tag <TAG> [OPTIONS]
```

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| | `--tag <TAG>` | Release tag to publish. It must be a valid git ref, and it must equal the version the manifests declare at `HEAD`, so the tag is derived from the release commit instead of trusted from the command line. |
| | `--commit <SHA>` | The release commit the caller verified. When given it must be `HEAD`; omit it to publish whatever `HEAD` is. |
| | `--notes-file <PATH>` | Release notes for the `on_post_release` payload, which is what renders the draft release. The file is the command's input rather than a change in the tree; `-` reads the notes from standard input instead. An empty stream — or one carrying only whitespace — is refused before the tag, the pushes and the commands, because a release published with no notes cannot be corrected by re-running. |
| | `--dry-run` | Validates and prints the plan without creating a tag, pushing anything or running a command. |
| `-c <PATH>` | `--config <PATH>` | Explicit path to `cutver.toml`. |

```bash
cutver bump minor --defer-publish --report .cutver-release.json
cutver publish --tag v1.3.0 --commit "$(git rev-parse HEAD)" --notes-file RELEASE_NOTES.md
```

---

### `cutver doctor`

Validates configuration syntax, verifies declared manifest files, and detects **version drift** across packages.

```bash
cutver doctor [OPTIONS]
```

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| | `--check-changelog` | Verifies that release entries in `CHANGELOG.md` match Git release tags. Headings are read through `[git] tag_prefix`, the same as `changelog latest` and `show`, so the commands agree: with `tag_prefix = "release-"`, `## release-1.2.0` is the entry for that tag. A tag whose entry is really absent is still drift, exit code `2`. |
| `-c <PATH>` | `--config <PATH>` | Explicit path to `cutver.toml`. |

---

### `cutver changelog`

Utilities to extract and inspect release notes from `CHANGELOG.md`:

```bash
# Print latest release notes to stdout (skipping [Unreleased])
cutver changelog latest [-H/--include-header] [--template <PATH>] [--json]

# Print release notes for an exact historical version
cutver changelog show <VERSION> [-H/--include-header] [--template <PATH>] [--json]

# Dispatch changelog or compare view in default browser
cutver changelog open [TARGET] [--print-url] [--browser <BROWSER>]
```

`changelog latest` and `show` read headings through `[git] tag_prefix`: with `tag_prefix = "release-"`,
`## release-1.2.0` is the entry for `1.2.0`. Exactly one marker is stripped, so a `v`/`V` heading still
matches under another prefix and a doubled marker is not read as a version. `changelog open` reads no
changelog: it builds the release URL from the same prefix.

`--json` builds its context from git and fails closed: an unreadable commit range (not a repository, an
unknown tag, an unusable `git`) goes to stderr with exit `1` instead of printing `"commits": []`, so a
consumer can tell an empty range from an unreadable one. `--template` keeps rendering the body with an
empty commit list.

---

### `cutver plugin`

Installs, inspects, updates and removes plugins from the registry index. Requires a plugin-enabled build
(`--features plugins`): a lean binary hides the subcommand and fails closed naming the flag.

```bash
cutver plugin add <NAME> [--version <VERSION>] [--registry <BASE>] [--event <EVENT>]... [--manifest-match <GLOB>]... [--dry-run] [--force]
cutver plugin list [--json]
cutver plugin restore [--registry <BASE>] [--dry-run]
cutver plugin update [NAME] [--version <VERSION>] [--registry <BASE>] [--dry-run]
cutver plugin info <NAME> [--registry <BASE>] [--json]
cutver plugin search [QUERY] [--registry <BASE>] [--json]
cutver plugin remove <NAME> [--keep-artifact]
```

`plugin` exits `1` on any failure: an unresolvable cache root, an unreachable registry, a digest
mismatch, an unknown capability, or a declaration that already exists. The install flow, the digest
trust model, the cache layout and the `[plugins.<name>]` keys live in the
[plugin reference](plugins.md).

---

## Related Documentation

- [Getting Started with cutver](../guides/getting-started.md): Walkthrough of your first release.
- [Complete cutver.toml Configuration Reference](configuration.md): Reference for all configuration keys.
- [Language Manifests and Surgical Diff Guarantees](manifests.md): How cutver updates each file kind.
