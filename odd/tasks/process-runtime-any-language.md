# process-runtime-any-language — make the process runtime usable by an author who does not write Rust

The `runtime = "process"` driver exists so a plugin can be written in Python, Go, bash or anything
that does not compile to WASM. Today its transport is sound and its *authoring* surface is not: three
of the four capabilities are documented only as Rust types, the child inherits no system environment,
and nothing pins the protocol a non-Rust plugin must speak. This is the plan to close that gap. No
architecture change: the supervisor, the timeout, the tree kill and the typed errors stay.

## Specs

S1-S9 are the plan this document proposes; S10 quotes the maintainer's requirement it answers.

- **S1 — A documented system baseline reaches the child.** A `runtime = "process"` child is spawned with
  `env_clear()` plus the granted keys, so it receives no `PATH`, no `HOME`/`USERPROFILE`, no
  `TMPDIR`/`TEMP`, no `LANG`/`LC_ALL`, and on Windows no `SystemRoot` — the last one stops a managed
  engine from starting at all. The driver must supply a named, documented baseline of *system* keys
  (never secrets, never `*_TOKEN`/`*_KEY` shapes) alongside the declared grants, and the reference page
  must list them by name. `permissions.env` keeps meaning "any other key this plugin needs".
  RED first: a fixture plugin that prints `$PATH` and `$HOME` shows them empty on `82d1843`.
- **S2 — The child runs in the workspace root, never the caller's directory.** `child_command`
  (`src/plugin/driver/process.rs:84-89`) sets no `current_dir`, against `CONTRACT.md:154-155`. A plugin
  that imports a sibling module or opens a relative template resolves it against wherever the user ran
  `cutver`. The child's working directory is the resolved `root_dir`, and a behavioural test spawns a
  process plugin from an unrelated directory and asserts the child's cwd.
- **S3 — The envelope carries a protocol version.** `PluginInvocation` has three fields
  (`crates/cutver-pdk/src/invocation.rs:9-13`) and none states the version it speaks. A plugin built
  against an older shape fails today with `InvalidPayload`, indistinguishable from a malformed line.
  The invocation carries `protocol` (a version string), and a mismatch fails with a typed error naming
  the host's version and the plugin's, with `Where:` and `Fix:`.
- **S4 — All four capabilities are specified outside Rust.** `changelog.v1` is documented end to end
  (`docs/guides/plugin-overview.md:61-91`); `manifest.v1`, `versioning.v1` and the three lifecycle
  payloads are not, and their request field lists exist only in `crates/cutver-pdk/src/*.rs`. Each
  capability gets its request/response pair in the docs, field by field, with the optionality rules
  that `tests/plugin_wire_contract.rs:110-124` already proves: `Option` fields are omitted and never
  `null`, while `commits`, `is_prerelease` and `contributors` are always present.
- **S5 — The framing is pinned by test, per capability.** No test asserts that the request has no
  trailing newline or that stdin is closed; every fixture uses `cat` or `read -r line`, both
  framing-agnostic. A golden per capability pins the exact bytes written to stdin and the sha256 of
  the request, so appending `\n`, length-prefixing or closing early fails a test instead of a user's
  Python plugin.
- **S6 — stdout is bounded.** `src/proc.rs:255-267` reads the child's stdout with `read_to_end` into an
  unbounded `Vec`. A plugin that logs to stdout by mistake exhausts the host's memory. The read is
  capped at a named limit and overrunning it is a typed error naming the limit and the remedy.
- **S7 — stderr has a stated fate.** It is captured and used only when the exit status is non-zero
  (`src/plugin/driver/process.rs:154-170`); on success it is discarded, undocumented. Choose one and
  write it down: forward it when a verbose flag is set, or document that it is dropped unless the
  process fails. A plugin author debugging a working call needs the answer in the reference page
  either way.
- **S8 — Two failures are distinguishable.** Invalid UTF-8 on stdout and a well-formed JSON document
  that does not match the schema both surface as `InvalidPayload` (`src/plugin/error.rs:86-95`). The
  first gets its own message naming the byte offset or the decode error, so the author is not sent
  hunting a schema bug that does not exist.
- **S9 — The authoring guide says how to write bytes.** The child has no locale, so a Python author
  must write UTF-8 through `sys.stdout.buffer` rather than `print()`; the failure on a non-ASCII
  commit message is otherwise a `UnicodeEncodeError` inside their plugin on Windows only. Stated in
  `docs/guides/plugin-process-runtime.md` beside the "log to stderr" rule.
- **S10 — the maintainer's requirement, verbatim**: «Realmentee no quiero borrar las APIs de plugins
  process, pero me gustaría hacerla lo más robusta posible, porque si un desarrollador de python
  quiere crear un plugin o cualquier otro lenguaje que no sea de los soportados, por eso lo incluí».

**Explicitly out of scope.** A resident/keep-alive mode. One child is spawned per call
(`src/plugin/driver/process.rs:114-118`), and `manifest` costs two spawns per file
(`src/manifest/plugin.rs:36,52`), so a Python plugin importing a heavy library pays seconds per call
under a 30s per-call timeout. That is the real ceiling of this design for heavy plugins, it is not a
defect, and it is a separate feature with its own protocol (correlation ids, a health check, a
restart policy).

## Tasks

| ID | Spec | Route | Commit |
| --- | --- | --- | --- |
| T1 | S2 | **Done in #367** — merged as `c3b7390` (commit `fda707c`). `child_command` names the
workspace root, no constructor is root-less, and `tests/plugin_process_cwd.rs` is the behavioural
proof. Recorded here as closed so nobody redoes it | — |
| T2 | S1 | Named system baseline in the factory; RED `$PATH`/`$HOME` fixture first; reference page lists the keys | 1 |
| T3 | S3 | `protocol` in the envelope, typed mismatch error, PDK constant, wire golden updated | 2 |
| T4 | S5 | One golden per capability for the request bytes | 2 |
| T5 | S4 | Docs: request/response pairs for `manifest.v1`, `versioning.v1`, lifecycle, optionality rules | 3 |
| T6 | S9, S7 | Authoring guide: `sys.stdout.buffer`, stderr's fate, the PATH-of-the-parent oddity | 3 |
| T7 | S6 | Bounded stdout read with a typed overrun error | 4 |
| T8 | S8 | Invalid UTF-8 gets its own message | 4 |

Splitting note for the reviewer: T1 is already merged (#367). T2 is the one that changes behaviour a
user can feel; T3-T5 are the contract a non-Rust author codes against; T6 is documentation that makes
the first five usable; T7+T8 are hardening. Commits are ordered so each is reviewable alone, and T3
lands before T4 because the goldens pin the version field.

## Log

- **L1** — user, verbatim: «Realmentee no quiero borrar las APIs de plugins process, pero me gustaría
  hacerla lo más robusta posible, porque si un desarrollador de python quiere crear un plugin o
  cualquier otro lenguaje que no sea de los soportados, por eso lo incluí, pero me haz dicho varias
  veces como que no está completa o que pasan cosas, si se puede salvar para mantenerlas dime».
- **L2** — the decision that produced this plan: the process runtime stays and is made honest rather
  than deleted. Evidence for keeping it: it is the only runtime compiled in a lean build
  (`src/plugin/manager/factory.rs:78` is ungated), and eight integration suites build process plugins
  so the plugin machinery is exercisable without `--features plugins`
  (`tests/plugin_env_grants_at_invocation.rs:19-20`). Recorded in memory 152, topic
  `cutver/plugin/process-runtime-fate`.
- **L3** — honest correction to the earlier framing. #367 carries two contract-hygiene findings (the
  inherited cwd, and `permissions.filesystem`/`network` being silently inert for a process plugin).
  Neither is "the runtime is incomplete for a Python author"; *that* is S1, S3, S4 and S5, measured
  afterwards against the authoring surface. The third earlier candidate — "`args` is unreachable from
  config" — was **refuted**: no `args` key exists in the schema and
  `docs/reference/plugins.md:121` documents the empty argument list truthfully.
- **L4** — maintainer's disposition on this document: «No implementar todavía: solo el plan». Nothing
  here is authorized for implementation until the maintainer picks a scope; the file itself is
  uncommitted on `main` at `82d1843`.
- **L5** — T1 closed by #367 before this document was committed: `fda707c` on
  `fix/367-contract-honesty`, squash-merged as `c3b7390`. That change named the process child's
  working directory (S2 here) and made the required root a constructor parameter, so **no driver can
  be built without one**. It also gave the `doctor` grant rows their honest `effect`
  (`applied`/`ignored`), which is not part of this plan. What this plan still owes a non-Rust author
  is unchanged and starts at T2: **the system environment baseline**, the one item that needs a
  security decision (which keys, and whether inheriting them is opt-in).
- **L6** — state of the queue when this document was committed (2026-11-22): `main` = `c3b7390`;
  #300 and #367 closed; 11 issues open. The maintainer's disposition is «mergea, crea pr para lo que
  está sin commitear y lo que resta pendiente lo atacaremos cuando despierte» — so this lands as
  documentation and every task in it stays unimplemented and unauthorized until it is picked up.
