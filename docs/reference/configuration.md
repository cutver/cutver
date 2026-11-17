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
| `package` | `string` | No | Optional package identity used to group changelog sections. Must be unique across manifests and may not contain whitespace or path separators. |
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
| `new_contributors` | `bool` | `false` | Emit a Level 3 `### New Contributors` section listing first-time authors when non-empty. |

Alongside the per-category strings, a template receives `sections`: the release already grouped, in
canonical order, so a template can render headings without grouping the flat commit list itself. The
remaining variables are listed in [Customizing Release Templates](../templates/release-notes.md).

| Key | Type | Description |
| :--- | :--- | :--- |
| `section.heading` | `string` | Section heading, for example `"✨ Features"` or `"🛠️ Maintenance"`. |
| `section.entries` | `list<commit>` | Commits under that heading, with the same fields `commits` entries carry. |
| `section.is_breaking` | `bool` | `true` only for the breaking section, which holds every breaking entry and no other section holds one. |

A section left with no entries is omitted, so a loop never renders an empty heading. With no commits
at all, `sections` is empty and `all_changes` keeps rendering `fallback_entry`.

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
| `[hooks]` | `pre_bump` | `string` | `None` | Shell command executed before manifest edits. Aborts the release cleanly on non-zero exit. Bounded by `default_timeout`. |
| `[hooks]` | `post_bump` | `string` | `None` | Shell command executed after manifest edits, before git staging. Bounded by `default_timeout`. |
| `[hooks]` | `default_timeout` | `integer` | `None` | Optional timeout in seconds applied to both hooks, the same unit and the same opt-in default as `[preflight] default_timeout` and `[publish] default_timeout`; `0` is rejected. A hook that outlives it is killed together with its child processes and the release fails with an error naming the hook, its command and the limit. The rollback restores the manifest and changelog files cutver wrote — `pre_bump` fails before anything is written, `post_bump` fails before the release commit — but a file the hook itself created is left on disk, because it is not cutver's to delete. Without the key a hook is never killed, which is the pre-existing behavior. |
| `[publish]` | `push` | `bool` | `false` | When `true`, pushes `refs/heads/<branch>` when the release commit is created, then `refs/tags/<release tag>`, then the floating major tag (when `[git] floating_major_tag` is enabled) — one `git push origin <ref>` per ref, in that order. A single-phase `cutver bump` sends all three in one run; `bump --defer-publish` stops after the branch push and `cutver publish` sends the two tag pushes. No push carries `--tags`, so a rejected tag cannot roll the branch update back. |
| `[publish]` | `commands` | `list<string>` | `[]` | Shell commands executed after the release tag exists: in the closing phase (`cutver publish`) for a release cut in two phases, in the same run for a single-phase `cutver bump`. |
| `[publish]` | `default_timeout` | `integer` | `None` | Optional timeout in seconds applied to each publish command. |

A hook runs in its own process group on unix, which is what lets the timeout kill its whole tree. The
consequence is that an interactive Ctrl-C reaches `cutver`, not the running hook: a hook that reads the
terminal does not see the interrupt, and may end up in the background until the timeout or the release
kills it. Hooks that prompt on a terminal therefore need a terminal of their own.

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
