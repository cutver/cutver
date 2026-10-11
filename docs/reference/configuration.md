# cutver.toml Configuration Reference

The canonical declarative configuration reference for `cutver`.

Release behavior is configured in `cutver.toml`, which discovery walks up to find rather than requiring at the repository root (§1); CLI flags (`--dry-run`, `--skip-preflight`) and the `CUTVER_PLUGIN_CACHE_DIR` environment variable change specific behaviours too.

---

## Configuration Discovery & Precedence

1. **Ancestor Directory Traversal**: When invoked without `--config`, `cutver` traverses parent directories looking for `cutver.toml`.
2. **Repository Boundary Stop**: Search terminates immediately upon reaching a `.git` directory or submodule boundary.
3. **CLI Override**: `-c <PATH>` or `--config <PATH>` overrides automatic discovery.
4. **Root-Relative Resolution**: All paths specified inside `cutver.toml` resolve relative to the directory containing `cutver.toml`.

---

## Full Configuration Template

```toml
# cutver.toml

[version]
current_source = "Cargo.toml"     # Optional: explicit path to primary manifest
strategy = "conventional"         # "conventional", "manual", or "plugin"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
primary = true                    # Source of truth for current version

[[manifest]]
path = "package.json"
kind = "json"
field = "version"                 # Dotted JSON key path

[[manifest]]
path = "pyproject.toml"
kind = "pyproject"
table = "project"                 # "project" (PEP 621) or "tool.poetry"

[[manifest]]
path = "android/app/build.gradle.kts"
kind = "gradle"
version_name_field = "versionName"
version_code_field = "versionCode"

[[manifest]]
path = "version.txt"
kind = "regex"
pattern = 'release/v\d+\.\d+\.\d+'
replacement = 'release/{{version}}'

[preflight]
default_timeout = 300             # Optional; no default is applied. `cutver init` scaffolds 300
tests = "cargo test"
lint  = { command = "cargo clippy -- -D warnings", timeout = 180 }

[changelog]
path = "CHANGELOG.md"
format = "keep-a-changelog"
mode = "conventional"             # "conventional" or "template"
entry_template = "Maintenance and updates."
template_file = ".github/templates/cutver/RELEASE.md"
full_template = false
header_template = "## [{{ tag }}] - {{ date }}"
include_scopes = true
fallback_entry = "Maintenance and updates."
ignore_release_commits = true     # Exclude self-referential release commits
ignore_scopes = ["internal"]      # Exclude specific commit scopes

[git]
tag_prefix = "v"
floating_major_tag = true         # Create and push floating major tags (e.g. v1)
commit_message = "chore(release): v{version}"
require_clean_tree = true         # Require zero uncommitted changes
require_branch = "main"           # Branch restriction: string or array of strings

[hooks]
post_bump = "cargo check --workspace" # Executed after manifest updates, before staging

[publish]
push = true                       # Push commit and tags upstream
commands = [
  "cargo publish"
]
```

---

## Configuration Tables

### `[version]`

`cutver` does not read a `[workspace]` table: `mode` and `members` are ignored if present. Independent per-package releases remain future work ([design](../internals/design.md)).

| Table | Key | Type | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `[version]` | `strategy` | `string` | `"manual"` | Bump strategy: `"manual"`, `"conventional"` (commits), or `"plugin"`. |
| `[version]` | `plugin` | `string` | `None` | With `strategy = "plugin"`, names the `versioning.v1` plugin; omit it when exactly one declares that capability. |
| `[version]` | `current_source` | `string` | `None` | Path to primary manifest. If omitted, first `[[manifest]]` is used. |

---

### `[project]`

| Key | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `name` | `string` | `None` | Optional project name. Parsed, but the release pipeline does not consume it today. |

---

### `[[manifest]]`

Declares one target file to update. Multiple `[[manifest]]` blocks can be specified.

| Key | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `path` | `string` | Yes | Path to the manifest file relative to `cutver.toml`. |
| `kind` | `string` | Yes | Parser kind: `"cargo-package"`, `"json"`, `"pyproject"`, `"gradle"`, `"regex"`, or `"plugin"`. |
| `primary` | `bool` | No | When `true`, designates this manifest as primary source of truth. |
| `field` | `string` | For `json` | Dotted field path to version string (e.g. `"version"`). Required for `json`. |
| `table` | `string` | For `pyproject` | TOML table holding version: `"project"` (PEP 621) or `"tool.poetry"`. |
| `version_name_field` | `string` | For `gradle` | Field name updated to target SemVer string. Required for `gradle`. |
| `version_code_field` | `string` | For `gradle` | Integer field name incremented on every bump. Required for `gradle`. |
| `pattern` | `string` | For `regex` | Regular expression capturing the version substring. |
| `replacement` | `string` | For `regex` | Replacement template string; `{{version}}` is substituted. |
| `plugin` | `string` | No | For `kind = "plugin"` only: names the `manifest.v1` plugin to use. When omitted, `cutver` matches the declared path against the plugins' `manifest_match` globs. Requires a plugin-enabled build. |

---

### `[changelog]`

| Key | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `path` | `string` | `None` | Changelog file to update. Without it, no changelog is rendered. |
| `format` | `string` | `"keep-a-changelog"` | `"keep-a-changelog"` renders with the built-in renderer; `"plugin"` dispatches a `changelog.v1` plugin instead. |
| `plugin` | `string` | `None` | With `format = "plugin"`, names the plugin to use. Omit it when exactly one declares `changelog.v1`. |
| `mode` | `string` | `"conventional"` | `"conventional"` groups entries by commit type. A template is used whenever `template` or `template_file` is set, or when `mode = "template"`. |
| `template_file` | `string` | `None` | Template file, used when set and `template` is unset; when both are set, `template` wins and this is ignored. A template is used whenever either is set, or when `mode = "template"`; with `mode = "template"` and neither set, `entry_template` is rendered. |
| `entry_template` | `string` | `""` | Entry rendered when `mode = "template"` and neither `template` nor `template_file` is set. |
| `template` | `string` | `None` | Inline MiniJinja template string; takes precedence over `template_file`. |
| `full_template` | `bool` | `false` | When `true`, the template controls the release heading instead of the default prefix. |
| `header_template` | `string` | `None` | MiniJinja template evaluated for the release heading. |
| `include_scopes` | `bool` | `true` | Prefix changelog entries with `**scope**:`. |
| `fallback_entry` | `string` | `"Maintenance and updates."` | Bullet rendered when no commits match a category. |
| `ignore_release_commits` | `bool` | `true` | Exclude self-referential release commits from the changelog. |
| `ignore_scopes` | `list<string>` | `[]` | Exclude commits whose scope matches one of these values. |

All of these are accepted. Only `template` and `plugin` are absent from the full configuration template
above — `template` is an inline template string, an alternative to `template_file`.

### `[preflight]`, `[git]`, `[hooks]`, & `[publish]`

| Table | Key | Type | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `[preflight]` | `default_timeout` | `integer` | `None` | Optional timeout in seconds applied to steps without their own; `cutver init` scaffolds `300`. |
| `[preflight]` | `<step_name>` | `string \| table` | `None` | Command string or `{ command = "...", timeout = <sec> }`. |
| `[git]` | `tag_prefix` | `string` | `"v"` | Tag prefix (e.g. `"v"` -> `v1.2.0`). |
| `[git]` | `floating_major_tag` | `bool` | `false` | Automatically maintain floating major tag (e.g. `v1`). |
| `[git]` | `commit_message` | `string` | `"chore(release): v{version}"` | Release commit message template. |
| `[git]` | `require_clean_tree` | `bool` | `true` | Abort if uncommitted changes exist. |
| `[git]` | `require_branch` | `string \| list` | `None` | Branch or list of branches allowed to cut releases. |
| `[hooks]` | `pre_bump` | `string` | `None` | Parsed, but the release pipeline does not execute it today; only `post_bump` runs. |
| `[hooks]` | `post_bump` | `string` | `None` | Shell command executed after manifest edits, before git staging. |
| `[publish]` | `push` | `bool` | `false` | When `true`, runs `git push origin <branch> --tags`. |
| `[publish]` | `commands` | `list<string>` | `[]` | Shell commands executed post-tag/post-push. |
| `[publish]` | `default_timeout` | `integer` | `None` | Optional timeout in seconds applied to each publish command. |

---

### `[plugins.<name>]`

Each plugin gets its own table, written by `cutver plugin add` rather than by hand. The keys
(`runtime`, `source`, `hash`, `command`, `capabilities`, `events`, `manifest_match`, `permissions`,
`timeout_seconds`) and the two digest gates are documented in the
[plugin reference](plugins.md).

---

## Related Documentation

- [Language Manifests and Surgical Diff Guarantees](manifests.md): In-depth formatting and AST preservation guarantees.
- [CLI Commands and Exit Codes](cli.md): CLI command arguments, flags, and exit codes.
- [Installing and Managing Plugins](plugins.md): The `[plugins.<name>]` table, install flow, digest verification, and cache location.
- [Customizing Release Templates](../templates/release-notes.md): ReleaseContext variables, filters, and templates.
