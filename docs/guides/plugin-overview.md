# Plugin Overview: What a Plugin Is

A **plugin** is code `cutver` did not compile, called at one defined point of a release to take over
that stage or to observe it. `cutver` ships its own implementation of every stage, so no plugin is
needed to cut a release: a plugin replaces one stage with your own logic instead of a patched binary.

This page starts from zero: no plugin written, none configured.

## Quick path: you only want to install one

1. Use a plugin-enabled build: a `cutver-plugins-*` archive, the `:plugins` image, or
   `cargo install cutver --features plugins`. A lean build hides `cutver plugin` and fails closed
   with that instruction (`src/cli/runner/plugin.rs:50-67`).
2. `cutver plugin add github-releases` resolves the registry entry, downloads the artifact, verifies
   its SHA-256, caches it, and appends the `[plugins.github-releases]` table to `cutver.toml`.
3. `cutver plugin list` prints declared, installed and digest-matched state without loading anything.

Every flag, cache path and troubleshooting row is in
[Installing and Managing Plugins](../reference/plugins.md); read on to understand what you installed,
or to write one.

## Where a plugin can plug in

Four **capabilities** — a closed set. A plugin declares the ones it serves, and `cutver` refuses to
invoke a capability the entry did not declare, before any call reaches the plugin
(`src/plugin/manager/core.rs:209-235`).

| Capability | Operations on the wire | What the plugin decides |
| :--- | :--- | :--- |
| `manifest.v1` | `read`, `write` | How one manifest format is parsed and rewritten. |
| `changelog.v1` | `render` | The body of the release notes. |
| `versioning.v1` | `compute` | The next version, from the commits. |
| `lifecycle.v1` | `on_pre_bump`, `on_post_bump`, `on_post_release` | Nothing about content: a slot that runs your code at a point in the release. |

A `changelog.v1` plugin is only consulted when `[changelog] format = "plugin"`, a `versioning.v1` one
only when `[version] strategy = "plugin"`, and a `manifest.v1` one only for a manifest whose
`kind = "plugin"`. Otherwise they load and stay idle.

## The three lifecycle slots, and when each runs

| Slot | Runs | What it can change |
| :--- | :--- | :--- |
| `on_pre_bump` | After preflight steps, before any manifest or changelog byte is written (`src/bump/exec/pipeline.rs:95-98`; writes start at `:134`). | The release: answer `allow = false` and `cutver` aborts, quoting your `reason`. |
| `on_post_bump` | After the new files are written, before `git stage` (`src/bump/exec/pipeline.rs:134-143`). | Files only. An edit to a path already in the staging list travels with the release commit; a new file is not staged and not committed. |
| `on_post_release` | Last step: after the tag exists and after `[publish] commands` (`src/bump/exec/publish.rs:265-285`). | Nothing — the commit and tag are already made. It creates things outside the repository, such as a draft release. |

Three details decide what runs and when:

- **`events` picks the slots.** Empty means all three; a non-empty list means exactly those
  (`src/plugin/manager/core.rs:63-73`).
- **`bump --defer-publish` skips `on_post_release`.** The slot belongs to the closing
  `cutver publish`, together with the tag pushes, once the built artifacts have been checked
  (`src/bump/exec/publish.rs:265-273`).
- **A dry run still dispatches the slots**, with `dry_run = true` in the payload, so a plugin can
  rehearse.

## The invocation envelope, with one worked example

Both runtimes carry the same three-field JSON envelope (`crates/cutver-pdk/src/invocation.rs:5-13`). A
WASM plugin receives it through its one export, `invoke`; a process plugin on stdin. This is a
`changelog.v1` request, `cutver` on the left, plugin on the right:

```json
{
  "capability": "changelog.v1",
  "operation": "render",
  "payload": {
    "root_dir": "/home/me/project",
    "version": "1.1.0",
    "tag_name": "v1.1.0",
    "previous_tag": "v1.0.0",
    "release_date": "2026-11-13",
    "commits": [
      { "sha": "4f2c1ab", "message": "feat: add helm adapter", "type": "feat", "is_breaking": false }
    ],
    "is_prerelease": false,
    "contributors": []
  }
}
```

The answer is the capability's response DTO, and nothing else:

```json
{ "body": "### Features\n\n- add helm adapter\n" }
```

`payload` and the answer are typed per capability in `cutver-pdk`
(`crates/cutver-pdk/src/changelog.rs:4-75`, `crates/cutver-pdk/src/lifecycle.rs:4-57`). A `manifest.v1`
answer returns the file content, `versioning.v1` the next SemVer string, and the two `on_post_*` slots
answer with any JSON value, which `cutver` discards. Anything that does not decode as the expected DTO
is an `InvalidPayload` error naming the plugin and the capability.

## WASM versus the process runtime, in one table

| | `runtime = "wasm"` | `runtime = "process"` |
| :--- | :--- | :--- |
| What runs | A `.wasm` module exporting `invoke`, inside the Extism sandbox (`src/plugin/driver/wasm.rs:16,159-163`). | An executable, spawned once per invocation (`src/plugin/driver/process.rs:74-90`). |
| Declared with | `source`, plus an optional `hash`. | `command`, required and non-empty (`src/plugin/manager/factory.rs:57-70`). `source` and `hash` are unused. |
| Host access | Only what the entry grants: hosts, directories, environment keys (`src/plugin/driver/wasm.rs:118-137`). | Everything the account running `cutver` can reach. **No sandbox.** |
| Bound on the call | `timeout_seconds` when set; with no value the sandbox gets no timeout configured (`src/plugin/driver/wasm.rs:139-143`). | `timeout_seconds`, default 30 (`src/plugin/manager/factory.rs:10,67`). |
| Extra gate | A declared `hash` is re-verified on every load (`src/plugin/driver/wasm.rs:93-96`). | None: the file on disk is executed as it is. |
| When to choose it | Default. Untrusted or third-party code. | Only code you already trust — see [the process runtime](plugin-process-runtime.md). |

## What a plugin cannot do

- **Grant itself anything.** Permissions are read from `cutver.toml` and nowhere else; the envelope
  has no permission channel (`crates/cutver-pdk/src/invocation.rs:5-13`).
- **Be called outside a capability it declared.** Refused before dispatch, with no bypass flag
  (`src/plugin/manager/core.rs:249-262`).
- **A WASM plugin, reach anything ungranted.** No directory, host or variable enters the sandbox
  unless the entry names it; nothing skips that mapping (`src/plugin/driver/wasm.rs:118-137`).
- **A WASM plugin, leave the workspace through a grant.** A `filesystem` entry that resolves outside
  the workspace root is refused at load time, before anything is mounted
  (`src/config/discovery.rs:59-75`).
- **A WASM plugin, load in a lean build.** It fails with `WasmNotSupported`. A `runtime = "process"`
  entry does load in a lean build, because only the WASM driver is feature-gated
  (`src/plugin.rs:1-14`, `src/plugin/manager/factory.rs:20-30`).
- **Be installed without the install-time digest check.** `cutver plugin add` verifies the downloaded
  bytes against the registry before they reach the cache. The *load*-time check is separate and runs
  only when the entry declares `hash`.

## How a plugin fails

| What happened | Error | What the release does |
| :--- | :--- | :--- |
| `on_pre_bump` answered `allow = false` | `plugin '<name>' rejected pre-bump checks: <reason>` (`src/bump/types.rs:209-215`) | Aborts before anything is written. |
| A call in `on_post_bump` failed | the plugin's own error | Rolls back the files `cutver` wrote in this run (`src/bump/exec/pipeline.rs:138-141`). |
| A call in `on_post_release` failed | `<error>`, naming the tag and commit (`src/bump/types.rs:173-181`) | Leaves the commit and tag in place; the message says what to inspect. |
| The call outlived `timeout_seconds` | `Timeout`, naming plugin, capability and limit (`src/plugin/error.rs:55`) | Fails the slot it was dispatched from. |
| The answer did not decode | `InvalidPayload` (`src/plugin/error.rs:90`) | Same as above. |
| A declared `permissions.env` key was unset | `UnsetEnvGrant` (`src/plugin/error.rs:127`) | Fails at load, before any plugin is called. |

The six runtime rows carry `Where:` and `Fix:` lines naming where the failure happened and what to do
(`src/plugin/error.rs:39-127`); an install or download failure reports the underlying transport error.

## What this page does not cover yet

- **The key-by-key contract.** Every key a `[plugins.<name>]` table accepts, the two digest gates and
  the cache location: the [plugin reference](../reference/plugins.md).
- **Writing and publishing a WASM plugin.** The crate scaffold, the `invoke` export,
  `wasm32-wasip1`, the registry entry: [Writing a Plugin](plugin-authoring.md).
- **The process runtime in detail.** `command` resolution, the stdin/stdout exchange, env grants and
  what `timeout_seconds` does and does not kill: [The Process Runtime](plugin-process-runtime.md).
- **The `[plugins.<name>.config]` table**, which the host parses and never reads: it is a reserved
  table for the plugin itself, described in the [configuration reference](../reference/configuration.md).
- **The reference plugin, end to end.** The declaration this repository releases with, the
  draft-then-promote workflow and its failure modes: [the reference plugin](plugin-github-publish.md).
- **Why the engine looks like this:** [RFC 0001](../rfcs/0001-wasm-plugin-architecture.md).
- **Not documented anywhere yet:** a plugin entry's `command` is only checked for being non-empty at
  load time, so a missing executable is reported at the first call; and no grant on the process
  runtime is a containment guarantee, because that runtime has no sandbox at all.
