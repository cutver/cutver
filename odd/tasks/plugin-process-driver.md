# Feature: ProcessDriver JSON IPC and Timeout Isolation (Issue #144)

> **Status**: In Progress  
> **Target Version**: v0.11.0  
> **Issue**: [#144](https://github.com/cutver/cutver/issues/144)  
> **Constitutional Law**: [CONTRACT.md](../../CONTRACT.md) (Pillars I, III, V, VI)

---

## Acceptance Criteria
- [ ] Implement `ProcessDriver` in `src/plugin/driver/process.rs` conforming to `PluginDriver`
- [ ] Enforce capability validation (`Capability` check before invocation)
- [ ] Safe subprocess execution with piped stdin/stdout/stderr
- [ ] Timeout isolation with fail-closed child process termination
- [ ] Unit & regression tests covering happy path, capability mismatch, non-zero exits, and timeouts
- [ ] Zero clippy warnings, canonical formatting, and green test suite

---

## Task Breakdown
- [x] Task 1: Create `src/plugin/driver/process.rs` and wire into `src/plugin/driver.rs`
- [x] Task 2: Implement execution engine with stdin writing, stdout reading, and timeout thread/polling
- [x] Task 3: Implement comprehensive unit tests in `process.rs`
- [x] Task 4: Verify test suite, clippy `-D warnings`, and fmt
- [x] Task 5: Close with atomic Conventional Commit linked to #144

