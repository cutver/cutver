# Writing a Process Plugin in Another Language

A `runtime = "process"` plugin is an executable that speaks JSON on stdin and stdout, so it can be
written in Python, Go, bash or anything else. This page is the authoring half: the three things that
break first, one complete plugin, and the process it lives in. The runtime's own rules — the exchange,
the envelope, `command` resolution, the baseline, `timeout_seconds` and the failure table — are on
[the process runtime](plugin-process-runtime.md); the fields are in the
[capability reference](plugin-capabilities.md).

## Bytes break first

The exchange has no framing to lean on. `cutver` writes the whole envelope to stdin and closes it, so
the plugin reads until EOF; it reads stdout until EOF and decodes the result as the capability's answer
type. Two habits follow.

- **Read stdin to the end.** There is no length prefix, no delimiter and no trailing newline
  (`src/proc.rs`, `run_capture`), so `json.load(sys.stdin)` and `io.ReadAll(os.Stdin)` are exactly right
  and a `readline()` that stops at the first newline in a commit message is not.
- **Write one JSON value and nothing else.** stdout is decoded as one document, so a second document, a
  progress line or a `print("debug")` makes the answer undecodable and is reported as
  `returned invalid IPC JSON payload`. It is also bounded: past `MAX_PLUGIN_STDOUT_BYTES` (1 MiB,
  `src/plugin/driver/process.rs`) the answer is refused with its own error instead of being read into
  memory, so a stray print costs a message, not the machine. stderr has the opposite policy — past
  `MAX_PLUGIN_STDERR_BYTES` (64 KiB) the host keeps the first 64 KiB, appends a line saying so, and
  the call is not failed for it.

Then the encoding, which is where a working plugin becomes an intermittent one. Python's `print()`
encodes with whatever the interpreter derives from the locale, and the child is given the host's
environment: `LANG` and `LC_ALL` are both in the baseline, and a value the host sets to the empty
string is dropped rather than handed over empty (`src/plugin/driver/baseline.rs:36-37,59-63`, pinned at
`:86-100`). So the child is not locale-less in general — it is locale-less on a host that exports no
locale, which is a typical Windows host, and there the interpreter falls back to a single-byte code
page (ANSI on Windows, ASCII under the C locale). A character outside it then reaches the answer as a
`UnicodeEncodeError`.

**Write the bytes yourself, in UTF-8.** That removes the dependency instead of predicting it:

```python
sys.stdout.buffer.write(json.dumps(answer).encode("utf-8"))
```

Measured, with `PYTHONIOENCODING=cp1252` standing in for a locale that resolves to a single-byte code
page: `sys.stdout.write` of a string holding `✓` raised `UnicodeEncodeError: 'charmap' codec can't
encode character '\u2713'` and exited 3, while the `sys.stdout.buffer` line wrote the same text as
valid UTF-8 and exited 0. The interpreter default moved once already — Python 3.15 turns UTF-8 mode on
unless disabled (PEP 686), which is why reproducing that needed `-X utf8=0` — so name the encoding in
the plugin rather than trust the interpreter.

## stderr survives only when the call fails

The host drains the child's stderr into memory and hands it to `finish_outcome` as `stderr_bytes`, and
the success path never reads that parameter: it is not printed, not logged and not returned
(`src/plugin/driver/process.rs:201-208,247-268`). The bytes reach a human in exactly one place, the
non-zero exit, trimmed into `invocation failed with exit status <n>: <your text>` (`:255-260`).

So a child that writes to stderr and returns `0` writes into the void. The drain still runs, so the
bytes cost part of the `timeout_seconds` budget either way. There is no way to ask for them: `cutver`
has no `--verbose`, no `-v` and no log level — its own diagnostics are `eprintln!` at the call site, and
nothing in `src/**` reads `RUST_LOG`. Two ways out while you debug:

- **Write a file.** The workspace root is the child's working directory, so a path under
  `payload["root_dir"]` lands beside the release notes and needs no grant. Remove it before you ship.
- **Exit non-zero on purpose.** During a debugging session, end the plugin non-zero and your diagnostics
  travel in the error message the user sees.

Nothing in the suite pins the success-path drop; the only stderr assertion is the failure case
(`src/plugin/driver/process/tests.rs:83-98`).

## `command` is looked up on the host's `PATH`, not in the workspace

`command` goes to `Command::new` as one program string, and the host never checks that it exists or is
absolute (`src/plugin/driver/process.rs:114-121`); validation rejects only an empty or blank value
(`src/plugin/manager/factory.rs:93-101`). The lookup therefore uses the **host's** `PATH`, which the
baseline copies into the child's environment verbatim (`src/plugin/driver/baseline.rs:26-28`) — **not** the
workspace root, even though that is the directory the child then runs in. A plugin file sitting next to
the release scripts is therefore not found as `my_plugin.py` unless the host's `PATH` already holds that
directory; the failure arrives as `failed to spawn plugin '<name>' process`. Two ways out:
- **An absolute path.** `command = "/opt/cutver/pre_bump_guard.py"` needs no lookup and resolves the
  same way on both platforms; prefer it in a repository that runs on more than one (#367).
- **A relative path, which is Unix-only advice here.** `command = "./ci/pre_bump_guard.py"` resolves
  against the child's working directory — the workspace root — on Unix, and that is the only platform it
  has been verified on: on Windows `Command` resolves a relative program differently, which is exactly why
  the absolute form is the recommendation for anything that runs on both. The file needs `chmod +x` and a
  shebang either way: the host execs the program directly, never through a shell, so the kernel reads
  `#!/usr/bin/env python3`. On Windows only an executable image can be spawned, so a `.py` there needs
  a launcher (`Command::new` never sees a `.cmd`, `command` carries no arguments).

## One plugin, one capability

`lifecycle.v1` / `on_pre_bump` is the smallest honest example, because its answer is one required
boolean plus an optional reason. This is the whole plugin, saved as `ci/pre_bump_guard.py`.

```python
#!/usr/bin/env python3
"""A cutver process plugin with one capability: lifecycle.v1 / on_pre_bump."""

import json
import sys


def on_pre_bump(payload):
    # Your project's refusal goes here; `payload` carries the versions, the tag and dry_run.
    return {"allow": True}


HANDLERS = {"on_pre_bump": on_pre_bump}


def main():
    request = json.loads(sys.stdin.buffer.read())  # no framing: read to EOF
    handler = HANDLERS[request["operation"]]
    answer = handler(request["payload"])
    sys.stdout.buffer.write(json.dumps(answer).encode("utf-8"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
```

```toml
[plugins.pre-bump-guard]
runtime = "process"
command = "./ci/pre_bump_guard.py"
capabilities = ["lifecycle.v1"]
events = ["on_pre_bump"]
```

Dispatching on `request["operation"]` is not decoration: one process is spawned per call, so the
envelope is the only thing that says which slot is being answered. A `"protocol": "1"` key may sit
beside the answer fields if you want to state the version you speak; absent means `"1"`. What
`on_pre_bump` needs is `allow` alone; `reason` is quoted when `allow` is false.

## One process per call, thirty seconds by default

Every invocation spawns the executable; nothing stays resident between calls. `timeout_seconds` defaults
to 30 and covers the whole exchange — the stdin write, the run and the drain of the answer — and on
expiry the deadline kills the process tree. Two consequences for a language that pays for its imports:

- **A module-scope import is paid on every call.** `import pandas` at the top of the file costs its
  seconds each time the host asks a question, against that one budget. Import what the answer needs and
  load the heavy part on the branch that needs it.
- **The call count is not one.** A `manifest.v1` plugin is invoked twice per file it is offered, a
  `read` before the `write` (`src/manifest/plugin.rs:29-56`).

## What this page does not cover

- **The runtime's own rules.** The numbered exchange, `command` resolution platform by platform, the
  full environment baseline, what `timeout_seconds` does and does not kill, every failure message, and
  why a process plugin has no sandbox at all: [the process runtime](plugin-process-runtime.md).
- **The fields.** Every request and answer, field by field, with the one optionality rule that decides
  them: the [capability reference](plugin-capabilities.md).
- **The declaration.** Every `[plugins.<name>]` key, the digest gates and the trust model: the
  [plugin reference](../reference/plugins.md).
