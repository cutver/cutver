# cutver-grants-at-invocation — issue #300

Resolve each plugin's declared `[plugins.<name>.permissions] env` grants when the plugin is
invoked, not while the configuration is loaded, so a diagnostic command stays runnable without
production secrets.

## Specs

Quoted verbatim from #300. These are the requirements; nothing else is one.

- **S1**. `cutver doctor` succeeds with a `[plugins.*.permissions] env` key unset, and prints that
  the grant is unset. RED first: it must fail on `7f6ff96`.
- **S2**. `cutver plugin restore` likewise succeeds with the grant unset.
- **S3**. Invoking the plugin with an unset grant still fails with the typed `UnsetEnvGrant` naming
  plugin and key — the #264 guarantee is unchanged.
- **S4**. The workflow steps `release.yml:116-120` keep their newly added `GITHUB_TOKEN` as a
  redundant, removable line; removing it must not break either step once the fix lands.
- **S5**. `git grep -n "resolve_env_grants" src/` shows no call inside a construction path — only
  inside invocation.

And the remedy, which names the shape:

> resolve the declared grants at invocation, not at construction. The driver should be able to
> exist with the *declared* keys unresolved and resolve them when the plugin is actually invoked
> (the process driver already builds the child environment at spawn; the WASM driver at
> instantiation). `doctor` should report an unset grant as a finding — a diagnostic row, which is
> what #268 is about — rather than failing to load.

## Tasks

| ID | Spec | Route | Commit |
| --- | --- | --- | --- |
| T1 | S1, S2 | RED integration tests: `doctor` and `plugin restore` with an unset grant | 1 |
| T2 | S3, S5 | `DeclaredEnvGrants` in `src/plugin/driver.rs`; process driver resolves at spawn | 1 |
| T3 | S3, S5 | wasm driver instantiates on first invoke, resolving grants there | 1 |
| T4 | S1 | `doctor` prints one row per declared env grant, set or unset, and carries it in `--json` | 1 |
| T5 | S5 | source-scan test: no construction path resolves grants | 1 |
| T6 | S4 | `release.yml`: the redundant `GITHUB_TOKEN` blocks become removable | 2 |
| T7 | — | source-line ratchet: split `src/cli/doctor.rs`, `driver/wasm.rs` and `driver/process/tests.rs` under 400 lines | 1 |

## Log

- **L1** — user, verbatim: «y continúa». The pinned plan (`cutver/next/resume-plan`, memory id 149)
  orders this session as: "### 1. `#300` — grants resolved at invocation. **Alone, whole session.**"
- **L2** — requirement change against the previous session: `tests/doctor_grants.rs` pins the D1
  decision ("`PluginManager::from_config` resolves `permissions.env` while it constructs the
  drivers … `doctor` reports the typed error as its own output instead"). S1 makes that premise
  false, so that test is rewritten to assert the new outcome, not deleted.
- **L3** — **S2 does not reproduce.** The pre-fix tree already ran `cutver plugin restore` on a host
  without the declared key: `restore_one` returns `Skipped` for a non-wasm runtime and
  `AlreadyPresent` for a local source *before* touching the registry or constructing any driver
  (`src/plugin/manage/restore.rs:107-127`). The new test passed against `22a409b` in 27s before a
  line of this change existed, so S2 is pinned rather than fixed; the issue's claim that restart
  fails the same way as `doctor` is not what the code does.
- **L4** — **public API change for the v0.22.0 notes** (same class as the `preflight::run` change
  already recorded): `PluginManager::from_config_with_lookup` and `WasmDriver::from_env_lookup` now
  take `EnvLookup` (`Arc<dyn Fn>`), `ProcessDriver::new` takes `impl Into<DeclaredEnvGrants>` (a
  literal `BTreeMap` still works, `ProcessDriver::env()` became `grants()`), and `WasmDriver` no
  longer instantiates at construction: a module that fails to compile now reports
  `WasmInitFailed` at the first invocation instead.
- **L5** — **load-sensitive flakiness, not this change.** Three descendant/tree-kill timing tests
  failed in three separate full-suite runs on a loaded machine — two here
  (`plugin::driver::process::tests::test_a_descendant_of_a_timed_out_plugin_never_writes_its_marker`,
  `tests/release_deferred_publish.rs::a_hook_timeout_kills_the_whole_process_tree`) and, on a clean
  `22a409b` worktree, `tests/process_driver_l3_budget.rs`. Each passes alone and on rerun; the
  baseline run is the evidence that this predates the branch.
- **L6** — T7 exists because the change pushed three files over `tests/source_line_budget.rs`: the
  work moved into `src/cli/doctor/plugins.rs`, `src/plugin/driver/wasm/tests.rs` and
  `src/plugin/driver/process/tests/env_grants.rs` rather than raising a cap. `("src/cli/doctor.rs",
  438)` was deleted from `ALLOWLIST` because the file now fits inside the 400-line ceiling.
