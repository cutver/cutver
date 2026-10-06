# Feature: Git-style External Subcommand Dispatch (Issue #32)

> **Status**: In Progress  
> **Target Version**: v0.11.0  
> **Issue**: [#32](https://github.com/cutver/cutver/issues/32)  
> **Constitutional Law**: [CONTRACT.md](../../CONTRACT.md) (Pillars I, III, IV, V, VI)

---

## Acceptance Criteria
- [x] Add `#[command(external_subcommand)] External(Vec<String>)` to `Commands` enum in `src/cli/args/commands.rs`.
- [x] Implement `run_external_subcommand(args: &[String]) -> i32` in `src/cli/runner.rs` (or modular submodule `src/cli/runner/external.rs`).
- [x] Resolve binary name `cutver-<subcommand>` and execute via `std::process::Command` passing trailing arguments.
- [x] Inherit `stdin`, `stdout`, and `stderr` for transparent interactive or piped CLI execution.
- [x] Propagate child process exit status code cleanly to `cutver` main exit.
- [x] Provide actionable error message per CONTRACT.md when binary is not found in `$PATH` (What failed, Where, Fix).
- [x] Unit & E2E integration tests in `tests/` verifying dispatch to mock binary, argument forwarding, exit code propagation, and missing binary error.
- [x] Zero clippy warnings, canonical formatting, and green test suite.

---

## Task Breakdown
- [x] Task 1: Add `External(Vec<String>)` variant in `src/cli/args/commands.rs`.
- [x] Task 2: Implement external subcommand runner with process spawning and exit code forwarding.
- [x] Task 3: Invert missing binary into actionable CONTRACT.md error message on stderr.
- [x] Task 4: Add unit and integration tests verifying external binary execution.
- [x] Task 5: Verify full test suite, clippy `-D warnings`, and fmt.
- [ ] Task 6: Close with atomic Conventional Commit linked to #32.
