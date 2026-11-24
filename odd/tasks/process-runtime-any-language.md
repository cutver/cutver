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
- **S3 — The envelope carries a protocol version, and a mismatch is detectable.** `PluginInvocation` has
  three fields (`crates/cutver-pdk/src/invocation.rs:9-13`) and none states the version it speaks. A plugin
  built against an older shape fails today with `InvalidPayload`, indistinguishable from a malformed line.
  The shape was decided in L8, because the version cannot travel in the request alone: the only upward
  channel is stdout, so a request-only field tells the plugin what it is talking to and tells the host
  nothing.
  - **`protocol` is the first field of the invocation**, always written by the host from one constant,
    `PROTOCOL_VERSION = "1"`, exported by the PDK. Required, never defaulted: a default would recreate
    the silence this task removes. Serde ignores unknown keys, so a plugin built on PDK 0.1.x decodes the
    new request unchanged — compatibility does not depend on anyone upgrading.
  - **A plugin may declare the version it speaks** with a top-level `protocol` key beside its DTO fields.
    Present and different is a typed `ProtocolMismatch` naming the host's version and the plugin's, with
    `Where:` and `Fix:`. Absent means `"1"`, which is true of every plugin that exists today — and is why
    the key is optional in this direction: required would break the eight shell fixtures and
    `github-releases` with nothing to diagnose.
  - **The check lives where the answer is decoded** (`src/plugin/manager/core.rs`), not in a driver: a
    driver only serializes the request and never sees the answer's shape.
  - **`cutver-pdk` goes to `0.2.0`**: a published crate whose public struct gains a field is a breaking
    change for anyone matching on it exhaustively.
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
| T2 | S1 | **Done** — `6d5bd5c` on `feat/369-process-env-baseline`, plus `90202e6` and `f2d70fa` repairing the
fixtures that CI caught. One named list, `SYSTEM_ENV_BASELINE`, applied between `env_clear()` and the
declared grants in `child_command`, listed by name in the reference page. See L7 | 1 |
| T3 | S3 | **Done** — `236e47f` + `9fbc626` on `feat/369-protocol-version`, merged as `96e4189` (PR #371);
`cutver-pdk` 0.2.0 published to crates.io. See L9 | 2 |
| T4 | S5 | **Done** — `tests/plugin_request_goldens.rs`: one golden per request *shape* (seven, because
`manifest.v1` and `lifecycle.v1` each carry more than one), produced through the driver's own
`encode_invocation` so the framing has a single author, plus a `sha256` per request so `cutver/plugins`
reconciles with one line instead of a byte diff. See L10 | 2 |
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
- **L7** — **T2 (S1) implemented** on `feat/369-process-env-baseline`, opened as **PR #370**. The unit is
  four commits and 453 authored lines — past the 300-400 target, because CI found two fixture defects
  that are its own slices:
  - `6d5bd5c` — the behaviour (`feat(plugin): …`), 13 files, 346/51. **This is the hash to quote**;
    `5b32071` never landed, it was rewritten by an amend and two earlier notes pointed at it. A rewritten
    commit is a dangling reference, not evidence.
  - `417b627` — this document, updated when T2 closed.
  - `90202e6` — the dump fixture drank from a pipe the child had closed: `/usr/bin/env` and `cmd /C set`
    answer without reading stdin, so the host's envelope write races the exit and lands on
    `StdinWriteFailed { BrokenPipe }` on any host where the child wins. Windows and ubuntu won it; macOS
    lost it and turned the first CI round red. The fixture now drains first.
  - `f2d70fa` — **the fix's own bug**: with `sh` in the middle, `PWD` is synthesized by the shell, so
    naming it as an ambient key the driver must strip made the assertion about `sh` and turned all three
    unix legs red. `SHELL`, `LOGNAME` and `USER` are inherited and synthesized by nothing. Verified with
    `env -i /bin/sh -c 'cat </dev/null; exec /usr/bin/env'`.
  - **A third copy of the dump fixture lives in `src/plugin/driver/process/tests.rs` and was deliberately
    not touched**: the file is at 397 of its enforced 400-line budget and rustfmt expands the new literal
    into three lines. Known latent race, same one-line fix, waiting for whoever next opens that file.
  - **The descendant/timeout family flaked under my gate and I did not call it pre-existing without
    numbers.** Measured on this host, `cargo test --lib --features plugins`: branch 2 failures in 8 full
    runs, `main` 0 in 10; **isolated, the two tests are 0/10 on each side**. So it only reproduces under
    full-suite parallelism. 2/8 against 0/10 is not separable from chance (Fisher p ≈ 0.48), which means
    neither "my diff causes it" nor "it was already there" is established. The unit adds four
    process-spawning tests, which is a plausible load source and an unproven one. **Not fixed here** —
    those tests are timing-sensitive and deserve their own unit; the recipe above is the starting point.
  - The two decisions L5 said this task owed the maintainer, taken before the first write: the
    baseline is the twelve-key portable set — `PATH`, `HOME`, `USERPROFILE`, `TMPDIR`, `TEMP`, `TMP`,
    `LANG`, `LC_ALL`, `SystemRoot`, `SYSTEMDRIVE`, `APPDATA`, `LOCALAPPDATA` — one list for both
    platforms, where a key the host does not set **or sets empty** is omitted and never errors; and
    it applies **always**, with no new configuration key, so `permissions.env` keeps meaning "any
    other key this plugin needs".
  - **S1's wording moved by evidence**: the spec says "baseline in the factory", the implementation
    puts it in `ProcessDriver::child_command`, because that is the only place `env_clear()` exists and
    where #300 already resolves the grants per invocation. A factory-side list would have had to
    become a second driver field anyway, and `src/plugin/manager/**` is under the #300 source-scan pin.
    `ProcessDriver::new`'s signature is unchanged, which is why this unit breaks no API.
  - **Independent verification (`Risk: item 2`, the child's environment is a security boundary) found
    one blocking defect in the first attempt**: a baseline key the host exports as an empty string
    (`LANG=`) was handed to the child empty, contradicting the module's own rule and reddening the
    new test on such a host. Closed with a filter and its own RED-first unit test
    (`a_key_the_host_sets_to_the_empty_string_is_absent_from_the_map`), plus three documentation
    contradictions (`"GITHUB_TOKEN and nothing else"`, `"permissions.env narrows"`, and a doc comment
    that promised more than the secret-shape guard can detect).
  - **A scope drift worth remembering**: the reference page is at 199 of a hard 200-line ceiling
    (`tests/ci_workflow_contract.rs`), so listing twelve keys forced a trim. My first attempt took it
    out of two unrelated paragraphs; verification flagged that as reviewer-visible churn, the
    paragraphs were restored verbatim, and the page now sits at **200/200** — the next edit there must
    trim something. Editing a page that close to its ceiling is a scope decision, not a formatting one.
  - **State at close**: merged as **`bfeebe1`** (PR #370, squash) on 2026-11-22, all five test legs, `CI gate`,
    `Exactly one type label` and `Publish dry-run` green before the merge; the branch and its five commits are
    gone from the remote. Local `sh scripts/check.sh` was `9 passed, 0 failed, 1 not run here` (the linux+plugins
    clippy needs a cross C compiler this host lacks). The next task is **T3**.
- **L8** — the maintainer's decision on S3's shape, taken before any code: asked «¿por dónde viaja la versión de
  protocolo, para que el host pueda detectar el desacuerdo?», answered «Cuál me recomendas», which selects the
  recommended option — **request required, answer optional**. The question existed because S3 as first written was
  unimplementable: it asked for an error naming *both* versions, and the only upward channel is stdout while the
  answer is the DTO and nothing else (`docs/guides/plugin-overview.md:82-91`). Chosen because it is the only shape
  that produces that error without breaking the eight shell fixtures, `github-releases`, or any plugin built on PDK
  0.1.x. S3 rewritten above accordingly. The version string is `"1"`, following `SCHEMA_VERSION: u32 = 1`
  (`src/cli/json.rs:15`) instead of inventing a second versioning style in the same program.
- **L9** — **T3 (S3) implemented** on `feat/369-protocol-version` as `236e47f`, plus `9fbc626` for the S3 rewrite.
  254 authored lines, inside the target. Gate `9 passed / 0 failed / 1 not run here`; the not-run step is the
  linux+plugins clippy, which this host cannot compile.
  - **What landed**: `PROTOCOL_VERSION = "1"` and `protocol` as the envelope's first field, filled in
    `PluginInvocation::new`, so no call site changed; `dto::decode_answer` as the one answer-decode seam
    (`src/plugin/manager/core.rs` calls it), which is where the optional answer key is checked; and
    `ProtocolMismatch { name, host: &'static str, plugin: String }`, which keeps `PluginError` at 96 bytes —
    the budget in `src/plugin/error/tests.rs` did not move.
  - **The independent verifier (`Risk: item 3`) found two blocking defects in the first attempt.**
    **B1**: `decoded.get("protocol").and_then(Value::as_str)` ignored every non-string declaration, so
    `"protocol": 2` — the numeric style this program uses for `schemaVersion` — was read as protocol 1 and
    the call succeeded. **B2**: the new enum variant shifted `src/plugin/error.rs`, and six `path:line`
    citations in three guides went false, plus one `invocation.rs:5-13` that now pointed at the constant's
    doc block. Both closed, B1 with `a_protocol_this_host_cannot_read_is_refused_too`. **A doc citation is a
    claim: inserting ten lines in one file falsified six sentences in another, and nothing in the suite
    checks them.**
  - **Advisories closed**: the wire-contract module doc said "three top-level envelope fields" (now four),
    `docs/guides/plugin-authoring.md` still pinned `cutver-pdk = "0.1.0"` after the bump, and the
    "`on_post_*` accepts any JSON value" promise gained the exception that `protocol` is checked there too.
  - **Two follow-ups that are not in this repository** and block nothing here: the twin wire fixture lives in
    `cutver/plugins` (`plugins/github-releases/tests/fixtures/changelog_request.json`) and must move in the
    same breath — it is a *request*, so a PDK 0.2 plugin rejects the stale copy with `missing field protocol`,
    while a 0.1.x plugin keeps working because unknown keys are ignored; and `publish-pdk.yml` is manual, so
    that repository can only adopt 0.2.0 after a dispatch.
  - **Accepted asymmetry, worth knowing**: nothing on the host reads the request-side `protocol`; it is there
    so the *plugin* can refuse a host it cannot talk to. A PDK-0.2 plugin cannot talk to a pre-0.2 host
    (`missing field protocol`), which is the cost of "never defaulted", and it is not documented yet.
  - **Next**: T4, one golden per capability for the request bytes, now that the version field is pinned.
- **L10** — **T4 (S5) implemented** on `feat/369-request-goldens` as `2631260`. **Slice boundary recorded
  here because it is the first one past the ~400-line target**: 432 authored lines in one commit, of which
  391 are the new test module, 22 the extraction below, and the rest seven fixtures plus 8 `.gitattributes`
  lines. The reviewable production change is 22 added lines; the goldens are data.
  - **What landed**: `tests/plugin_request_goldens.rs` (four tests), `tests/fixtures/plugin_requests/` (seven
    files, no trailing newline, marked `-text` in `.gitattributes` as the sibling twin is), and
    `encode_invocation` as a free function in `src/plugin/driver/process.rs`.
  - **Seven goldens for four capabilities, deliberately**: `manifest.v1` carries `read` and `write`, and
    `lifecycle.v1` carries three payloads, so "one golden per capability" would have left four request
    *shapes* unpinned. The coverage assertion names every variant in an exhaustive `match` with no wildcard
    arm, so a new `PluginCall` variant breaks that test's compilation instead of slipping past a list nobody
    updated — which is what the first version of it did (see L11).
  - **The defect this unit found in its own first attempt**: pinning `serde_json::to_vec` *inside the test*
    pinned the serializer and not the driver — appending a newline in `ProcessDriver::invoke` would have left
    the suite green. Hence the extraction, so the test and the driver go through one function. Proven by
    mutation rather than by reading: `bytes.push(b'\n')` inside `encode_invocation` reddened the framing, the
    bytes and the digest assertions, and after reverting it `git diff` was byte-identical to the pre-mutation
    diff.
  - **The digest leg is feature-gated on purpose**: `sha2` is optional behind `plugins`, so the sha256
    assertions run only there. The byte assertions run on every leg, and the digest field is *read* on every
    leg too (length and lowercase-hex shape), which keeps `-D warnings` honest instead of an
    `allow(dead_code)`.
  - **A gate failure worth recording**: clippy with default features caught the `sha256` field as dead code
    that clippy with `--features plugins` could not see, because the only test reading it was compiled out. A
    feature-gated test that leaves a field unread is an error on the other leg, not a warning.
  - **Registry correction**: T3's row had no **Done** despite L9, so it was closed here. L7 needed nothing —
    it already records `bfeebe1` in "State at close", and the stale note that prompted the correction was
    itself wrong. A document that records state in prose gets read by whoever next opens it; reading it
    before editing it is cheaper than trusting a summary of it.
  - **Gate**: `9 passed, 0 failed, 1 not run here` — the `x86_64-unknown-linux-gnu` + `plugins` clippy needs
    `x86_64-linux-gnu-gcc`, absent on this host.
  - **Next**: **T5**, the request/response pairs in the docs (S4).
- **L11** — **independent verification of T4** (`Risk: item 3`, because the diff changes the body of
  `ProcessDriver::invoke`, which every process-plugin invocation goes through): **S1, S2 and S4 Met; S3 and S5
  Partial; no blockers**, four advisories — all four closed inside the unit before publishing.
  - **A1, the claim that overstated its own guard**: the module doc said framing "cannot be added at the
    driver" without failing. It can: the verifier traced the gap from `encode_invocation`'s call site
    (`src/plugin/driver/process.rs:161`) through `proc::run_capture` to the supervisor's `write_all`
    (`src/proc.rs:242`), where a newline or an early close leaves all seven tests green. The doc now names that
    gap instead of papering over it; closing it needs an end-to-end test with a real child, which this unit
    deliberately does not add.
  - **A2, the coverage assertion could not notice a variant**: it iterated a hand-written array, so an eighth
    `PluginCall` variant would have slipped past it green. It is now an exhaustive `match` with no wildcard arm,
    so adding a variant breaks the test file's compilation. L10's sentence was corrected, not left standing.
  - **A3, two fixtures held values production cannot emit**: `is_prerelease: true` beside `version: "1.2.0"` —
    the host derives that flag from the semver string (`src/changelog/context/assemble.rs`) — and a 20-hex sha
    where `src/git/log/range.rs` emits the full 40. The shapes were right and the *examples* were wrong, which
    is what a golden nobody compares against production decays into. Fixed to `1.3.0-rc.1` and a 40-character
    sha; two goldens and their two digests regenerated.
  - **A4**: the digest comparison is the only thing `--features plugins` adds, and the module doc says so
    rather than implying a second guard.
  - **What verification confirmed, kept here as evidence**: the extraction is byte-identical to `96e4189` (same
    `to_vec`, same error variant and field values), the `UnsupportedCapability` check still precedes the encode,
    all seven digests re-derived independently match their constants, and no fixture carries a trailing newline.
  - **The lesson, which cost one function extraction**: a test that pins a *value* is not a test that pins the
    *producer* of that value, and the difference stays invisible until someone asks what would have to change
    for the test to stay green while the behaviour broke.
