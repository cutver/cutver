# cutver — Design Document

> Outcome first: `cutver` is a standalone Rust CLI that replaces per-project
> release scripts with one declarative, cross-language release tool driven by
> `cutver.toml`. This document describes the implemented design and the
> evolution roadmap; task plans live in `odd/tasks/`.
>
> **Status convention.** `[Implemented]` means this tree contains the code and
> the sentence cites its `path:line`; `[Partial]` means some of it ships;
> `[Planned]` means design intent only; `[Deferred]` means decided against for
> now, with a recorded reason and a reopen condition. A `[Planned]` item is never
> written as a key table, a configuration example, or present-tense capability
> language. `[External]` marks a capability delivered outside this repository.
> Unmarked prose describes structure or history, not a capability.

## Decision & Origin

The Angular/Rust academic-management project *Kairos* had a Bun/TypeScript
release script (`scripts/release.ts`) that hardcoded project paths, used
`execSync` with unchecked casts, synced only 3 of 5 version manifests, and
staged with a dangerous `git add .`. Rather than fixing it in place, the
decision was made to extract the concept — multi-manifest version lockstep +
preflight verification + changelog + tag — into a generic, reusable tool. This
section is history, not a capability claim.

**Name**: `cutver` ("cut a release" + "semver"). Verified available on both
crates.io and GitHub on the day of creation.

## Core Principles

1. **Zero project knowledge in the release pipeline.** [Implemented] Manifests
   are adapters and everything the pipeline needs lives in the repository's
   `cutver.toml`. Two deliberate exceptions carry ecosystem knowledge in the
   binary: `init`'s discovery knows the common manifests (`Cargo.toml`,
   `package.json`, `pyproject.toml` in `src/init/discovery.rs:137-161`), and
   staging knows the standard lockfile names (`KNOWN_LOCKFILES` in
   `src/bump/exec/command.rs:18`).
2. **Type-driven, panic-free Rust.** [Implemented] Edition 2024+, `thiserror`
   for the error hierarchy, the `semver` crate for version math. The
   "no `.unwrap()`/`.expect()` in production paths" rule is scoped to
   production paths: `#[cfg(test)]` modules and test-only constructors are
   exempt (for example the test-gated `GradleEditor::new`,
   `src/manifest/gradle.rs:28-31`).
3. **Declarative, deterministic, organic.** [Implemented] *Declarative*:
   configuration lives in `cutver.toml` (`src/config/types.rs`) and the CLI
   stays terse. *Deterministic*: identical Git states yield identical SemVer
   bumps (`src/conventional/deduce.rs`) and manifest diffs
   (`src/manifest/mod.rs`); the changelog entry is stamped with the date it was
   cut, so that part also depends on the day. *Organic*: the tool scales from a
   single-file crate to a polyglot repository through the same manifest
   adapters.
4. **Safe Git by default.** [Implemented] Requires a clean working tree and
   stages selectively the files `cutver` itself modified, plus recognized
   lockfiles the `post_bump` hook regenerated — not `git add .`
   (`src/git/ops.rs:7`, `src/bump/exec/command.rs:33`).
5. **Fail-fast preflight.** [Implemented] Runs the project's declared
   verification commands and aborts before any manifest mutation when a step
   fails (`src/preflight.rs`).
6. **Dry-run first.** [Implemented] `bump` is simulated end to end with
   `--dry-run`; `plugin add --dry-run` prints the plan and returns before
   `apply_add`, so it also skips the install-time digest gate
   (`src/cli/runner/plugin/add.rs:57`). `init` and `plugin remove` have no
   dry-run flag.
7. **Microkernel with a plugin extension surface.** [Implemented] The core
   release pipeline is small and capability-driven; optional integrations —
   manifest formats, changelog rendering, version computation, lifecycle
   events — are extensions behind the plugin engine (`src/lib.rs:1-12`).

## Module Layout

```text
src/
├── main.rs          # thin entry: parse args, run, exit with the returned code
├── lib.rs           # module roots; the library surface
├── atomic.rs        # same-directory atomic file replace
├── bump/            # pipeline orchestration, types
│   └── exec/{pipeline,transaction,manifest,changelog,command,doctor}.rs
├── cli/             # clap derive args (args/), runners (runner/), styling, tree/open/link
├── config/          # cutver.toml discovery, path, types, validation, preflight steps
├── conventional/    # Conventional Commits parser and bump deduction
├── changelog/       # context (assemble/parse), extract (parser/io), render (body/template), update
├── git/             # clean-tree/branch checks, commit, log, tags, remote, push
├── init/            # discovery + scaffold (config_gen/runner/templates)
├── manifest/        # ManifestEditor trait + json, json_scan, cargo_toml, pyproject, gradle, regex, plugin
├── plugin/          # cache, dto, manager, driver/{wasm,process}, registry, install, manage, error
├── preflight.rs     # fail-fast command runner with timeouts
└── semver_bump.rs   # patch/minor/major over the `semver` crate
```

## Plugin Engine

A design-level summary. The wire contract lives in
[RFC 0001: Cutver Microkernel & Unified Plugin Engine](../rfcs/0001-wasm-plugin-architecture.md)
and the user-facing surface in
[Installing and Managing Plugins](../reference/plugins.md).

- **Capabilities.** [Implemented] Four closed capability domains:
  `manifest.v1`, `lifecycle.v1`, `changelog.v1`, `versioning.v1`
  (`crates/cutver-pdk/src/types.rs:9-15`). Capability and operation are coupled
  at the host in one `PluginCall`, so an impossible pairing cannot be expressed
  (`src/plugin/types.rs:101-109`).
- **Drivers and feature gate.** [Implemented] `PluginDriver` is one trait with
  a WASM driver and a process driver (`src/plugin/driver.rs:1-20`). The process
  driver always compiles; the WASM driver and the `install`/`manage`/`registry`
  modules are gated behind the `plugins` Cargo feature, which is off in the
  default build (`src/plugin.rs:5-13`, `Cargo.toml:40`). The `plugin`
  subcommand is declared in every build but hidden without the feature and
  fails closed with an actionable message (`src/cli/runner/plugin.rs:1-6`).
- **Registry index and entry validation.** [Implemented] The index is an HTTPS
  base of `<name>.toml` entries; parsing validates every field and fails closed
  when the declared `name` disagrees with the requested file stem
  (`src/plugin/registry/entry.rs:77-97`, `src/plugin/registry.rs:36-49`).
- **Two digest gates.** [Implemented] *Install time*: `apply_add` verifies the
  downloaded bytes against the registry `sha256` before the artifact is renamed
  into the cache (`src/plugin/install/apply.rs:52-60`). *Load time*: the loader
  re-verifies the cached file against the `hash` in `cutver.toml`
  (`src/plugin/driver/wasm.rs:171-183`). Each gate is skipped under exactly two
  conditions: the install-time gate is skipped whenever `apply_add` is not
  reached, which includes `plugin add --dry-run`; the load-time gate is skipped
  when the entry has no `hash` (`verify_sha256` returns `Ok(())` for `None`,
  `src/plugin/driver/wasm.rs:93-96`).

## CLI Surface

```text
cutver [-c <PATH>] bump <patch|minor|major|auto> [--first-release] [--dry-run] [--skip-preflight <step>...]
cutver [-c <PATH>] doctor [--check-changelog]
cutver [-c <PATH>] changelog <latest|show|open> [...]
cutver [-c <PATH>] open [<target>] [--print-url] [--browser <cmd>]
cutver [-c <PATH>] init [--update] [--force] [--path <dir>] [--no-template]
cutver [-c <PATH>] plugin <add|list|remove> [...]        # plugin-enabled builds only
```

`-c` / `--config` is a global flag (`src/cli/args/commands.rs:8-11`);
`changelog` has `latest`, `show`, and `open` subcommands
(`src/cli/args/commands.rs:83-130`); `plugin` is hidden unless the feature is
on (`src/cli/args/commands.rs:73-74`). Flag-level detail and exit codes live in
the [CLI reference](../reference/cli.md).

## Configuration: `cutver.toml`

[Implemented] `cutver.toml` is discovered by walking up from the current
directory, stopping at a repository boundary, or supplied with `-c <path>`;
paths inside it resolve relative to its own directory (`src/config/discovery.rs`).
The keys are owned by the [configuration reference](../reference/configuration.md):
`[version]`, `[[manifest]]`, `[preflight]`, `[changelog]`, `[git]`, `[hooks]`,
`[publish]`, `[plugins.<name>]`. This document does not restate them, so a key
table cannot drift.

## Manifest Kinds

[Implemented] `cargo-package`, `json`, `pyproject`, `gradle`, `regex`, and
`plugin`, dispatched by `manifest::editor_for` (`src/manifest/mod.rs:49-109`).
The per-kind guarantees, the surgical-diff limits, and the staged-lockfile
allowlist are owned by the [manifest reference](../reference/manifests.md).

## Evolution Roadmap

Status is the point of this table. It covers the capabilities the roadmap named,
not a feature inventory; the reference pages are the inventory. `[Implemented]`
cites the deciding `path:line`; `[Partial]` names what is missing; `[Planned]`
names what was searched for and not found.

| Capability | Status | Evidence |
| :--- | :--- | :--- |
| Canonical `cutver.toml` | Implemented | `src/config/discovery.rs` |
| Primary manifest (`primary = true`, `current_source`) | Implemented | `src/config/types.rs:71-77` |
| Conventional Commits and `bump auto` | Implemented | `src/conventional/deduce.rs:51` |
| Structured changelog categorization | Implemented | `src/changelog/render/body.rs:155-200` |
| `post_bump` hook and `[publish]` hooks | Implemented | `src/bump/exec/pipeline.rs:272,329` |
| `cutver init` | Implemented | `src/init/scaffold/runner.rs:84` |
| `cutver doctor` version and changelog checks | Implemented | `src/bump/exec/doctor.rs:15,39` |
| `cutver doctor --fix` | Planned | No `--fix` flag exists; `doctor` runs version and changelog checks (`src/cli/args/commands.rs:50-54`). |
| `glob` in `[[manifest]]` | Planned | No glob for manifest paths; the only glob is the plugin `manifest_match` (`src/cli/args/plugin.rs:45`). |
| Cargo `[workspace.package]` / `version.workspace = true` | Planned | Unsupported; the editor reads and writes `package.version` only (`src/manifest/cargo_toml.rs:15,28`). |
| Independent workspace mode (`--package`, path filtering) | Deferred | No `--package` flag and no independent release unit. The changelog does group commits per declared `package` identity ([configuration reference](../reference/configuration.md)), which reports history per package without changing what ships; the independent mode itself stays deferred, with reasons and a reopen condition in [ADR-0002](../adr/0002-shared-version-with-path-filtered-changelogs.md). |
| Internal workspace dependency cascade | Planned | No code updates inter-manifest dependency constraints. |
| Pre-release channels (`--channel`, `bump release`) | Planned | No `--channel` on `bump` and no `release` level (`src/cli/args/commands.rs:35-47`). |
| GitHub Actions step outputs | External | Emitted by the `cutver/release@v1` action, not by this tree (`$GITHUB_OUTPUT` does not appear in `src/`). |
| Pi release skill | Implemented | Shipped as `cutver-release` (planned as `release-creator`): drives `cutver bump` with a mandatory dry-run simulation and health checks (`.pi/skills/cutver-release/SKILL.md`). |

No `[Planned]` row appears as a configuration key or an example above; those
sections describe only shipped behaviour.
