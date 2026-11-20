# #344 — stop mutating the process environment to test an env grant

`plugin::driver::process::tests::test_only_granted_env_keys_reach_the_plugin` failed once on macOS and passed on the
rerun of the same commit: `resolve_env_grants` did not find a key the test itself had set three lines earlier.
`std::env::set_var` is `unsafe` because it is undefined behaviour when other threads read the environment, and other
tests in the same binary do — `shell_env()` calls the same resolver, and driver tests spawn children. The value was
dropped, not stale. A suite that fails one run in N teaches re-running instead of reading (#318's lesson).

## Specs

**S1 — no test in the crate mutates the process environment again.** `set_var` and `remove_var` appear nowhere under
`src/plugin/**`, and a committed test proves it by walking the source, so a future fixture cannot reintroduce it
silently.

**S2 — the resolver keeps its contract and gains a pure sibling.** `resolve_env_grants(plugin, declared)` keeps its
signature and behaviour: a missing key and a non-UTF-8 value are both `PluginError::UnsetEnvGrant` naming plugin and
key, never a silent `""`. A sibling takes the lookup instead of reading the process, mirroring the convention the
crate already uses twice — `plugin::cache::from_env_lookup` + `resolve()` and `cli::ci::from_env_lookup` + `resolve()`.

**S3 — the injected lookup is what the tests exercise.** A present key resolves to its value; an absent key is
`UnsetEnvGrant` naming plugin and key and pointing at `[plugins.<name>.permissions]`; a value the lookup cannot produce
stands for the non-UTF-8 case and is `UnsetEnvGrant`, never `""`.

**S4 — the child receives exactly the granted keys, proven without the environment.** The driver-level test passes the
map into `ProcessDriver::new` directly, asserts the granted key reaches the child, asserts a key that was never passed
is absent, and keeps the credential-leak guard: the child's environment dump is never formatted into a panic message.

**S5 — the manager path is drivable with an injected lookup.** `PluginManager::from_config` stays the boundary binding
to the process environment; a sibling takes the lookup, so its `Ok` and `UnsetEnvGrant` cases need no mutation.

**S6 — nothing else moves.** The process driver still `env_clear()`s before `envs`; the wasm driver still fails
construction on an unset grant rather than reaching the guest with an empty string; the suite is green with and without
the `plugins` feature.

## Constraints

- `src/plugin/manager/tests.rs` is allowlisted at **719** lines by `tests/source_line_budget.rs` and may not grow: the
  change there must delete more than it adds.
- `tests/source_line_budget.rs` also caps `src/**/*.rs` at 400 lines unless allowlisted; report any file that
  approaches it.
- Never touch `.git/gentle-ai/candidate-views/**` (stale copies of the tree).

## Tasks

| ID | Task | Spec | Route | Commit |
| :--- | :--- | :--- | :--- | :--- |
| T1 | Pure resolver plus boundary constructors through the driver, factory and manager | S2, S5 | `gentle-ai-worker` | _pending_ |
| T2 | Rewrite the three mutating tests and add the no-mutation pin | S1, S3, S4, S6 | `gentle-ai-worker` | _pending_ |
| T3 | Independent verification, gate, commit and PR | all | `gentle-ai-verify` | _n/a_ |

## Log

**L1 — the issue is the brief.** `cutver/cutver#344` carries the failure, the race mechanism with `path:line`, the fix
direction and four acceptance criteria; this document only fixes the surfaces and the constraints for the session that
implements it.
