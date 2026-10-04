# CLI Command Reference

Comprehensive command-line interface documentation for `cutver`.

All commands support the `--help` flag for quick reference:
```bash
cutver --help
cutver bump --help
cutver doctor --help
cutver init --help
cutver changelog --help
```

---

## Exit Codes

`cutver` uses deterministic exit codes across all commands:

| Code | Meaning | Common Scenarios |
| :---: | :--- | :--- |
| `0` | **Success** | Release finished successfully, doctor checks passed, or notes emitted. |
| `1` | **General Error** | Configuration parse error, missing file, preflight test failure, or git safety violation. |
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

Calculates SemVer updates, runs preflights, applies atomic manifest changes, updates the changelog, commits, and tags.

```bash
cutver bump [LEVEL] [OPTIONS]
```

- `[LEVEL]`: Target SemVer level: `patch`, `minor`, `major`, or `auto` (default: `auto`).

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| | `--dry-run` | Simulates the release without writing to disk, committing, or pushing tags. |
| `-fr`, `--fr` | `--first-release` | Cuts an initial release tag without incrementing the manifest version. |
| | `--skip-preflight <STEP>` | Bypasses one or more named preflight checks (can be repeated). |
| `-c <PATH>` | `--config <PATH>` | Explicit path to `cutver.toml`. |

```bash
# Preview automated release calculated from conventional commits
cutver bump auto --dry-run

# Run automated release
cutver bump auto

# Skip a specific preflight step
cutver bump auto --skip-preflight lint
```

---

### `cutver doctor`

Validates configuration syntax, verifies declared manifest files, and detects **version drift** across packages.

```bash
cutver doctor [OPTIONS]
```

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| | `--check-changelog` | Verifies that release entries in `CHANGELOG.md` match Git release tags. |
| `-c <PATH>` | `--config <PATH>` | Explicit path to `cutver.toml`. |

---

### `cutver changelog`

Utilities to extract and inspect release notes from `CHANGELOG.md`:

```bash
# Print latest release notes to stdout (skipping [Unreleased])
cutver changelog latest [-H/--include-header] [--template <PATH>]

# Print release notes for an exact historical version
cutver changelog show <VERSION> [-H/--include-header] [--template <PATH>]

# Dispatch changelog or compare view in default browser
cutver changelog open
```

---

## Related Documentation

- [Getting Started with cutver](../guides/getting-started.md): Walkthrough of your first release.
- [Complete cutver.toml Configuration Reference](configuration.md): Reference for all configuration keys.
- [Language Manifests and Surgical Diff Guarantees](manifests.md): How cutver updates each file kind.
