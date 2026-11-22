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

`cutver init` generates a real `cutver.toml`. The skeleton below shows the shape and nesting only; the [configuration tables reference](configuration-tables.md) is canonical, so the skeleton cannot drift from it.

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

A complete file that the loader accepts and the test suite executes — three manifests, preflight steps,
hooks, artifacts, gates and a publish command — is [the worked example cutver.toml](../../examples/cutver.toml).

---

## Unknown Keys

`cutver.toml` is not a permissive document: every unrecognised key is a hard
error naming the enclosing table and the offending key. This is deliberate — silently
ignored unknown keys once let a half-built `[workspace]` table look valid while the
documentation drifted from the loader.

```text
unknown configuration key 'mode' in [workspace] table in cutver.toml
```

Two key sets are open by design: `[preflight]`, where every key other than
`default_timeout` is a user-named step; and `[[manifest]]`, which accepts the union of
every `kind`'s keys, so a key annotated for one `kind` below (such as `pattern`) is accepted
on any manifest because `kind` selects the variant. An unknown key inside `[plugins.<name>]`
or its `permissions` sub-table is rejected the same way.

### Deprecated Keys

Three spellings still parse but are deprecated. Each one keeps working, and each one warns on stderr
in exactly one sentence naming the spelling, its replacement and the release that removes it:

| Deprecated | Replacement | Removed in |
| :--- | :--- | :--- |
| `[version] source` | `current_source` | 1.0.0 |
| `[[manifest]] kind = "toml"` | `kind = "cargo-package"` | 1.0.0 |
| `[[manifest]] kind = "pyproject-toml"` | `kind = "pyproject"` | 1.0.0 |

```text
warning: [version].source is deprecated, use current_source; it will be removed in 1.0.0
```

The warning goes to stderr, never stdout, so `--json` stays parseable; under a detected CI the same
sentence is also a `::warning::` annotation, and the exit code never changes. `cutver doctor --json`
carries the same sentences in its `warnings` array, and `cutver init --update` prints them, which are
the two migration surfaces [the contract](../../CONTRACT.md#2-configuration-backward-compatibility)
names. A key that was never valid is not deprecated: it still fails closed (see above), so a typo
never hides behind a warning.

---

## Plugin Filesystem Grants

A `[plugins.<name>.permissions] filesystem` entry is a capability, not a hint. Each entry is resolved
against the workspace root when `cutver.toml` loads — forward-slashed, without the Windows verbatim
prefix, lexically normalized — and a grant whose *real* directory falls outside the workspace is a
load error that names the directory it reached, before anything is mounted:

```text
plugin 'probe' grants filesystem entry '../outside', which resolves to '/tmp/outside' — outside the workspace root '/tmp/repo'.
  A sandboxed plugin may only reach the workspace: declare a path inside it, or drop the grant.
```

Resolution keeps all three values, and `cutver doctor` prints one row per grant: the spelling the
document declared, the guest path the plugin mounts and the canonical host directory behind it, plus
the plugin name and its capability set. A grant naming a directory that does not exist yet has no
canonical host path, and the row says so instead of guessing. `cutver doctor --json` carries the same
three values per grant under `plugins[].grants[]` (`declared`, `resolved`, `canonical`, and `location`:
`inside`, `outside` or `unresolved`), so a pipeline can report what a sandbox really opened without
scraping the dashboard.

---

## Configuration Tables

The tables are indexed here and specified in the [configuration tables reference](configuration-tables.md), which is canonical for every key, its type and its default:

- [`[version]`](configuration-tables.md#version): where the current version is read, and the versioning plugin that computes it.
- [`[project]`](configuration-tables.md#project): the optional project name.
- [`[[manifest]]`](configuration-tables.md#manifest): every file a release edits, and how each format is parsed.
- [`[[artifact]]`](configuration-tables.md#artifact): the files a release must produce before the tag.
- [`[gates]`](configuration-tables.md#gates): the attestation channel and the gates `publish` requires.
- [`[changelog]`](configuration-tables.md#changelog): the changelog path, how entries are grouped, and templates.
- [`[preflight]`, `[git]`, `[hooks]` and `[publish]`](configuration-tables.md): commands, git guards, hook timeouts and push order.
- [`[plugins.<name>]`](configuration-tables.md): one table per installed plugin, and the `config` table the plugin owns.

---

## Related Documentation

- [Language Manifests and Surgical Diff Guarantees](manifests.md): In-depth formatting and AST preservation guarantees.
- [CLI Commands and Exit Codes](cli.md): CLI command arguments, flags, and exit codes.
- [Installing and Managing Plugins](plugins.md): The `[plugins.<name>]` table, install flow, digest verification, and cache location.
- [Customizing Release Templates](../templates/release-notes.md): ReleaseContext variables, filters, and templates.
