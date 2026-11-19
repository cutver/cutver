# CLI Reference: Diagnostics, Changelog, Open and Plugins

The four commands that report, read or reach outside the repository, split from the [release command
reference](cli.md) because nine commands on one page is past the documentation ceiling and compression
is what deletes facts. Every command supports `--help`. Exit codes and the `--ci` channel are the same
for all of them and documented once, [there](cli.md#exit-codes).

### `cutver doctor`

Validates configuration syntax, verifies declared manifest files, and detects **version drift** across packages: `cutver doctor [--check-changelog] [--json]`.

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| | `--check-changelog` | Verifies that release entries in `CHANGELOG.md` match Git release tags. Headings are read through `[git] tag_prefix`, the same as `changelog latest` and `show`, so the commands agree: with `tag_prefix = "release-"`, `## release-1.2.0` is the entry for that tag. A tag whose entry is really absent is still drift, exit code `2`. |
| | `--json` | Prints the diagnosis as one JSON object on stdout — `ok`, a `checks` entry per check (`ok`/`drift`/`skipped`) and the drift arrays — and renders no dashboard or drift text. The exit code still reports the outcome: `0` clean, `2` drift, `1` a check could not run. See the [machine contract](../internals/machine-contract.md). |
| `-c <PATH>` | `--config <PATH>` | Explicit path to `cutver.toml`. |

---

### `cutver changelog`

Utilities to extract and inspect release notes from `CHANGELOG.md`; `latest` prints the newest entry to stdout and skips `[Unreleased]`:

```bash
cutver changelog latest [-H/--include-header] [-p/--path <PATH>] [--template <PATH>] [--json]
cutver changelog show <VERSION> [-H/--include-header] [-p/--path <PATH>] [--template <PATH>] [--json]
cutver changelog open [TARGET] [--print-url] [--browser <BROWSER>]
```

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| `-H` | `--include-header` | Keeps the release heading the entry was extracted from. |
| `-p <PATH>` | `--path <PATH>` | Reads this changelog file instead of the one `cutver.toml` declares or `CHANGELOG.md`. |
| | `--template <PATH>` | Renders the body through an arbitrary MiniJinja template file. |
| | `--json` | Prints the release context as one JSON object on stdout instead of the rendered notes. |

`changelog latest` and `show` read headings through `[git] tag_prefix`: with `tag_prefix = "release-"`,
`## release-1.2.0` is the entry for `1.2.0`. Exactly one marker is stripped, so a `v`/`V` heading still
matches under another prefix and a doubled marker is not read as a version. `changelog open` reads no
changelog: it builds the release URL from the same prefix.

`--json` builds its context from git and fails closed: an unreadable commit range (not a repository, an
unknown tag, an unusable `git`) goes to stderr with exit `1` instead of printing `"commits": []`, so a
consumer can tell an empty range from an unreadable one. `--template` keeps rendering the body with an
empty commit list. The object opens with `schemaVersion` and then the release context keys, including
`version` — the release version, not the schema one.

---

### `cutver open`

Resolves the repository remote, a release or a tag comparison to a URL and opens it in a browser, or
prints it: `cutver open [TARGET] [--print-url] [--browser <BROWSER>]`. `TARGET` takes a version
(`0.9.1`), a range (`v1.0.0...v1.1.0`) or the literal `compare`, and omitting it targets the repository
remote. `changelog open` is the same surface reached through the changelog command, listed above with
the flags it shares.

| Flag | Long Option | Description |
| :--- | :--- | :--- |
| | `--print-url` | Prints the resolved URL to stdout instead of launching a browser; `--dry-run` is a visible alias, so a script can ask for the target without opening it. |
| | `--browser <BROWSER>` | Launches this browser or command instead of the environment's default. |

---

### `cutver plugin`

Installs, inspects, updates and removes plugins from the registry index. Requires a plugin-enabled build (`--features plugins`): a lean binary hides the subcommand and fails closed naming the flag.

```bash
cutver plugin add <NAME> [--version <VERSION>] [--registry <BASE>] [--event <EVENT>]... [--manifest-match <GLOB>]... [--dry-run] [--force]
cutver plugin list [--json]
cutver plugin restore [--registry <BASE>] [--dry-run]
cutver plugin update [NAME] [--version <VERSION>] [--registry <BASE>] [--dry-run]
cutver plugin info <NAME> [--registry <BASE>] [--json]
cutver plugin search [QUERY] [--registry <BASE>] [--json]
cutver plugin remove <NAME> [--keep-artifact]
```

`plugin` exits `1` on any failure: an unresolvable cache root, an unreachable registry, a digest mismatch, an unknown capability, or a declaration that already exists. The install flow, the digest trust model, the cache layout and the `[plugins.<name>]` keys live in the [plugin reference](plugins.md). Every `plugin` JSON surface opens with `schemaVersion`; `list` and `search` wrap their entries in `{"schemaVersion": 1, "plugins": [...]}`, because a bare array has no room for a version.

---

## Related Documentation

- [Release Command Reference](cli.md): exit codes, the `--ci` channel and the five release commands.
- [Plugin Reference](plugins.md): the install flow, capabilities and the `[plugins.<name>]` keys.
- [Complete cutver.toml Configuration Reference](configuration.md): every configuration key.
- [Machine Contract](../internals/machine-contract.md): `schemaVersion`, every JSON surface, and exit codes.
