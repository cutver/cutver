# cutver — the doctor reports what it resolves, and no read turns a failure into an empty value

Feature branch `fix/266-268-doctor-grants`, worktree `D:/development/workspaces/cutver-266-268`, based
on `origin/main` at `d829a2d` (v0.21.0).
Spec sources: GitHub issues **#268** and **#266**, both read in full. Every `S#` quotes them
verbatim; nothing here adds a requirement they do not state. The one decision the issues do not make
is recorded as `D1` and is the user's, not this unit's invention.

## Specs

**S1** (#268 evidence) — "`src/cli/doctor.rs:13-32` renders four fixed rows — Configuration, Manifests,
Preflight, and Changelog with `--check-changelog`. It prints **nothing** about plugins, capabilities or
`permissions.filesystem` grants."

**S2** (#268 evidence) — "`src/config/discovery.rs:47-63` **overwrites**
`PluginConfig.permissions.filesystem: Vec<String>` (`src/plugin/types.rs:166`) with the resolved guest
path, so the declared spelling is destroyed before anything downstream can print it."

**S3** (#268 evidence) — "`src/config/discovery.rs:70-85` (`resolve_grant`) holds `declared`,
`resolved` and `real` together and drops `real` on the success path (`:80`)." · "`src/plugin/driver/wasm.rs:123-126`
computes the canonical host path as a local and discards it; the only struct that ever carries both
values is the error variant `ConfigError::GrantOutsideWorkspace` (`src/config/types.rs:18-25`)."

**S4** (#268 failure mode) — "a grant that resolves somewhere unexpected is invisible until a guest
hits `EPERM`, and the error cannot say which host directory that was."

**S5** (#268 remedy) — "Retain the declared spelling alongside the resolved one — a new field, or a map
returned by `resolve_plugin_filesystem` and carried on `Config`." · "Render one row per grant in
`doctor`: `declared -> resolved -> canonical`, plus the plugin name and capability set."

**S6** (#268 acceptance criteria) — "`cutver doctor` on this repository prints one row per entry in
every `permissions.filesystem` grant." · "A test asserts the declared spelling survives resolution and
appears in the rendered output." · "The output distinguishes a grant resolving inside the workspace
from one that does not." · "`--json` output, if the command has it, carries the same three values."

**S7** (#266 contract) — Pillar III.4, *Zero Swallowed Errors*: "discarding process outputs without
checking status codes is forbidden. Every ignored condition, skipped hook, or non-critical failure must
be explicitly logged or tracked in the execution `Summary` with its causal rationale."

**S8** (#266 failure mode) — "`cutver changelog latest|show --json` emits an empty `raw_commits`
(contributors, first-time contributors, raw lines) and exits 0 when git fails. A drain-thread panic or
a short read on a child that exited 0 returns `Ok` with truncated stdout, and the caller reports an
opaque payload error with no cause."

**S9** (#266 remedy) — "propagate typed errors; a panicked drain thread is a bug, not an empty buffer.
Keep `unwrap_or_default()` only where the receiver is `Option` and the empty value is genuinely the
domain default."

**S10** (#266 acceptance criteria) — "A test points `changelog` at a repository where the git call
fails and asserts a non-zero exit with an actionable message instead of an empty JSON array." ·
"`git grep -n \"unwrap_or_default()\" src/` shows no remaining call on a `Result`, with the review
listing the four that remain and why."

**S11** (scope decision, the maintainer's, 2026-11-21) — this unit absorbs the `.ok()` half of #267:
the four `config::load(..).ok()` sites that discard a configuration error. #267 is thereby reduced to
validating `TagPrefix::format_tag` at construction, which is a contract change and **is not in this
unit**.

## D1 — the doctor's `.ok()` site is not fixed by propagating

`src/cli/doctor.rs` calls `PluginManager::from_config(&config.plugins).ok()`. Turning that into `?`
would satisfy S9's letter and break the command: `from_config` resolves `permissions.env` at driver
construction, so on a host with the grant unset (the #300 scenario) `doctor` would start **failing**
instead of reporting. #300 is open and out of scope here.

Resolution: `doctor` reports the load failure as part of its own output — a row or a JSON field
carrying the typed error — instead of discarding it and instead of propagating it. That satisfies S7
(the condition is tracked, not swallowed) without borrowing #300's fix. Record this in the code where
the error is handled.

## Established facts (do not re-derive)

- The line numbers in both issues were verified against `main@2f8805b` (v0.18.1). Two of #266's four
  cited sites are already fixed; the rest moved. Locate by construct, not by line.
- `src/plugin/driver/process.rs`'s drain joins cited by S8 now live behind `src/proc.rs` and arrive as
  a channel result rather than an unbounded `join()`; check what remains before changing it.
- `tests/source_line_budget.rs` enforces a 400-line ceiling on `src/` with a ratchet allowlist that
  "can shrink but never grow". Do not add an entry; if a file breaches, report it.

## Tasks

- **T1** — retain the declared spelling, the resolved guest path and the canonical host path in one
  place produced by `resolve_plugin_filesystem` and carried on `Config`. Specs: S2, S3, S5.
- **T2** — render one row per grant in `doctor` (`declared -> resolved -> canonical`, plugin name,
  capability set) and carry the same three values on the `--json` surface. Specs: S1, S5, S6.
- **T3** — make `doctor`'s plugin-manager load failure visible instead of `.ok()`-discarded, per D1.
  Specs: S7, D1.
- **T4** — propagate typed errors at the remaining `Result` → empty conversions in `src/`, including
  the four `config::load(..).ok()` sites. Specs: S7, S9, S11.
- **T5** — the `changelog` git-failure test: non-zero exit, actionable message, no empty JSON array.
  Specs: S8, S10.
- **T6** — docs for the new doctor rows and the `--json` keys. Specs: S6, S9.

## Log

**L1** (maintainer, 2026-11-21): the worktree pair was authorised («DALE»), and on the question of the
#267 split chose «Sí: B absorbe la mitad `.ok()` de #267».
