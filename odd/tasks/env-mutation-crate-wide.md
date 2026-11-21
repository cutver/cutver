# #352 — no source under `src/**` mutates the process environment

`std::env::set_var` is `unsafe` in edition 2024 because it is undefined behaviour while another thread reads
the environment, and `cargo test --lib` runs every unit test as a thread of one process. #344 fixed the grant
resolver and pinned `src/plugin/**`; the independent verification of PR #351 found the remainder: three
writers in `src/cli` and `src/changelog`, outside the pin's scope. On macOS/Linux the `PATH` injection races
`shell_env()` and `descendant_env()`; on Windows it races `descendant_env`. The signature is #344's exactly — a
value disappears, `expect("the platform shell needs these keys to start")` fires, and the run is red for a
reason that has nothing to do with the change under review.

## Specs

**S1 — the pin watches the crate, not the module.** `tests/no_env_mutation.rs` (renamed from
`no_env_mutation_in_plugin.rs`) walks `src/**`, keeps a scanned-file floor that defeats a vacuous walk, and
reports `file:line`. It fails on the tree as it was: `["src/changelog/render/tests.rs:274 contains `set_var`",
"src/cli/ci.rs:18 contains `set_var`", "src/cli/open.rs:282 …", "src/cli/open.rs:288 contains `remove_var`",
"src/cli/runner/tests.rs:780 …", "src/cli/runner/tests.rs:784 …"]`, 168 files scanned.

**S2 — prose is not a call.** A line whose first non-space characters are `//` is documentation, not code, and
is not reported: `src/cli/ci.rs` explains in its module doc why it never calls the mutators, and a pin that
reported documentation would be taught to stop documenting. Pinned by the teeth test, which plants a doc
comment naming the mutator next to a real call and asserts exactly one finding.

**S3 — every reader stays a boundary; only the test's value moves.** `resolve_browser_command` (#352 item 2)
keeps reading the process and gains `resolve_browser_command_with_env(override, lookup)`;
`run_external` keeps reading `PATH` and gains `run_external_with_path(args, path_var)`; `create_environment`
keeps reading the process through `env_global` and gains
`create_environment_with_env_lookup(lookup)`. The real paths stay covered: `tests/e2e_open.rs` runs the real
binary with `.env("BROWSER", "true")`, and `test_external_subcommand_not_found` drives the real `PATH` read to
its not-found answer.

**S4 — the fixtures state their value and prove where it came from.** The browser fixture passes
`|key| (key == "BROWSER").then(…)` and pairs it with a control that injects nothing and must not see that
value; the env-global fixture renders `{{ env("CUTVER_TEST_ENV_VAR") }}` through two different lookups;
the external-subcommand fixture passes the search path it built instead of writing `PATH`.

**S5 — nothing else moves.** `grep -rn "set_var\|remove_var" src/` prints nothing outside comments; the
external subcommand still resolves `cutver-<subcmd>` with the same Windows extension order and still runs a
`.bat`/`.cmd` through `cmd /C`; `cutver open` still honours the override before `BROWSER` and `BROWSER` before
the platform default; the suite is green with and without the `plugins` feature.

## Evidence

- RED: `cargo test --test no_env_mutation` failed with the six findings above, 168 files scanned.
- GREEN: the same file passes with the three seams in place; `cargo test --lib` 509 passed, 0 failed.
- Gate: `sh scripts/check.sh` → **PASS** (9 passed, 0 failed, 1 not run here: the
  `x86_64-unknown-linux-gnu` plugins clippy needs a cross C compiler this host does not have — the gate names
  the reason instead of passing quietly).
- AC4 on this host: **20 consecutive `cargo test --lib` and 20 consecutive `cargo test --lib --features
  plugins`, 40/40 ok, 0 failures** (5m8s). The macOS and Linux legs of the same claim live in CI, which is where
  the original `PermissionDenied` / dropped-value signature appeared.

## Notes

- The ratchet decided where the test lives. `src/changelog/render/tests.rs` is allowlisted at 755 lines
  (`tests/source_line_budget.rs`) and the allowlist may only shrink, so the env-global test could not be added
  there: it first failed `an_allowlisted_file_may_not_grow` at 771. It moved to
  `src/changelog/render/template.rs`, beside the seam it proves, and the allowlisted file came back to 749. The
  alternative — raising the cap — is exactly what CONTRACT.md:174 forbids.
- `src/cli/runner.rs` re-exports `run_external_with_path` beside `run_external` so the fixture reaches it
  through `use super::*`; the seam is a sibling of the boundary, not a test-only back door.
- `env_global` stays private in `changelog/render/template.rs` and is passed to
  `create_environment_with_env_lookup` as the adapter, so the process read still has exactly one name.
- Observed while proving the control: an absent key renders as `None`, not empty, because the global returns
  `Option<String>`. That is pre-existing behaviour this change deliberately leaves alone — the control asserts
  provenance (`assert_ne!`) rather than freezing the rendering.
- Found, not fixed (out of #352's scope): `docs/templates/release-notes.md:115` documents
  `env("VAR_NAME", "default")` with a default argument, while `create_environment` registers a one-argument
  global (`template.rs`, `fn env_global(key: &str)`). A template that copies the documented two-argument call
  has no default to fall back on. Needs its own issue and a decision: accept a default, or correct the docs.
