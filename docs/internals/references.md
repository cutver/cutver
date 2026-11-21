# Cutver Technical Reference

> Outcome first: `cutver` provides a unified, deterministic release toolchain. This document serves as the high-level technical reference and index connecting user-facing configuration, manifest behaviors, and internal pipeline execution.

Detailed guides and specifications are partitioned into focused modular documents under `docs/`:

| Domain | Canonical Reference | Focus |
| :--- | :--- | :--- |
| **Configuration** | [Configuration Reference](../reference/configuration.md) | Discovery, precedence, the unknown-key rule, and the canonical skeleton |
| **Configuration tables** | [Configuration Tables Reference](../reference/configuration-tables.md) | Per-table keys, types and defaults for every `cutver.toml` section |
| **Manifests** | [Manifest Editors](../reference/manifests.md) | Format-specific editing and AST preservation rules |
| **CLI Commands** | [CLI Command Reference](../reference/cli.md) | Exit codes, the `--ci` channel, and the five release commands |
| **CLI Diagnostics** | [Diagnostics, Changelog, Open and Plugins](../reference/cli-tools.md) | The four commands that report, read or reach outside the repository |
| **Templates** | [Release Notes & Changelog Templates](../templates/release-notes.md) | Template variables, commit objects, and filters |
| **Plugins** | [Plugins Reference](../reference/plugins.md) | Capability specifications and runtime management |
| **Architecture** | [Architecture & Design Decisions](design.md) | Design principles, roadmap, and core decisions |
| **Pipeline** | [Pipeline Execution & Atomic Guarantees](pipeline.md) | Two-phase execution, rollback, and process trees |
| **Contributing** | [The Local Gate & Source Line Budget](local-gate.md) | The one command that mirrors CI locally, and the ratchet that keeps `src/` under 400 lines |
| **Machine output** | [Machine Contract](machine-contract.md) | `schemaVersion`, the surface index, exit codes, and the error envelope |
| **Machine surfaces** | [Command Surfaces](machine-contract-surfaces.md) | The CI output channel, `doctor --json`, and `plan --json` as data |

---

## Configuration Discovery Algorithm

When invoked without an explicit `-c / --config` argument, `cutver` locates its configuration dynamically:

1. Begins traversal at the current working directory.
2. Traverses upward through parent directories looking for `cutver.toml`.
3. Stops traversal immediately upon reaching a Git repository boundary (`.git` directory or submodule file).
4. All relative paths defined inside `cutver.toml` resolve relative to the configuration file's own directory.

---

## Primary Manifest Deduction

`cutver` synchronizes multiple manifests from a single source of truth:

1. **Explicit Primary**: Any manifest entry declaring `primary = true` serves as the primary version source.
2. **Explicit Current Source**: `[version] current_source = "path/to/manifest"` explicitly overrides the version reader path.
3. **Implicit Convention**: If neither `primary = true` nor `current_source` is declared, the **first manifest entry** in `cutver.toml` is used as the primary manifest by convention.
4. **Validation Guard**: Multiple manifests marked with `primary = true` or conflicting `current_source` definitions result in an immediate configuration validation error.

---

## Manifest Editor Capabilities

`cutver` selects format-specific editors based on the declared `kind` to preserve file formatting, comments, and structure:

| Kind | Target | Editor Engine | Preservation Scope |
| :--- | :--- | :--- | :--- |
| `cargo-package` | `Cargo.toml` | `toml_edit` AST | Preserves table structure, comments, whitespace, and key order. |
| `pyproject` | `pyproject.toml` | `toml_edit` AST | Supports PEP 621 (`[project]`) and Poetry (`[tool.poetry]`). |
| `json` | `*.json` | Custom byte-span scanner | Replaces exact version slice; preserves whitespace and key order. |
| `gradle` | `build.gradle(.kts)` | Regex byte replacement | Updates `versionName` string and increments `versionCode` integer. |
| `regex` | Arbitrary text | Regex pattern match | Replaces match with configured `replacement`, substituting `{{version}}`. |
| `plugin` | Dynamic files | `manifest.v1` plugin | Delegates read/write operations to an installed microkernel plugin. |

---

## Conventional Commit Bump Rules

When `cutver bump auto` runs, it scans commit history from the latest tag matching `tag_prefix` up to `HEAD`, and the highest signal in that range wins:

| Signal in the range | Deduced bump | Example commit |
| :--- | :--- | :--- |
| A breaking change marker (`!` after the type or scope) or a `BREAKING CHANGE:` footer | **Major** (`X.0.0`) | `feat(api)!: redesign auth flow` |
| Type `feat` | **Minor** (`0.X.0`) | `feat(cli): add quiet mode` |
| `fix`, `perf`, `refactor`, `chore`, `docs`, `test`, or a non-conventional message | **Patch** (`0.0.X`) | `fix(git): unstage index on error` |

An explicit level (`cutver bump patch`, `minor` or `major`) overrides the deduction instead of being weighed against it.

### Commit Filtering

By default, self-referential release commits are excluded so they do not pollute changelog categories:

- **`ignore_release_commits = true`** (default): Excludes a commit only when its type is `chore`, its scope is `release`, and its description is a bare version token — a single token (optionally prefixed `v`/`V`) containing a digit and either a separator (`.`, `+`, `-`, `_`, `~`) or only digits, so `chore(release): v1.2.0`, `chore(release): 0.16.0`, `chore(release): 2026.11`, `chore(release): 1.0.0_rc1` and `chore(release): 1.2.3-α` all qualify. The token may be followed by marker groups — a whitespace-separated tail of bracketed `[...]` or parenthesized `(...)` groups such as `[skip ci]`, `[SKIP CI]`, `[bump: minor]` or `(hotfix)`. This is a shape test, not a guarantee for every conceivable scheme, and commits carrying content are kept: `fix(release)`/`feat(release)` commits, a single token with no version separator (`chore(release): step1`), and `chore(release)` commits like `chore(release): pin the github-publish plugin to v0.2.1 (#236)`.
- **`ignore_scopes = ["internal", "wip"]`** (default: `[]`): Excludes commits matching specified scopes case-insensitively.

Filtered commits are excluded from both Keep-a-Changelog sections and the template context `commits` list.

---

## Release Notes & Template Engine

Release notes can be generated using built-in Keep-a-Changelog categorisation or custom MiniJinja templates:

- **Template Context**: Templates receive a rich `ReleaseContext` containing `version`, `tag`, `date`, `compare_url`, pre-formatted category strings (`features`, `fixes`, `breaking`, `maintenance`), and structured `commits` lists.
- **Header Control**: Setting `full_template = true` or beginning a template with `# ` or `## ` bypasses the default heading prefix, granting templates full output ownership.
- **Auto-Escaping Disabled**: Markdown syntax characters (`*`, `<`, `>`, `&`) are never HTML-escaped.

Detailed template variables and examples are documented in [Release Notes & Changelog Templates](../templates/release-notes.md).

---

## Standard CLI Exit Codes

`cutver` subcommands adhere to consistent exit code conventions:

| Exit Code | Meaning | Commands |
| :---: | :--- | :--- |
| `0` | Successful execution | All commands |
| `1` | Configuration error, file read failure, preflight check failure, or unparseable input | All commands |
| `2` | Version drift or changelog drift detected | `cutver doctor` |

- **Exit Code 0**: Operation completed successfully without warnings or unhandled conditions.
- **Exit Code 1**: Recoverable operational failure (syntax error in `cutver.toml`, missing manifest path, preflight step failure, or uncommitted Git working tree).
- **Exit Code 2**: Specialized diagnostic failure emitted exclusively by `cutver doctor` when manifest versions differ or git tags diverge from `CHANGELOG.md`.

---

## Atomic Safety & Process Guarantees

File modifications and process execution follow fail-fast atomic guarantees:

- Manifests are modified only after all preflight steps succeed.
- Updates use same-directory temporary files and atomic filesystem replacement (`rename`), retried a bounded number of times for the transient collisions Windows reports when the destination is held open.
- Timeouts enforce complete process tree termination (`SIGKILL` to `-pgid` on Unix, `taskkill` on Windows).
- Failure before commit triggers best-effort RAII rollback and Git index unstaging.

Full implementation details and limitations are specified in [Pipeline Execution & Atomic Guarantees](pipeline.md).
