# Cutver Technical Reference

> Outcome first: `cutver` provides a unified, deterministic release toolchain. This document serves as the high-level technical reference and index connecting user-facing configuration, manifest behaviors, and internal pipeline execution.

Detailed guides and specifications are partitioned into focused modular documents under `docs/`:

| Domain | Canonical Reference | Focus |
| :--- | :--- | :--- |
| **Configuration** | [Configuration Reference](../reference/configuration.md) | Complete `cutver.toml` schema and table options |
| **Manifests** | [Manifest Editors](../reference/manifests.md) | Format-specific editing and AST preservation rules |
| **CLI Commands** | [CLI Command Reference](../reference/cli.md) | Command arguments, options, and subcommand usage |
| **Templates** | [Release Notes & Changelog Templates](../templates/release-notes.md) | Template variables, commit objects, and filters |
| **Plugins** | [Plugins Reference](../reference/plugins.md) | Capability specifications and runtime management |
| **Architecture** | [Architecture & Design Decisions](design.md) | Design principles, roadmap, and core decisions |
| **Pipeline** | [Pipeline Execution & Atomic Guarantees](pipeline.md) | Two-phase execution, rollback, and process trees |

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

When `cutver bump auto` runs, it scans commit history from the latest tag matching `tag_prefix` up to `HEAD`:

- **Major Bump (`X.0.0`)**: Triggered if any commit contains a breaking change marker (`!` after type/scope, e.g. `feat!: ...`) or a `BREAKING CHANGE:` footer.
- **Minor Bump (`0.X.0`)**: Triggered if any commit has type `feat`.
- **Patch Bump (`0.0.X`)**: Default fallback for `fix`, `perf`, `refactor`, `chore`, `docs`, `test`, or non-conventional commits.

### Commit Filtering

By default, self-referential release commits are excluded so they do not pollute changelog categories:

- **`ignore_release_commits = true`** (default): Excludes commits like `chore(release): v1.2.0`, commits with scope `release`, and prefix descriptions matching release patterns.
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
- Updates use same-directory temporary files and atomic filesystem replacement (`rename`).
- Timeouts enforce complete process tree termination (`SIGKILL` to `-pgid` on Unix, `taskkill` on Windows).
- Failure before commit triggers best-effort RAII rollback and Git index unstaging.

Full implementation details and limitations are specified in [Pipeline Execution & Atomic Guarantees](pipeline.md).
