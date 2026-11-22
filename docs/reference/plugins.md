# Installing and Managing Plugins

`cutver` can delegate changelog rendering, version calculation, manifest edits and release lifecycle
events to sandboxed WebAssembly plugins. `cutver plugin` installs one from the registry index, verifies
it, and writes the configuration entry that makes `cutver` load it.

> **Gotcha**: plugins are an opt-in build. The default `cutver` binary and `:latest` image are lean:
> no `plugin` command and no WebAssembly runtime. Use a published `cutver-plugins-*` archive or the
> `:plugins` image; the fallback for older versions is `--features plugins`.

---

## Quick Path

1. Get a plugin-enabled binary from a `cutver-plugins-*` archive or the `:plugins` image (older
   versions: `cargo install cutver --features plugins`).
2. Install the official `github-releases` plugin:
   ```bash
   cutver plugin add github-releases
   ```
   This resolves the registry entry, downloads the artifact, verifies its SHA-256, stores it in the
   cache, and appends the `[plugins.github-releases]` table to `cutver.toml`.
3. Confirm what landed, without loading anything:
   ```bash
   cutver plugin list
   ```
4. Preview the next install without writing or downloading the artifact (the registry entry is still resolved):
   ```bash
   cutver plugin add github-releases --dry-run
   ```

Writing your own plugin — scaffolding the crate, exporting `invoke`, building for `wasm32-wasip1`
and publishing the digest a registry entry needs — is covered in
[Writing a Plugin](../guides/plugin-authoring.md).

## Building with Plugin Support

Plugin support ships as its own artifact, so most users never build from source. Releases carry
`cutver-plugins-<version>-<slug>` archives beside the lean ones, and the `:plugins` image
(`ghcr.io/cutver/cutver:<version>-plugins`, also `:<major>.<minor>-plugins`) is the container equivalent.

| Build | Command | Result |
| :--- | :--- | :--- |
| Lean (default) | `cargo install cutver` | No `plugin` command and no plugin runtime; materially smaller than the plugin build. |
| Plugin-enabled archive | `cutver-plugins-<version>-<slug>` or the `:plugins` image | `plugin` command and the WebAssembly runtime; larger, because that runtime is linked in. |
| Plugin-enabled, Homebrew | `brew install Row0902/tap/cutver-plugins` | Same as above; installs the `cutver` binary. |
| Plugin-enabled, Scoop | `scoop bucket add row https://github.com/Row0902/scoop-bucket && scoop install cutver-plugins` | Same as above, on Windows. |
| Plugin-enabled, pre-artifact versions | `cargo install cutver --features plugins` | Same build, compiled locally. |

Both failure modes name the remedy, so a lean binary is never a dead end: `cutver plugin` fails closed
with `--features plugins`, and loading a `[plugins.<name>]` entry at release time fails with the same
instruction instead of a `runtime` setting that cannot help.

Sizes are per-target and change every release, so none is quoted here: the release workflow writes the
bytes and MiB of every archive it uploads into that run's job summary, and the archives are on the
[releases page](https://github.com/cutver/cutver/releases).

## Commands

| Command | What it does |
| :--- | :--- |
| `cutver plugin add <NAME>` | Resolves, downloads, verifies, installs and configures a plugin. |
| `cutver plugin restore [--dry-run]` | Fetches missing declared plugin artifacts into cache without modifying config. |
| `cutver plugin update [NAME]` | Upgrades plugins to the latest compatible release, preserving sub-tables. |
| `cutver plugin search [QUERY]` | Searches the registry index by name, description, capability, or author. |
| `cutver plugin info <NAME>` | Shows metadata, repository, tier, capabilities, and release history. |
| `cutver plugin list [--json]` | Reports declared versus installed versus digest-matched state. Mutates nothing. |
| `cutver plugin remove <NAME>` | Removes the declaration, and its cached artifact when nothing else uses it. |

### `cutver plugin add`

```text
cutver plugin add <NAME> [--version <VERSION>] [--registry <BASE>] \
  [--event <EVENT>]... [--manifest-match <GLOB>]... [--dry-run] [--force]
```

| Flag | Description |
| :--- | :--- |
| `--version <VERSION>` | Version requirement, e.g. `0.1.0` or `^0.1`. Without it, the highest **stable** release wins; a pre-release is selected only when the requirement names one. |
| `--registry <BASE>` | Registry index base. `https` only, default `https://raw.githubusercontent.com/cutver/registry/main/plugins/`. |
| `--event <EVENT>` | Lifecycle event the plugin handles. Repeatable. Required when the entry declares `lifecycle.v1`. |
| `--manifest-match <GLOB>` | Manifest glob the plugin matches. Repeatable. Required when the entry declares `manifest.v1`. |
| `--dry-run` | Resolves the registry entry and prints version, URL, digest and the config diff. Downloads no artifact, writes nothing. |
| `--force` | Replaces an existing `[plugins.<name>]` declaration instead of failing closed. |

`--event` and `--manifest-match` are not cosmetic. An empty `events` list matches **every** lifecycle
event, and an empty `manifest_match` matches **nothing**, so `add` refuses to write either one
implicitly: it fails closed naming the missing key and the capability that required it.

### `cutver plugin list`

Prints each configured plugin with its runtime, `source`, declared `hash`, whether the artifact is
actually present, and whether the on-disk bytes still match the declaration — `matched`, `mismatched`,
or `unknown`. It reports effective state, so a `source` pointing at a missing file reads as not
installed. `--json` emits the same data for scripts.

### `cutver plugin remove`

Removes the `[plugins.<name>]` table with a surgical edit: comments, key order and unrelated tables are
untouched. The cached artifact is deleted only when no other declaration shares the same `source`;
`--keep-artifact` keeps it regardless. A `source` that is your own local file is never deleted.

## What `add` Writes

```toml
[plugins.github-releases]
runtime = "wasm"
source = "github-releases-0.1.0.wasm"
hash = "a828cbabd247afdaf78fec358ae5a27271097c7c2ef6fc9ec0fe04af22edf841"
capabilities = ["changelog.v1"]
```

| Key | Meaning |
| :--- | :--- |
| `runtime` | Always `wasm` for an installed plugin: the registry only publishes `.wasm` artifacts. |
| `source` | Artifact file name inside the cache, `<name>-<version>.wasm` as `add` writes it. A hand-written entry may point at a local file instead, which is read in place and never deleted. |
| `hash` | The registry's SHA-256, re-verified on every load **when present**. `add` always writes it; an entry written by hand without it gives up the load-time check. |
| `capabilities` | What the plugin may do. Validated against the canonical set; an unknown value fails closed. |
| `events` | Lifecycle events this plugin handles. Empty means **all** of them. |
| `manifest_match` | Manifest globs this plugin is offered. Empty means **none**. |
| `permissions` | `network`, `env` and `filesystem` grants. For `wasm`, denied unless granted ([Runtime Sandbox](#runtime-sandbox)); for `process`, only `env` applies and nothing is sandboxed. |
| `timeout_seconds` | `integer`, seconds. `30` when unset for `runtime = "process"` (`src/plugin/manager/factory.rs:10,67`); passed to the WASM sandbox when set, and with no value the sandbox gets no timeout configured (`src/plugin/driver/wasm.rs:139-143`). It bounds one call, not the whole release. |
| `command` | Executable for a `runtime = "process"` plugin; required and non-empty there, and always run with an empty argument list. Unused by `wasm`. See [the process runtime](../guides/plugin-process-runtime.md). |
| `config` | A free-form TOML table ([details](configuration.md)). `cutver` parses it, preserves it across `plugin update`, and reads nothing out of it: the keys are the plugin's own, and the host fills no channel from them. `github-publish` is the worked case ([the reference plugin](../guides/plugin-github-publish.md); `cutver.toml:84-94`). |

> **A `changelog.v1` plugin only runs when `[changelog] format = "plugin"`.** With the default
> `keep-a-changelog` the built-in renderer writes the notes and the installed plugin stays idle —
> `plugin list` still reports it installed and digest-matched. Name the plugin explicitly with
> `[changelog] plugin = "<name>"` when more than one declares `changelog.v1`.

## Trust Model: Two Gates, One Digest

1. **Install time.** `add` verifies the downloaded bytes against the `sha256` in the registry entry
   *before* the artifact reaches its final cache name. A mismatch leaves no cache file and no config
   change.
2. **Load time.** The loader re-verifies the cached file against the `hash` in `cutver.toml` on every
   load, so a tampered cache is caught even though the digest was already checked once. This gate is
   automatic when `hash` is present — which is what `add` always writes — and absent when an entry
   declares no `hash`.

No flag skips the install-time gate: there is no `--insecure` and no "continue anyway", and the digest
is checked before the artifact enters the cache. The registry index itself is `https`-only on purpose,
because the index is what distributes the digest — TLS there is trust, not just transport.

## Runtime Sandbox

**A plugin has zero authority by default.** This is the canonical statement of that rule; other pages
link here instead of restating it.

- A WASM plugin gets no filesystem, no network and no environment access unless the entry grants it
  under `[plugins.<name>.permissions]`, which accepts `network`, `env` and `filesystem`
  (`src/plugin/driver/wasm.rs:118-137`). Only the granted hosts, directories and keys are added; a
  `filesystem` entry that resolves outside the workspace is refused at load
  (`src/config/discovery.rs:59-75`).
- A plugin is also denied every capability it did not declare: declaring is what makes a call possible,
  and there is no flag that widens it (`src/plugin/manager/core.rs:249-262`).
- There is no `--insecure` bypass for the digest gates either ([above](#trust-model-two-gates-one-digest)).
- **The process runtime is not sandboxed.** `permissions.network` and `permissions.filesystem` have no
  effect on it; only `permissions.env` is applied, by clearing the child's environment. `cutver doctor`
  still prints one row per `filesystem` entry, because a row reports the declaration, and marks that row
  inert there (`"effect": "ignored"` under `doctor --json`). See
  [the process runtime](../guides/plugin-process-runtime.md) and, for the filesystem mount names a
  plugin author works with, the [authoring guide](../guides/plugin-authoring.md).

## Cache Location

| Source | Cache root |
| :--- | :--- |
| `CUTVER_PLUGIN_CACHE_DIR` | Used **verbatim**, as-is. |
| `$HOME` (Linux, macOS) | `$HOME/.cache/cutver/plugins` |
| `%USERPROFILE%` (Windows) | `%USERPROFILE%\.cache\cutver\plugins` |

With none of them set, `cutver` fails closed naming the variable to set instead of guessing. A
`source` that is an existing local file always wins over the cache, which is how you develop a plugin
you are working on.

## Troubleshooting

| Symptom | Cause and remedy |
| :--- | :--- |
| `cutver plugin` reports the `plugins` feature is missing | Lean binary. Install or build with `--features plugins`. |
| `unresolvable plugin cache root` | Neither the override nor a home directory is set. Set `CUTVER_PLUGIN_CACHE_DIR`. |
| `integrity check` on install | The artifact does not match the registry digest. Retry; if it persists, the release or the index is wrong. |
| `plugin list` shows `mismatched` | The cached file changed after installation. Reinstall with `cutver plugin add <NAME> --force`. |
| `already declared under [plugins.<name>]` | The entry exists. Use `--force`, or remove it first. |
| `requires at least one --event` / `--manifest-match` | The entry declares `lifecycle.v1` or `manifest.v1`; pass the flag. |

## Related Documentation

- [Plugin Overview](../guides/plugin-overview.md): what a plugin is, the lifecycle slots, and one
  worked request and answer.
- [The Reference Plugin](../guides/plugin-github-publish.md): the `github-publish` declaration this
  repository releases with, its draft-then-promote flow, and how that release fails.
- [The Process Runtime](../guides/plugin-process-runtime.md): `command`, stdin/stdout, env grants and
  the timeout, for the runtime with no sandbox.
- [Plugin Architecture and Wire Contract](../rfcs/0001-wasm-plugin-architecture.md): how the WebAssembly
  runtime, capabilities and the host boundary fit together.
- [cutver.toml Configuration Reference](configuration.md): the rest of the configuration tables.
- [CLI Commands and Exit Codes](cli.md): global flags and exit codes.
