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

`cutver init` generates a real `cutver.toml`. The skeleton below shows the shape
and nesting only; the configuration tables further down are canonical, so the
skeleton cannot drift from them.

```toml
# cutver.toml — shape only
[version]
strategy = "manual"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
[changelog]
path = "CHANGELOG.md"
[preflight]
tests = "cargo test"
[git]
tag_prefix = "v"
[hooks]
post_bump = "cargo check"
[publish]
push = true
```

---

## Unknown Keys

`cutver.toml` is not a permissive document: every unrecognised key is a hard
error naming the enclosing table and the offending key. This is deliberate —
silently ignored unknown keys once let a half-built `[workspace]` table look
valid while the documentation drifted from the loader.

```text
unknown configuration key 'mode' in [workspace] table in cutver.toml
```

Two key sets are open by design: `[preflight]`, where every key other than
`default_timeout` is a user-named step; and `[[manifest]]`, which accepts the
union of every `kind`'s keys, so a key annotated for one `kind` below (such as
`pattern`) is accepted on any manifest because `kind` selects the variant. An
unknown key inside `[plugins.<name>]` or its `permissions` sub-table is rejected
the same way.

---

## Configuration Tables

### `[version]`

`cutver` has no `[workspace]` table. Because unknown keys are now a hard error,
declaring a `[workspace]` table fails the load instead of being ignored;
independent per-package releases remain future work ([design](../internals/design.md)).

| Table | Key | Type | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `[version]` | `strategy` | `string` | `"manual"` | `"plugin"` routes version computation to a `versioning.v1` plugin; `"manual"` and `"conventional"` are accepted but no release code reads them, so the bump level comes from the CLI argument. |
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
| `table` | `string` | No | For `pyproject`: TOML table holding the version (`"project"` for PEP 621, `"tool.poetry"` for Poetry). Auto-detected when omitted. |
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

### `[preflight]`, `[git]`, `[hooks]`, & `[publish]`

| Table | Key | Type | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `[preflight]` | `default_timeout` | `integer` | `None` | Optional timeout in seconds applied to steps without their own; `cutver init` scaffolds `300`. |
| `[preflight]` | `<step_name>` | `string \| table` | `None` | Command string or `{ command = "...", timeout = <sec> }`. |
| `[git]` | `tag_prefix` | `string` | `"v"` | Tag prefix (e.g. `"v"` -> `v1.2.0`). |
| `[git]` | `floating_major_tag` | `bool` | `false` | Automatically maintain floating major tag (e.g. `v1`). |
| `[git]` | `commit_message` | `string` | `"chore(release): v{version}"` | Release commit message template. |
| `[git]` | `require_clean_tree` | `bool` | `true` | Abort if uncommitted changes exist. |
| `[git]` | `require_branch` | `string` | `None` | Single branch name allowed to cut releases. The type is `Option<String>` (`src/config/types.rs:257`); there is no list form. |
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
