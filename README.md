<div align="center">

# cutver

**Cut a release. Bump SemVer. Every project, every language.**

> *cutver versions itself, tests itself, generates its own changelogs, and publishes itself.*

[![CI](https://github.com/cutver/cutver/actions/workflows/ci.yml/badge.svg)](https://github.com/cutver/cutver/actions/workflows/ci.yml)
[![Release](https://github.com/cutver/cutver/actions/workflows/release.yml/badge.svg)](https://github.com/cutver/cutver/actions/workflows/release.yml)
[![GitHub Release](https://img.shields.io/github/v/release/cutver/cutver?logo=github&color=blue)](https://github.com/cutver/cutver/releases)
[![Cosign Signed](https://img.shields.io/badge/cosign-signed_binaries-blue?logo=sigstore)](https://github.com/cutver/cutver/releases)
[![Crates.io](https://img.shields.io/crates/v/cutver.svg?logo=rust&color=orange)](https://crates.io/crates/cutver)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

<br/>

<p align="center">
  <img src="assets/demo.svg" alt="cutver bump auto preview" width="680">
</p>

</div>

---

`cutver` is a standalone, format-preserving release orchestration engine written in pure Rust. It synchronizes version numbers across any set of manifests, runs verification checks with process-tree timeouts, renders changelogs with expressive release templates, commits, tags, and publishes — all driven by one declarative `cutver.toml` configuration.

**Zero Node.js runtime. Zero external dependencies. Zero broken comments or stripped formatting.**

---

## 30-Second Mental Model

```text
       cutver.toml (single source of truth)
                       │
         ┌─────────────┼─────────────┐
         ▼             ▼             ▼
   Cargo.toml    package.json   build.gradle.kts
  (toml_edit)    (byte-span)        (regex)
         │             │             │
         └─────────────┼─────────────┘
                       ▼
       Preflight Checks (Process Groups)
                       ▼
       Conventional Commits SemVer Math
                       ▼
       Atomic In-Memory Write + Rollback Guard
                       ▼
       Release Commit + SemVer Tag + Push
```

`cutver` treats releases as atomic transactions. It computes all diffs in memory, verifies preflights, applies surgical single-line updates without disturbing comments or formatting, and automatically rolls back if any step fails.

---

## Quick Install

```bash
# Homebrew (macOS & Linux)
brew install Row0902/tap/cutver

# Scoop (Windows)
scoop bucket add row https://github.com/Row0902/scoop-bucket && scoop install cutver

# Cargo
cargo install cutver

# OCI Container (Docker / Podman)
docker pull ghcr.io/cutver/cutver:latest
podman pull ghcr.io/cutver/cutver:latest
```

### Precompiled Binaries

Download Cosign-signed binaries from [GitHub Releases](https://github.com/cutver/cutver/releases) for 8 modern 64-bit platforms:

| Platform | Target Slug | Target Triple | Archive |
| :--- | :--- | :--- | :--- |
| **Linux x86_64** (glibc) | `linux-x86_64` | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| **Linux x86_64** (musl) | `linux-musl-x86_64` | `x86_64-unknown-linux-musl` | `.tar.gz` |
| **Linux ARM64** (glibc) | `linux-arm64` | `aarch64-unknown-linux-gnu` | `.tar.gz` |
| **Linux ARM64** (musl) | `linux-musl-arm64` | `aarch64-unknown-linux-musl` | `.tar.gz` |
| **macOS Intel** | `macos-x86_64` | `x86_64-apple-darwin` | `.tar.gz` |
| **macOS Apple Silicon** | `macos-arm64` | `aarch64-apple-darwin` | `.tar.gz` |
| **Windows x86_64** | `windows-x86_64` | `x86_64-pc-windows-msvc` | `.zip` |
| **Windows ARM64** | `windows-arm64` | `aarch64-pc-windows-msvc` | `.zip` |

---

## The 3-Step Happy Path

```bash
# 1. Detect project manifests and scaffold cutver.toml
cutver init

# 2. Verify manifest health and check for version drift
cutver doctor

# 3. Simulate release with dry-run, then cut the release
cutver bump auto --dry-run
cutver bump auto
```

---

## Documentation Directory

Explore the complete modular guides and technical references:

| Category | Guide | Description |
| :--- | :--- | :--- |
| **Guides** | [Getting Started Walkthrough](docs/guides/getting-started.md) | Step-by-step first release workflow and core concepts. |
| | [Monorepos & Multi-Package Strategies](docs/guides/monorepos.md) | Coordinated releases across polyglot workspaces without drift. |
| **Reference** | [cutver.toml Configuration Reference](docs/reference/configuration.md) | Canonical reference for every table, preflight timeout, and hook. |
| | [Language Manifests & Diff Guarantees](docs/reference/manifests.md) | Format-preserving byte-span scanner and AST editor guarantees. |
| | [CLI Commands & Exit Codes](docs/reference/cli.md) | Comprehensive command options, flags, and exit codes. |
| **Templates** | [Customizing Release Templates](docs/templates/release-notes.md) | Variables, commit filters, breaking descriptions, and CI attribution. |
| **Integrations** | [GitHub Actions Integration Guide](docs/integrations/github-actions.md) | Official `cutver/setup` and `cutver/release` action workflows. |
| | [Container Integration Guide](docs/integrations/containers.md) | Running cutver with Podman, Docker, `docker://`, and GitLab CI. |
| **Internals** | [Architecture & Design Decisions](docs/internals/design.md) | Two-phase release pipeline, atomic scanner, and design history. |
| | [Technical Specification Archive](docs/internals/references.md) | Comprehensive legacy reference and internal specifications. |

---

## AI Coding Skills

Equip coding assistants (Pi, Claude Code, Cursor, GitHub Copilot) with release skills from [Cutver Agent Skills](https://github.com/cutver/skills):

```bash
npx skills add cutver/skills
```

Includes autonomous skills for `cutver-release` (with mandatory `--dry-run` simulation), `cutver-doctor`, `cutver-init`, and `cutver-changelog`.

---

## License

MIT © [Cutver Authors](https://github.com/cutver/cutver) & [Row0902](https://github.com/Row0902)
