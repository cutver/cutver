# cutver.toml Configuration Reference

The canonical declarative configuration reference for `cutver`.

Every release behavior in `cutver` is configured in `cutver.toml` at the root of your project or repository.

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
strategy = "conventional"         # "conventional" or "manual"

[workspace]
mode = "unified"                  # "unified" or "independent"
members = ["crates/*", "packages/*"]

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
default_timeout = 600             # Global timeout in seconds (default: 600)
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

### `[version]` & `[workspace]`

| Table | Key | Type | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `[version]` | `strategy` | `string` | `"conventional"` | Strategy for bumps: `"conventional"` (commits) or `"manual"`. |
| `[version]` | `current_source` | `string` | `None` | Path to primary manifest. If omitted, first `[[manifest]]` is used. |
| `[workspace]` | `mode` | `string` | `"unified"` | `"unified"` (synchronized workspace) or `"independent"`. |
| `[workspace]` | `members` | `list<string>` | `[]` | Glob patterns designating workspace members. |

---

### `[[manifest]]`

Declares one target file to update. Multiple `[[manifest]]` blocks can be specified.

| Key | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `path` | `string` | Yes | Path to the manifest file relative to `cutver.toml`. |
| `kind` | `string` | Yes | Parser kind: `"cargo-package"`, `"json"`, `"pyproject"`, `"gradle"`, or `"regex"`. |
| `primary` | `bool` | No | When `true`, designates this manifest as primary source of truth. |
| `field` | `string` | For `json` | Dotted field path to version string (e.g. `"version"`). Default: `"version"`. |
| `table` | `string` | For `pyproject` | TOML table holding version: `"project"` (PEP 621) or `"tool.poetry"`. |
| `version_name_field` | `string` | For `gradle` | Field name updated to target SemVer string. Default: `"versionName"`. |
| `version_code_field` | `string` | For `gradle` | Integer field name incremented on every bump. Default: `"versionCode"`. |
| `pattern` | `string` | For `regex` | Regular expression capturing the version substring. |
| `replacement` | `string` | For `regex` | Replacement template string; `{{version}}` is substituted. |

---

### `[preflight]`, `[git]`, `[hooks]`, & `[publish]`

| Table | Key | Type | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `[preflight]` | `default_timeout` | `integer` | `600` | Global timeout in seconds for steps without an explicit timeout. |
| `[preflight]` | `<step_name>` | `string \| table` | `None` | Command string or `{ command = "...", timeout = <sec> }`. |
| `[git]` | `tag_prefix` | `string` | `"v"` | Tag prefix (e.g. `"v"` -> `v1.2.0`). |
| `[git]` | `floating_major_tag` | `bool` | `false` | Automatically maintain floating major tag (e.g. `v1`). |
| `[git]` | `commit_message` | `string` | `"chore(release): v{version}"` | Release commit message template. |
| `[git]` | `require_clean_tree` | `bool` | `true` | Abort if uncommitted changes exist. |
| `[git]` | `require_branch` | `string \| list` | `None` | Branch or list of branches allowed to cut releases. |
| `[hooks]` | `post_bump` | `string` | `None` | Shell command executed after manifest edits, before git staging. |
| `[publish]` | `push` | `bool` | `false` | When `true`, runs `git push origin <branch> --tags`. |
| `[publish]` | `commands` | `list<string>` | `[]` | Shell commands executed post-tag/post-push. |

---

## Related Documentation

- [Language Manifests and Surgical Diff Guarantees](manifests.md): In-depth formatting and AST preservation guarantees.
- [CLI Commands and Exit Codes](cli.md): CLI command arguments, flags, and exit codes.
- [Customizing Release Templates](../templates/release-notes.md): ReleaseContext variables, filters, and templates.
