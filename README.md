<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/cutver-mark-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/cutver-mark-light.svg">
  <img alt="Cutver logo: a geometric C monogram combined with an emerald green verified release checkmark, separated by an atomic 45-degree cut" src="assets/cutver-mark-dark.svg" width="84" height="84" />
</picture>

# cutver

**A release engine you can extend: one core, four typed capability slots, and plugins that run sandboxed by default.**

> *Cut a release. Bump SemVer. Every project, every language.*

[![CI](https://github.com/cutver/cutver/actions/workflows/ci.yml/badge.svg)](https://github.com/cutver/cutver/actions/workflows/ci.yml)
[![Release](https://github.com/cutver/cutver/actions/workflows/release.yml/badge.svg)](https://github.com/cutver/cutver/actions/workflows/release.yml)
[![GitHub Release](https://img.shields.io/github/v/release/cutver/cutver?logo=github&color=blue&style=flat-square)](https://github.com/cutver/cutver/releases)
[![released with: cutver](https://img.shields.io/badge/released%20with-cutver-0969da?style=flat-square&logo=git&logoColor=white)](https://github.com/cutver/cutver)
[![Cosign Signed](https://img.shields.io/badge/cosign-signed_binaries-blue?style=flat-square&logo=sigstore)](https://github.com/cutver/cutver/releases)
[![Crates.io](https://img.shields.io/crates/v/cutver.svg?logo=rust&color=orange&style=flat-square)](https://crates.io/crates/cutver)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg?style=flat-square)](LICENSE)

<br/>

<p align="center">
  <a href="#-quickstart">Quickstart</a> •
  <a href="#-installation">Install</a> •
  <a href="#-capabilities">Capabilities</a> •
  <a href="#-documentation">Docs</a>
</p>

<br/>

<p align="center">
  <img src="assets/cutver_bump_showcase.png" alt="Terminal output of cutver bump auto: Showing the boxed SIMULATION MODE banner, SemVer minor version deduction from conventional commits, hierarchical release tree, and atomic Git tagging." width="680" />
</p>

</div>

---

## Overview

`cutver` is a standalone, language-agnostic release orchestration engine written in **Rust**. It synchronizes versions across your manifests, runs your preflight checks, renders the changelog, commits, tags, and publishes — driven by one declarative `cutver.toml`. It versions itself, tests itself, generates its own changelogs, and publishes itself.

What separates it from a simple version-bumping script: **four stages are typed capability slots**. The core orchestrates; the behavior is pluggable.

---

## ⚡ Quickstart

```bash
cutver init                  # Discover manifests, scaffold cutver.toml
cutver doctor                # Validate config, manifests, and version drift
cutver bump auto --dry-run   # Simulate the entire release without touching disk
cutver bump auto             # Compute in memory, apply atomically
```

Every mutating command validates in memory first and then applies **atomically**. Comments, key order, and formatting survive the diff, and a failure before the release commit rolls the workspace back cleanly rather than leaving it half-mutated. The tag and the publish commands run in the closing phase, after the artifacts built from that commit pass their verification — or, without `--defer-publish`, as the deterministic tail of the same run.

[Getting Started Walkthrough](docs/guides/getting-started.md) takes the same four commands and shows the output at every step. [Visual Tour](docs/guides/visual-tour.md) is the same flow in screenshots, and the [CLI reference](docs/reference/cli.md) has every flag and exit code.

---

## 📦 Installation

Two builds ship for every platform: **`cutver-plugins` (~5 MB)** with the full WebAssembly plugin runtime, and the lean **`cutver` (~1.25 MB)** core engine for minimal, zero-overhead CI runners. The plugin build is recommended unless the runner must stay tiny.

| Platform | Plugin build |
| :--- | :--- |
| Windows (Scoop) | `scoop install cutver-plugins` |
| macOS / Linux (Homebrew) | `brew install Row0902/tap/cutver-plugins` |
| Rust (Cargo) | `cargo install cutver --features plugins` |
| Containers | `docker pull ghcr.io/cutver/cutver:plugins` |

The [Installation Guide](docs/guides/installation.md) covers every package manager, the lean builds, the bucket and tap setup, and how to verify the install. The [Plugins reference](docs/reference/plugins.md) explains which build a given project needs.

---

## 🏛️ Capabilities

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/architecture-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="assets/architecture-light.svg">
    <img alt="Cutver Architecture: cutver.toml configures cutver core, which orchestrates four typed capability slots (manifest.v1, lifecycle.v1, changelog.v1, versioning.v1) backed by sandboxed WebAssembly plugins via cutver-pdk." src="assets/architecture-dark.svg" width="100%" />
  </picture>
</p>

| Capability | What a plugin takes over |
| :--- | :--- |
| `manifest.v1` | Read and write a manifest format the core does not know. |
| `lifecycle.v1` | Run on `on_pre_bump`, `on_post_bump`, and `on_post_release`. |
| `changelog.v1` | Render the release notes. |
| `versioning.v1` | Decide the next version from your commits. |

The core ships its own implementation of every slot, so nothing is required to cut a release. Plugins replace or extend a stage when your project needs it. A plugin is a **sandboxed WebAssembly module** or a native process, and both speak the same versioned contract ([`cutver-pdk`](https://crates.io/crates/cutver-pdk)).

Read on: [Architecture and design decisions](docs/internals/design.md) · [Plugin engine RFC](docs/rfcs/0001-wasm-plugin-architecture.md) · [Language manifests and diff guarantees](docs/reference/manifests.md) · [Installing and managing plugins](docs/reference/plugins.md) · [Pipeline execution and atomic guarantees](docs/internals/pipeline.md).

---

## 📚 Documentation

| Category | Guide | Description |
| :--- | :--- | :--- |
| **Guides** | [Getting Started Walkthrough](docs/guides/getting-started.md) | Step-by-step first release workflow and core concepts. |
| | [Installation Guide](docs/guides/installation.md) | Scoop, Homebrew, Cargo and containers, both builds, and verification. |
| | [Visual Tour](docs/guides/visual-tour.md) | The release lifecycle as terminal screenshots. |
| | [Monorepos & Multi-Package Strategies](docs/guides/monorepos.md) | Coordinated releases across polyglot workspaces without drift. |
| | [Writing a Plugin](docs/guides/plugin-authoring.md) | Scaffold a crate, export `invoke`, build for `wasm32-wasip1`, publish and register it. |
| | [Frequently Asked Questions](docs/guides/faq.md) | Comparison with other release tools, failure behavior, and the security model. |
| **Reference** | [cutver.toml Configuration Reference](docs/reference/configuration.md) | Canonical reference for every table, preflight timeout, and hook. |
| | [Language Manifests & Diff Guarantees](docs/reference/manifests.md) | Format-preserving byte-span scanner and AST editor guarantees. |
| | [CLI Commands & Exit Codes](docs/reference/cli.md) | Comprehensive command options, flags, and exit codes. |
| | [Installing & Managing Plugins](docs/reference/plugins.md) | Install flow, digest verification, cache location, and the plugin commands. |
| | [Official Insignia](docs/reference/insignia.md) | The "released with cutver" badge in copy-ready Markdown and HTML. |
| **Templates** | [Customizing Release Templates](docs/templates/release-notes.md) | Variables, commit filters, breaking descriptions, and CI attribution. |
| **Integrations** | [GitHub Actions Integration Guide](docs/integrations/github-actions.md) | Official `cutver/setup` and `cutver/release` action workflows. |
| | [Production Release Workflow](docs/integrations/production-release-workflow.md) | The bot-token workflow with `first_release` and `dry_run` inputs. |
| | [Container Integration Guide](docs/integrations/containers.md) | Running Cutver with Podman, Docker, `docker://`, and GitLab CI. |
| | [AI Coding Skills](docs/integrations/agent-skills.md) | Install the release skills bundle into Pi, Claude Code, Cursor, or Copilot. |
| **Internals** | [Architecture & Design Decisions](docs/internals/design.md) | Principles, plugin engine summary, and evolution roadmap. |
| | [Pipeline Execution & Atomic Guarantees](docs/internals/pipeline.md) | Two-phase release pipeline, rollback bounds, and process trees. |
| | [Plugin Engine Architecture](docs/rfcs/0001-wasm-plugin-architecture.md) | The microkernel, the capability contract, and the two runtimes. |

---

## License

MIT © [Cutver Authors](https://github.com/cutver/cutver) & [Row0902](https://github.com/Row0902)

---

<div align="center">
  <sub>Built with Rust for deterministic releases and universal developer access.</sub>
</div>
