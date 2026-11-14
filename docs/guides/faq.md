# Frequently Asked Questions

Short answers to the questions that decide whether cutver fits a project.

---

## How does cutver differ from `semantic-release`, `changesets`, or `cargo-release`?

Cutver is a standalone, language-agnostic release engine written in Rust. Unlike
ecosystem-specific tools (`cargo-release` for Rust, `changesets` for npm), Cutver operates across any
combination of ecosystems simultaneously (Cargo, npm, PyPI, Gradle, arbitrary text). Unlike
Node-heavy orchestrators (`semantic-release`), Cutver requires zero node runtime or dependencies,
executes in milliseconds, provides a guaranteed zero-residue rollback on preflight failure, and
exposes four typed capability slots extended through sandboxed WebAssembly plugins.

The prior-art comparison recorded during design lives in the
[workspace releases RFC](../rfcs/0002-workspace-releases.md).

---

## What happens if a preflight check, hook, or commit fails halfway through?

Cutver uses a strict **two-phase atomic execution model**:

1. **Phase 1 (In-Memory Validation):** Computes new versions, parses commits, checks changelog
   drift, runs preflight verification, and dry-runs every mutation in memory.
2. **Phase 2 (Atomic Apply):** Commits, tags, and runs post-hooks only once all validations pass.
   The tag and the publish commands can also be deferred to a closing `cutver publish` run, so CI
   can verify the built artifacts before anything irreversible happens.

If a failure occurs during preflights or staging, Cutver immediately aborts without touching disk,
leaving no uncommitted diffs, no partial manifest bumps, and no orphaned Git tags.

The exact boundaries are documented in the
[pipeline execution and atomic guarantees](../internals/pipeline.md) page.

---

## Do I need WebAssembly or Wasmtime if I only want minimal CI bumps?

No. Cutver provides two distinct builds:

- **`cutver` (Lean build, ~1.25 MB):** Standalone zero-overhead static binary containing the
  complete core engine (manifest parsing, SemVer deduction, atomic apply, preflight checks,
  changelog rendering). Perfect for lean, minimal CI/CD runners.
- **`cutver-plugins` (~5 MB):** Adds the full sandboxed WebAssembly plugin engine (`Wasmtime`) for
  projects leveraging custom plugins.

---

## Are third-party WebAssembly plugins secure? Can they access my filesystem or secrets?

Cutver enforces a strict **default-deny capability model**:

- Every WASM plugin runs isolated within Wasmtime with **zero filesystem, zero network, and zero
  environment access** by default.
- Permissions must be explicitly declared and granted in `cutver.toml` under
  `[plugins.<name>.permissions]`.
- There is no `--insecure` CLI bypass. Furthermore, every downloaded plugin has its SHA-256 digest
  verified prior to caching and re-verified on every load.

The trust model and permission keys are specified in the
[plugin reference](../reference/plugins.md).

---

## How does cutver preserve formatting, comments, and key ordering in manifests?

Rather than re-serializing entire configuration files (which destroys comments and formatting),
Cutver uses surgical, format-preserving AST editors (like `toml_edit` for TOML and byte-span
targeted surgery for JSON and Gradle). Only the exact version fields are mutated; comments,
whitespace, and existing key order survive the diff untouched.

Per-format guarantees are listed in the
[language manifests reference](../reference/manifests.md).

---

## Related Documentation

- [Installation Guide](../guides/installation.md): which build you need.
- [Getting Started Walkthrough](../guides/getting-started.md): the first release end to end.
- [Plugin Engine Architecture](../rfcs/0001-wasm-plugin-architecture.md): the microkernel behind the capability slots.
