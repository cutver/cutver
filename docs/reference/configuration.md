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
