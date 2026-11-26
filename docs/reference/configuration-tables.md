# cutver.toml Configuration Tables

> Outcome first: one section per `cutver.toml` table, with every key, its type, its default and what
> the loader does with it, so a key is looked up instead of guessed. Discovery, precedence, the
> unknown-key rule and the canonical skeleton are on the
> [configuration reference](configuration.md), which indexes these sections.

## `[version]`

`cutver` has no `[workspace]` table. Because unknown keys are now a hard error, declaring a `[workspace]` table fails the load instead of being ignored; independent per-package releases remain future work ([design](../internals/design.md)).

| Table | Key | Type | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `[version]` | `strategy` | `string` | `"manual"` | `"plugin"` routes version computation to a `versioning.v1` plugin; `"manual"` and `"conventional"` are accepted but no release code reads them, so the bump level comes from the CLI argument. |
| `[version]` | `plugin` | `string` | `None` | With `strategy = "plugin"`, names the `versioning.v1` plugin; omit it when exactly one declares that capability. |
| `[version]` | `current_source` | `string` | `None` | Path to primary manifest. If omitted, first `[[manifest]]` is used. |

---

## `[project]`

| Key | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `name` | `string` | `None` | Optional project name. Parsed, but the release pipeline does not consume it today. |

---

## `[[manifest]]`

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

## `[[artifact]]`

Declares files a release must produce. `publish` verifies every entry after validation and before the tag; `plan` reports the count it observes and never blocks. Checksums and the target matrix are not implemented: an entry checks that the files exist, not what their bytes hash to.

| Key | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `glob` | `string` | Required | Workspace-relative glob (`**`, `*`, `?`); it may not escape the workspace. |
| `at_least` | `integer` | `1` | Fewest files the glob must match. |
| `description` | `string` | `None` | Only used in messages. |

---

## `[gates]`

The attestation channel. `cutver attest` writes one JSON object per gate into `dir`, `cutver publish` refuses a release whose required gate is missing, failed or recorded for another commit, and `cutver plan --json` reports each gate's state. `require` is enforced, not just reported: a gate listed there must attest the release commit whether or not `--require-gate` repeats it on the command line. A `dir` naming the workspace root or walking upwards is refused, because the directory is exempt from the clean-tree guard exactly like a `--notes-file` — an input to the command, not a change in the tree. Attestations are data, not trust: `cutver` records and reads them, signs nothing and talks to no network, so an attestation is only as trustworthy as the machine that wrote it, and a forged file is a valid attestation to this tool.

| Key | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `dir` | `string` | `".cutver/attestations"` | Workspace-relative directory holding one JSON object per gate. |
| `require` | `list<string>` | `[]` | Gates `publish` requires and `cutver plan --json` reports, in this order. Names are file-safe: letters, digits, `-`, `_` and an inner `.`. |

## `[changelog]`

| Key | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `path` | `string` | `None` | Changelog file to update, or to create when it is absent: a declared path that does not exist yet is a normal state, and the absence is recorded so a rollback removes the file the run created. Without it, no changelog is rendered. |
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

Alongside the per-category strings, a template receives `sections`: the release already grouped, in canonical order, so a template can render headings without grouping the flat commit list itself. The remaining variables are listed in [Customizing Release Templates](../templates/release-notes.md).

| Key | Type | Description |
| :--- | :--- | :--- |
| `section.heading` | `string` | Section heading, for example `"✨ Features"` or `"🛠️ Maintenance"`. |
| `section.entries` | `list<commit>` | Commits under that heading, with the same fields `commits` entries carry. |
| `section.is_breaking` | `bool` | `true` only for the breaking section, which holds every breaking entry and no other section holds one. |

A section left with no entries is omitted, so a loop never renders an empty heading. With no commits at all, `sections` is empty and `all_changes` keeps rendering `fallback_entry`.

## `[preflight]`, `[git]`, `[hooks]`, & `[publish]`

| Table | Key | Type | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `[preflight]` | `default_timeout` | `integer` | `None` | Optional timeout in seconds applied to steps without their own; `cutver init` scaffolds `300`. |
| `[preflight]` | `<step_name>` | `string \| table` | `None` | Command string or `{ command = "...", timeout = <sec> }`. |
| `[git]` | `tag_prefix` | `string` | `"v"` | Tag prefix (e.g. `"v"` -> `v1.2.0`). |
| `[git]` | `floating_major_tag` | `bool` | `false` | Maintain the floating major tag (e.g. `v1`) on stable releases only: a prerelease publishes exactly its own version, so `v1` is never moved onto `1.2.0-alpha.1` (ADR-0005). |
| `[git]` | `commit_message` | `string` | `"chore(release): v{version}"` | Release commit message template. |
| `[git]` | `require_clean_tree` | `bool` | `true` | Abort if uncommitted changes exist. |
| `[git]` | `require_branch` | `string` | `None` | Single branch name allowed to cut releases. The type is `Option<String>` (`src/config/types.rs:257`); there is no list form. |
| `[hooks]` | `pre_bump` | `string` | `None` | Shell command executed before manifest edits. Aborts the release cleanly on non-zero exit. Bounded by `default_timeout`. |
| `[hooks]` | `post_bump` | `string` | `None` | Shell command executed after manifest edits, before git staging. Bounded by `default_timeout`. |
| `[hooks]` | `default_timeout` | `integer` | `None` | Optional timeout in seconds applied to both hooks, the same unit and the same opt-in default as `[preflight] default_timeout` and `[publish] default_timeout`; `0` is rejected. A hook that outlives it is killed together with its child processes and the release fails with an error naming the hook, its command and the limit. The rollback restores the manifest and changelog files cutver wrote — `pre_bump` fails before anything is written, `post_bump` fails before the release commit — but a file the hook itself created is left on disk, because it is not cutver's to delete. Without the key a hook is never killed, which is the pre-existing behavior. |
| `[publish]` | `push` | `bool` | `false` | When `true`, pushes `refs/heads/<branch>` when the release commit is created, then `refs/tags/<release tag>`, then the floating major tag (when `[git] floating_major_tag` is enabled and the release is stable) — one `git push origin <ref>` per ref, in that order. A single-phase `cutver bump` sends all three in one run; `bump --defer-publish` stops after the branch push and `cutver publish` sends the two tag pushes. No push carries `--tags`, so a rejected tag cannot roll the branch update back. |
| `[publish]` | `commands` | `list<string>` | `[]` | Shell commands executed after the release tag exists: in the closing phase (`cutver publish`) for a release cut in two phases, in the same run for a single-phase `cutver bump`. |
| `[publish]` | `default_timeout` | `integer` | `None` | Optional timeout in seconds applied to each publish command. |

A hook runs in its own container — a process group on unix, a job object on Windows — which is what lets the timeout kill its whole tree in one kernel call instead of a walk (about 1 ms against the 89–397 ms `taskkill /F /T` took, #384). `[preflight]` steps and `[publish].commands` run in the same container and under the same rules: a host that refuses a Windows job object falls back to `taskkill /F /T`, which is a walk, and a hook, step or command that spawns with `CREATE_BREAKAWAY_FROM_JOB` is denied by Windows, because leaving the container is what the timeout promises cannot happen. An interactive Ctrl-C therefore reaches `cutver`, not the running hook: a hook that reads the terminal may end up in the background until the timeout or the release kills it, and a hook that prompts needs a terminal of its own.

---

## `[plugins.<name>]`

Each plugin gets its own table, written by `cutver plugin add` rather than by hand. The keys
(`runtime`, `source`, `hash`, `command`, `capabilities`, `events`, `manifest_match`, `permissions`,
`timeout_seconds`, `config`) and the two digest gates are documented in the [plugin reference](plugins.md).

## `[plugins.<name>.config]`

A free-form table that belongs to the plugin, not to `cutver`: no host behavior depends on a key inside it, an unknown key there is not rejected the way one in the enclosing table is, and the host does not deliver it either — `permissions.env` is the only route by which an entry's values reach the plugin (`src/plugin/driver/wasm.rs:198-200`, `src/plugin/driver/process.rs::child_command`). The loader parses the table (`src/plugin/types.rs:186`) and `cutver plugin update` rewrites only `source` and `hash`, leaving this one untouched (`src/plugin/manage/update.rs:1-6`): it is the only table in `cutver.toml` whose contents `cutver` neither validates nor reads. The reference implementation is `github-publish`, in this repository's own `cutver.toml`, which records why each key is there (`cutver.toml:84-94`):

```toml
[plugins.github-publish.config]
repository = "cutver/cutver"
draft = true
```

## Related Documentation

- [The Configuration Reference](configuration.md): discovery, precedence, the unknown-key rule and the full skeleton.
- [Language Manifests and Surgical Diff Guarantees](manifests.md): the editor behind each `kind`, and what it preserves.
- [Installing and Managing Plugins](plugins.md): the plugin tables' install flow, digest verification and cache location.
- [Customizing Release Templates](../templates/release-notes.md): the `[changelog]` template variables and filters.
