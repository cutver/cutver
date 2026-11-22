# The Process Runtime

`runtime = "process"` is the second driver behind the plugin contract: it runs an executable once per
invocation, hands it the same JSON envelope a WASM plugin receives, and reads one JSON answer back, so
a release can delegate to a script in any language without a `wasm32-wasip1` build step.

**Read this first: a process plugin has no sandbox.** See the next section before you configure one.

## No sandbox

A process plugin is an ordinary child process. It runs as the same user, in the same working
directory, with the same file permissions and the same network reach as `cutver` itself
(`src/plugin/driver/process.rs` sets no directory and no isolation). Nothing in the plugin
entry restricts where it reads, writes or connects:

- `permissions.network` and `permissions.filesystem` have **no effect** here: the driver is built from
  `command`, `timeout_seconds`, `capabilities` and `permissions.env` only
  (`src/plugin/manager/factory.rs:57-84`).
- `source` and `hash` are unused, so there is no digest gate either (same function).
- `permissions.env` narrows the child's environment, and that is the only containment there is.

Containment is a WASM-only property of the engine
([ADR-0001](../adr/0001-dual-runtime-plugin-microkernel.md), *Consequences*). Use this runtime only
for code you already trust as much as the release script — your own script, a binary you already run.
The [overview](plugin-overview.md#wasm-versus-the-process-runtime-in-one-table) compares the two.

## Declaring one

```toml
[plugins.helm-adapter]
runtime = "process"
command = "./ci/helm-version-plugin"
capabilities = ["manifest.v1", "lifecycle.v1"]
manifest_match = ["Chart.yaml"]
events = ["on_pre_bump"]
timeout_seconds = 20

[plugins.helm-adapter.permissions]
env = ["HELM_CACHE"]
```

`command` is required and must not be empty or whitespace; without it the load fails with
`'command' is required for process runtime and cannot be empty` (`src/plugin/manager/factory.rs:58-66`).
`capabilities` is enforced on every call: invoking a capability the entry did not declare is
`UnsupportedCapability`, and no call reaches your code (`src/plugin/driver/process.rs`, `invoke`).

## How `command` is resolved

- **One string, no arguments.** The whole of `command` is the program, and the driver passes an empty
  argument list (`src/plugin/manager/factory.rs:75-77`, used at `src/plugin/driver/process.rs`,
  `child_command`). A wrapper script is how you get arguments; there is no `args` key.
- **No shell.** Unlike `[preflight]` steps, `[hooks]` and `[publish] commands`, which go to `sh -c` or
  `cmd /C` (`src/preflight.rs`, `run_supervised`), a process plugin is spawned directly. Shell syntax,
  globs and `&&` are not interpreted; `command = "./script.sh && echo done"` fails to spawn.
- **Resolution is the operating system's.** A bare name is looked up like any `exec` of that name; a
  relative path is taken from `cutver`'s working directory. Prefer an absolute path.
- **The working directory is `cutver`'s**, not the workspace root, and the child inherits it. Resolve
  your own paths, or join `payload.root_dir`, which is the workspace root forward-slashed on every
  platform (`crates/cutver-pdk/src/lifecycle.rs:6-19`). This is the one stage that still inherits it:
  `[preflight]` steps, `[hooks]` and `[publish] commands` run in the workspace root instead (#295).
- **On Windows, name an executable.** Since no arguments cross the boundary, a `.cmd` wrapper cannot be
  handed to `cmd /C`; build an `.exe`.

## The per-call exchange

One invocation is one process, start to finish:

1. `cutver` spawns `command` with piped stdin, stdout and stderr.
2. It serializes the envelope to JSON and writes all of it to stdin, then closes it
   (`src/proc.rs`, `run_capture`). Read until EOF: there is no framing and no
   trailing newline.
3. The plugin writes exactly one JSON value to stdout and exits 0.
4. `cutver` reads stdout to EOF and decodes it as the capability's response DTO
   (`src/plugin/manager/core.rs:209-235`). Trailing text after the JSON value makes the answer
   undecodable, so log to stderr, never to stdout.
5. The slots whose answer is discarded (`on_post_bump`, `on_post_release`) still need one JSON value,
   `{}` being enough: the host decodes what arrives before dropping it
   (`src/plugin/manager/core.rs:179-188`).

A minimal `lifecycle.v1` plugin:

```bash
#!/bin/sh
# One envelope in on stdin, one JSON value out on stdout, diagnostics on stderr.
request=$(cat)
echo "pre-bump for $request" >&2
printf '{"allow": true}'
```

The exit status outranks stdout: a non-zero exit is `ExecutionFailed` carrying the trimmed stderr, even
when stdout held a valid answer (`src/plugin/driver/process.rs`, `finish_outcome`). A zero exit with an empty or
malformed stdout is `InvalidPayload` instead.

A plugin that answers and exits `0` **before** reading its envelope to EOF loses its own answer. The
host's write then lands on a closed pipe, and the run fails with `failed to write the invocation
envelope ... to stdin` even though a valid response was produced (`src/proc.rs`, `recv_written`).
Read stdin to EOF first and answer afterwards, the way the `cat` in the example above does. Skipping it
does not only risk the error: it makes the outcome depend on which side finishes first, which is a race
in the plugin, not in `cutver`.

## `permissions.env`

Environment is the only grant this runtime applies, and it applies it in both directions.

| Rule | Effect |
| :--- | :--- |
| The child starts with a **cleared** environment (`src/plugin/driver/process.rs`, `child_command`). | Nothing is inherited: `PATH`, `HOME`, `TMPDIR`, CI tokens and every other variable of the `cutver` process are absent unless granted. |
| Exactly the granted keys are set. | `env = ["GITHUB_TOKEN"]` gives the child `GITHUB_TOKEN` and nothing else. |
| A declared key that is **unset** is an error, not an empty string. | Loading fails with `plugin '<name>' declares the environment grant '<key>', but cutver cannot read '<key>' from its own environment` (`src/plugin/error.rs:127`). The value must also be valid UTF-8. |
| Grants are resolved once, when the plugin manager is built. | The failure lands at the start of a run, before any plugin is called (`src/plugin/manager/factory.rs:71`), through the function the WASM runtime shares (`src/plugin/driver.rs:20-38`). |

A script whose interpreter is resolved through `PATH` (`#!/usr/bin/env python3`) should declare `PATH`
in `env`, or name the interpreter by absolute path in the shebang.

## `timeout_seconds`

| | |
| :--- | :--- |
| Type | `integer`, seconds (`src/plugin/types.rs:184`) |
| Default | `30` when unset (`src/plugin/manager/factory.rs:10,67`) |
| Bounds | The whole exchange is one budget: the stdin write, the wait for exit, and draining the answer (`src/proc.rs`, `run_capture`) |
| On expiry | `plugin '<name>' timed out after <n>s during capability '<cap>'` (`src/plugin/error.rs:55`) |
| Reaches | The whole process tree: the plugin and every process it started |

That last row is the one that matters as soon as your plugin starts helpers. The plugin is spawned in
a process group of its own and the deadline signals that group — `killpg` on unix, `taskkill /F /T`
on Windows — so a descendant that inherited the pipes is killed with the plugin and cannot extend
the invocation past the limit (`src/proc.rs`, `spawn_group` and `kill_tree`). Your helpers are
reachable by that kill: nothing of your own is left running after a timeout.

One case is deliberately not a kill: a plugin that exits `0` while a descendant it started still
holds the pipes. A successful plugin gets no new kill semantics, but the exchange still ends at the
budget — draining its answer is bounded by what is left of `timeout_seconds`, so you get the timeout
error above instead of a caller waiting on a process it did not start.

`[preflight]` steps, `[hooks]` and `[publish] commands` reach the same supervision through
`run_supervised` (`src/preflight.rs`), so a timed-out step, hook or command is also killed with its
whole tree.

## How it fails

| Symptom | Cause |
| :--- | :--- |
| `failed to spawn plugin '<name>' process: ...` | `command` does not exist, is not executable, or a shell expression was used instead of a program (`src/plugin/error.rs:43`). |
| `invocation failed with exit status <n>: <stderr>` | The plugin exited non-zero; the message carries its stderr (`src/plugin/error.rs:66`). |
| `returned invalid IPC JSON payload` | stdout was empty, not one JSON value, or not the DTO the capability expects (`src/plugin/error.rs:90`). |
| `timed out after <n>s` | The exchange outlived `timeout_seconds`; the plugin's process tree was killed. |
| `failed to write the invocation envelope ... to stdin` | The plugin, or something it started, stopped reading stdin before the envelope ended (`src/plugin/error.rs:78`). |
| `declares the environment grant '<key>', but ...` | A granted key is unset in the environment running `cutver`. |

## What this page does not cover yet

- **The WASM runtime.** Sandbox grants, mount names and the `invoke` export:
  [Writing a Plugin](plugin-authoring.md) and the [overview](plugin-overview.md).
- **The other keys.** `source`, `hash`, `manifest_match` and the install commands: the
  [plugin reference](../reference/plugins.md).
- **Why two runtimes exist.** [RFC 0001](../rfcs/0001-wasm-plugin-architecture.md) and
  [ADR-0001](../adr/0001-dual-runtime-plugin-microkernel.md).
- **Nothing validates `command` until it is called.** A missing or non-executable file is reported as
  `SpawnFailed` at the first invocation, not when the configuration loads.
- **The argument list and the working directory cannot be configured** in `cutver.toml`: a plugin that
  needs either stays behind a wrapper script, since neither key exists yet.
