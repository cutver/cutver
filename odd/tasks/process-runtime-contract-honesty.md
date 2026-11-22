# process-runtime-contract-honesty — the process runtime's two lies (#367)

Issue #367 as a feature document. Two findings, one class: the process runtime and its diagnostic
report a state the runtime does not deliver. Neither changes what a config loads; both change what the
user is told.

## Specs

- **S1 — The child runs in the workspace root.** `ProcessDriver::child_command`
  (`src/plugin/driver/process.rs:84-89`) sets no `current_dir`, so the child inherits cutver's
  directory, against `CONTRACT.md:154-155` ("Every subprocess names its working directory explicitly
  and never inherits the caller's"). The child's working directory is the resolved `root_dir`. RED
  first: a plugin that prints its cwd, launched from an unrelated directory, prints that directory on
  `82d1843`. This is T1 of `odd/tasks/process-runtime-any-language.md`; that task is closed here.
- **S2 — `doctor` says what the runtime does with a grant.** For a `runtime = "process"` plugin, every
  `permissions.filesystem` entry currently renders as a healthy grant
  (`src/cli/doctor/plugins.rs:96-130` reads `filesystem_grants` without consulting the runtime), while
  only `permissions.env` is applied (`src/plugin/manager/factory.rs:95-110`,
  `docs/reference/plugins.md:158-160`). Each such row states that this runtime ignores it, and
  `doctor --json` carries the same fact under `plugins[].grants[]` — the field #268 already put there.
  RED first: a process plugin declaring `filesystem = ["assets"]` renders today as an applied grant.
  `permissions.network` is named by the issue and deliberately **not** touched here: `doctor` renders
  no row for `network` under either runtime (verified at `82d1843`), so there is nothing to correct
  and inventing that surface would be a new feature. Recorded so the omission reads as a decision.
- **S3 — The docs state it.** `docs/reference/plugins.md` and
  `docs/guides/plugin-process-runtime.md` say that the row marks these entries inert rather than
  applied, so the page that teaches the runtime and the page that prints the row agree.
- **S4 — Nothing stops loading.** A `cutver.toml` that declares `filesystem` or `network` on a
  `runtime = "process"` plugin still loads and runs exactly as on `82d1843`; only the diagnostic
  changes. `tests/unknown_config_keys.rs:267-273` keeps passing untouched. Refusing these keys at
  load is a breaking config change and belongs to a deprecation channel, not to this issue — see L3.
- **S5 — The examples the request named are run.** Both findings are covered by a test that exercises
  the public interface: `cutver doctor` for S2 (human row and `--json`), a spawned process plugin for
  S1.

## Tasks

| ID | Spec | Route | Commit |
| --- | --- | --- | --- |
| T1 | S1 | `child_command` sets `current_dir`; RED test spawns from an unrelated directory | 1 |
| T2 | S2 | `runtime` reaches the doctor record; filesystem/network rows marked inert for process; human row and `--json` | 2 |
| T3 | S3 | Reference page and process guide: what the row means | 3 |
| T4 | S1-S5 | gate (`scripts/check.sh`) and independent verifier | — |

## Log

- **L1** — issue #367, filed 2026-11-22, with the two findings and their evidence. The form is
  `contract-debt.yml`: a rule the contract declares and nothing enforces.
- **L2** — user, verbatim, on the order: «#367 primero y solo, después T2».
- **L3** — **the remedy for finding 2 changed after evidence, and that is recorded rather than
  quietly applied.** The issue proposed failing closed at load (`src/plugin/manager/factory.rs`),
  mirroring `capabilities` and `UnsetEnvGrant`. Checking first showed that remedy breaks two existing
  fixtures that declare sandbox grants on a process plugin — `tests/doctor_grants.rs:37-42` (whose
  assertion is the #268 contract that every `filesystem` entry gets a row) and
  `tests/unknown_config_keys.rs:267-273` — and, more importantly, that `doctor` already resolves and
  reports those grants *without consulting the runtime*, so the lie is in the report as much as in the
  driver. Reporting the inertness in the row fixes the report, keeps every config loading, and leaves
  the breaking variant for a deprecation channel. The maintainer delegated the choice ("qué nos
  convendría más"); this is the answer with its reasoning.
- **L4** — S4 exists because the reviewer's first question will be "does this break my config?". It
  does not, and the spec says so before the diff has to.
- **L5** — T2 (S2) and T3 (S3) done on `fix/367-contract-honesty`. RED, observed before the fix and
  with the driver untouched: `cargo test --locked --features plugins --test doctor_grants` exited
  `101`, and the process fixture's row printed
  `probe [changelog.v1]: "assets" -> …/assets -> …/assets (inside the workspace)` with no statement
  about the runtime, while `doctor --json` printed
  `{"canonical": …, "declared": "assets", "location": "inside", "resolved": …}` — no `runtime` on
  the plugin, no `effect` on the grant. GREEN: the same command exits `0`, 8 passed. The change: the
  plugin record carries `runtime`, each grant carries `effect` (`applied` | `ignored`, derived by an
  exhaustive match on `RuntimeKind`), and a process row ends
  `-> inert: runtime = "process" applies only permissions.env`. PRESERVE: a WASM fixture keeps the
  row text it had and reports `"effect": "applied"`. `tests/doctor_grants.rs` and
  `src/cli/doctor/plugins.rs` only; no validation, no factory, no config touched (S4), and
  `cargo test --locked --test unknown_config_keys` stayed green (17 passed).
- **L6** — T1 (S1) is **blocked on the allowed edit surface**, not on a technical obstacle. The
  resolved root never reaches the driver today: `PluginManager::from_config(plugins)`
  (`src/plugin/manager/core.rs:39`) takes the plugin map and nothing else, so `create_driver`
  (`src/plugin/manager/factory.rs:18`) and `ProcessDriver::new` see a `PluginConfig` and no
  `Config::root_dir`; the four call sites that do hold it (`src/bump/exec/doctor.rs:16`,
  `src/bump/exec/publish.rs:145`, `src/bump/plan.rs:246`, `src/cli/doctor/plugins.rs:180`) pass only
  `&config.plugins`. So the field the plan allows on `ProcessDriver` has no in-surface supplier, and
  setting `current_dir` to `std::env::current_dir()` instead would name the caller's directory rather
  than the workspace root — the very inheritance S1 removes, and the case `config::discovery`
  distinguishes when a configuration is found from a subdirectory or named with `--config`.
  Escalated to the orchestrator with the candidate paths; nothing of T1 was written.
- **L7** — T1 (S1) done, surface widened by the orchestrator to option 1 (root required everywhere,
  no root-less constructor left, `std::env::current_dir()` rejected). One constructor each:
  `ProcessDriver::new(name, root_dir, command, args, grants, timeout_seconds, capabilities)`,
  `create_driver(.., root_dir)`, `PluginManager::from_config(plugins, root_dir)` and
  `from_config_with_lookup(plugins, root_dir, lookup)`, all reached from `&config.root_dir`; no
  sibling constructor was added. RED, observed before the fix: `cargo test --locked --test
  plugin_process_cwd` exited `101` with
  `the child must run in the workspace root \\?\C:\…\cutver-e2e-process-cwd-…, not in the caller's
  directory \\?\D:\development\workspaces\cutver; it printed "D:\\development\\workspaces\\cutver\r\n"`.
  GREEN: the same command exits `0`, 2 passed, and the second test in the file proves the premise —
  the same probe spawned the way the driver used to spawn it prints the caller's directory. Unit
  half: `test_child_command_names_the_workspace_root_as_its_working_directory` reads
  `Command::get_current_dir()`, so the seam is asserted without spawning. Arithmetic: `
  src/plugin/driver/process/tests.rs` 348 → 396 (ceiling 400, 4 lines of headroom left, not
  allowlisted); `src/plugin/manager/tests.rs` 709 → 710 (allowlisted cap 719); docs 199/170/194/158,
  all under the 200-line ceiling. Two writer mistakes were self-fixed before any production edit: a
  malformed raw-string fixture in `tests/doctor_grants.rs` and a temporary-lifetime borrow in the new
  RED test. Not verifiable on this Windows host: the unix half of the cwd probe
  (`sh -c 'cat >/dev/null; pwd'`) never executed, because only the `cmd` branch runs here.

- **L8** — independent verification on the uncommitted tree: **S1 Met, S2 Met, S3 Met, S4 Met, S5
  partially met**, no blockers, three advisories. Evidence it produced rather than repeated: the RED
  was reproduced from a scratch baseline of `82d1843` with its own probe; S1's falsifiability was
  proven by mutation (removing `.current_dir` turns `plugin_process_cwd` and the unit assertion red)
  and the production file restored byte-identical; S2 was diffed baseline↔candidate on the real
  binary — the process row gains the inert clause, the env row and the whole WASM row are byte
  identical. Regression surface: `cargo test --workspace` 804 passed, `--features plugins` 894 passed,
  clippy and fmt clean, ratchet `process/tests.rs` 396/400 and `manager/tests.rs` 710/719.
- **L9** — **breaking public-API change, for the v0.22.0 notes**: `PluginManager::from_config` and
  `from_config_with_lookup` gain a workspace-root parameter, and `ProcessDriver::new` gains it as its
  second argument. No root-less constructor survives, deliberately — a root-less driver is the
  inheritance S1 removes. Same class as the `preflight::run` and `EnvLookup`/`DeclaredEnvGrants`
  changes already recorded for that release.
- **L10** — advisory 1 fixed in this change rather than deferred: the same guide said both "a relative
  path is taken from cutver's working directory" and "the working directory is the workspace root",
  and with `current_dir` set the first sentence became platform-dependent — Unix changes directory
  before `exec`, so a relative `command` resolves against the workspace root, while Windows'
  `CreateProcess` resolves it against the caller's directory. The guide now states both and still says
  to prefer an absolute path. The uniform rule (a `command` containing a separator resolves against
  `root_dir`, a bare name stays PATH resolution) is a behaviour change with its own tests, so it is
  not smuggled into this fix.
- **L11** — advisory 2 accepted, not fixed: no test asserts the wiring from `Config::root_dir` through
  `create_driver` into the driver, so a wrong root passed there would go unnoticed. Mitigation is the
  signature — one constructor, one pass-through, and the compiler now refuses a driver without a
  directory — so the gap is reviewable rather than absent. A behavioural test of it needs a fixture
  that spawns a directory printer through the binary, which is the next change's job, not this one's.
