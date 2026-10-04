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

`cutver` is a standalone, format-preserving release orchestration engine written in pure Rust. It synchronizes version numbers across any set of manifests, runs verification checks with process-tree timeouts, renders changelogs with expressive MiniJinja templates, commits, tags, and publishes — all driven by one declarative `cutver.toml` configuration.

**Zero Node.js runtime. Zero external dependencies. Zero broken comments or stripped formatting.**

---

## Why cutver?

| Principle | Guarantee |
| :--- | :--- |
| **Universal & Polyglot** | No hardcoded frameworks. Manage Rust, Node, Tauri, Android, Python, Go, or complex monorepos with equal fidelity. |
| **Format-Preserving** | Custom byte-span scanner for JSON and `toml_edit` for TOML. Comments, key order, quotes, indentation, and newlines stay untouched. Diffs are strictly 1 line per file. |
| **Atomic & Resilient** | Two-phase release pipeline: all edits are computed in memory first. Fail-safe RAII transaction guards ensure automatic rollback upon write, hook, or staging failure. |
| **Automated SemVer** | `cutver bump auto` inspects Conventional Commits since the last release tag to deduce whether to cut a patch, minor, or major bump. |
| **MiniJinja Release Notes** | Built-in template engine with native filters (`group_by_type`, `group_by_scope`), migration guides (`breaking_description`), commit bodies, and CI/CD attribution (`env()`). |
| **Accessible Terminal UX** | Interactive OSC 8 hyperlinks with accessible footnote references (`[^1]`, `[^2]`), clean noise-free companion banners for screen readers (NVDA), and workspace-relative diagnostics. |
| **Browser Dispatch (`open`)** | Launch release pages, tags, and compare diffs in your default browser from the terminal, with `$BROWSER` resolution and native WSL `wslview` support. |
| **Parallel & Fast** | Bounded parallel evaluation with `rayon` for instant read-only manifest computation, drift diagnostics, and monorepo discovery. |
| **Automatic Lockfile Staging** | Declarative `post_bump` hooks automatically detect and stage lockfiles (`Cargo.lock`, `package-lock.json`, `pnpm-lock.yaml`, `bun.lock`, `uv.lock`, `poetry.lock`), followed by optional automated push. |

---

## Quick Start

### 1. Install `cutver`

**Via Homebrew (macOS & Linux):**
```bash
# Recommended:
brew install Row0902/tap/cutver

# Or tapping separately:
brew tap Row0902/tap && brew trust Row0902/tap && brew install cutver
```

**Via Scoop (Windows):**
```powershell
scoop bucket add row https://github.com/Row0902/scoop-bucket
scoop install cutver
```

**Via Cargo:**
```bash
cargo install cutver
```

**Via Precompiled Binaries:**
Download cryptographic Cosign-signed binaries directly from [GitHub Releases](https://github.com/cutver/cutver/releases) for Linux (GNU/Musl), macOS (Apple Silicon/Intel), and Windows.

---

### 2. Initialize `cutver.toml`

Run `cutver init` to discover your project manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, Gradle, Tauri, etc.), scaffold a production-ready MiniJinja release template at `.github/templates/cutver/RELEASE.md`, and generate an idiomatic `cutver.toml` and starter `CHANGELOG.md`:

```bash
cutver init
```

If you add new manifests or sub-crates later, update your existing configuration without losing custom settings:

```bash
cutver init --update
```

A generated `cutver.toml` configures conventional bumping and manifests automatically. By convention, the first declared manifest serves as the primary source of truth:

```toml
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "pyproject.toml"
kind = "pyproject"

[changelog]
path = "CHANGELOG.md"
format = "keep-a-changelog"
mode = "template"
template_file = ".github/templates/cutver/RELEASE.md"

[hooks]
post_bump = "cargo check --workspace"

[publish]
push = true
```

---

### 3. Cut a Release

```bash
# Preview what would happen without touching disk or Git
cutver bump auto --dry-run

# Run preflight, bump manifests, update changelog, commit, tag, and publish
cutver bump auto
```

<p align="center">
  <img src="assets/cutver_bump_showcase.png" alt="cutver bump auto simulation release plan" width="850">
</p>

For first/initial releases where manifests are already at `0.1.0` (or `1.0.0`) and should not be incremented:
```bash
cutver bump --first-release # or -fr
```

You can also specify explicit bump levels at any time:
```bash
cutver bump patch
cutver bump minor
cutver bump major
```

---

### 4. Open in Browser (`cutver open`)

Quickly inspect your releases, tags, or comparison ranges in the browser directly from your terminal:

```bash
# Open repository forge root
cutver open

# Open a specific release tag
cutver open 0.10.0

# Open a comparison diff between tags or HEAD
cutver open compare
cutver open v0.9.1...v0.10.0

# Print the resolved URL to stdout without launching a browser
cutver open 0.10.0 --print-url
```

> **Cross-Platform & WSL Parity**: Respects the standard `$BROWSER` environment variable and automatically detects WSL to dispatch to `/usr/sbin/wslview` without requiring X11.

---

### 5. Verify Health & Drift (`cutver doctor`)

Check for version divergence across your declared manifests before cutting a release:

```bash
cutver doctor
```

<p align="center">
  <img src="assets/cutver_doctor_showcase.png" alt="cutver doctor configuration and manifest verification" width="750">
</p>

- **Exit 0**: Configuration is valid and all manifests are synchronized.
- **Exit 1**: Invalid configuration or manifest read error.
- **Exit 2**: Version drift detected across manifests.

You can also verify changelog consistency against Git release tags:
```bash
cutver doctor --check-changelog
```

---

## Dynamic Release Notes with MiniJinja

`cutver` features an expressive, embedded templating engine powered by [MiniJinja](https://github.com/mitsuhiko/minijinja). You can customize your release notes with `.github/templates/cutver/RELEASE.md` or custom templates:

```jinja
## [{{ tag }}] - {{ date }}

{%- if breaking %}
### ⚠️ Breaking Changes
{% for c in commits if c.is_breaking -%}
- {{ c.line }}
{%- if c.breaking_description %}
  > ⚠️ **Migration**: {{ c.breaking_description }}
{%- endif %}
{% endfor %}
{%- endif %}

{%- for type, type_commits in commits | group_by_type %}
{%- if type == 'feat' %}
### 🚀 Features & Enhancements
{%- elif type == 'fix' %}
### 🐛 Bug Fixes
{%- elif type == 'perf' %}
### ⚡ Performance Improvements
{%- elif type == 'refactor' %}
### 🔄 Code Refactoring
{%- elif type == 'docs' %}
### 📚 Documentation
{%- elif type in ['chore', 'build', 'ci', 'test', 'style', 'revert'] %}
### 🛠️ Maintenance & Dependencies
{%- else %}
### 📦 {{ type | title }}
{%- endif %}
{% for c in type_commits if not c.is_breaking -%}
- {{ c.line }}
{% endfor %}
{%- endfor %}

{%- if contributors %}
### 👥 Contributors
{% for author in contributors -%}
- @{{ author }}
{% endfor %}
{%- endif %}

---
{%- if compare_url %}
**Full Changelog**: {{ compare_url }}
{%- endif %}
{%- if env("GITHUB_RUN_NUMBER") %} • *CI Build #{{ env("GITHUB_RUN_NUMBER") }}*{%- endif %}
```

### Template Context & Filters

| Variable / Filter | Description | Example |
| :--- | :--- | :--- |
| `tag`, `version` | Target tag name and SemVer string | `v0.10.0`, `0.10.0` |
| `major`, `minor`, `patch` | Numeric SemVer components | `0`, `10`, `0` |
| `year`, `month`, `day` | Numeric date components | `2026`, `10`, `4` |
| `compare_url` | Computed GitHub/GitLab tag comparison link | `https://github.com/.../compare/v0.9.1...v0.10.0` |
| `contributors` | Deduplicated list of commit authors | `["Row0902"]` |
| `commits` | List of enriched commit objects (`type`, `scope`, `description`, `body`, `breaking_description`, `hash`, `short_hash`, `pr_number`, `pr_url`, `line`) | `for c in commits` |
| `group_by_type` | Filter grouping commits by type | `commits \| group_by_type` |
| `group_by_scope` | Filter grouping commits by component/scope | `commits \| group_by_scope` |
| `env(name)` | Global function reading environment variables | `env("GITHUB_RUN_NUMBER")` |

---

## Extracting Release Notes in CI/CD

Extract changelog bodies cleanly for release descriptions, Slack notifications, or webhook payloads:

```bash
# Extract the latest release notes body
cutver changelog latest

# Extract a specific historical release with full header
cutver changelog show v0.9.1 --include-header

# Render through a custom MiniJinja template on the fly
cutver changelog latest --template .github/templates/cutver/RELEASE.md

# Open changelog or compare view directly in the browser
cutver changelog open
```

<p align="center">
  <img src="assets/cutver_changelog_latest_showcase.png" alt="cutver changelog latest formatted output" width="850">
</p>

---

## Official GitHub Actions (CI/CD)

Integrate `cutver` into your GitHub workflows with zero boilerplate:

### `cutver/setup` & `cutver/release`

```yaml
name: Release

on:
  push:
    branches: [main]

permissions:
  contents: write
  id-token: write

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
        with:
          fetch-depth: 0

      - name: Setup Cutver CLI
        uses: cutver/setup@v1
        with:
          github-token: ${{ secrets.GITHUB_TOKEN }}

      - name: Run Cutver Release
        uses: cutver/release@v1
        with:
          command: "release"
          bump: "auto"
          template: ".github/templates/cutver/RELEASE.md"
```

- **[`cutver/setup@v1`](https://github.com/cutver/setup)**: Installs the official, Cosign-verified `cutver` binary matching the runner platform into `$PATH`.
- **[`cutver/release@v1`](https://github.com/cutver/release)**: Executes the release lifecycle, runs preflight verification, performs automated SemVer deduction, and outputs generated release notes.

---

## AI Coding Agent Skills

Equip your AI coding assistants (Pi, Claude Code, Cursor, GitHub Copilot, Codex) with official release engineering skills from **[`cutver/skills`](https://github.com/cutver/skills)**:

```bash
# npm
npx skills add cutver/skills

# bun
bunx skills add cutver/skills

# pnpm
pnpm dlx skills add cutver/skills
```

| Skill | Description | Triggers |
| :--- | :--- | :--- |
| **`cutver-release`** | Safe SemVer version bumping with mandatory `--dry-run` simulation before mutating disk. | `release`, `cut release`, `bump version`, `cutver bump` |
| **`cutver-doctor`** | Diagnostic preflight checks for manifest version drift and changelog consistency. | `doctor`, `cutver doctor`, `verify manifests` |
| **`cutver-init`** | Intelligent workspace onboarding and polyglot manifest autodiscovery. | `init`, `cutver init`, `setup cutver` |
| **`cutver-changelog`** | Extracts release notes for CI/CD, webhooks, or dynamic MiniJinja templates. | `changelog`, `cutver changelog`, `changelog latest` |

> **Agent Safety Invariant**: Cutver skills enforce a mandatory `--dry-run` golden rule. Agents simulate version calculations and preview manifest diffs before touching files or git tags.

---

## Supported Manifest Ecosystems

- **Rust / Cargo (`cargo-package` / `toml`)**: Preserves TOML comments, structure, and formatting via `toml_edit`.
- **Node.js / Web (`json`)**: Modifies **only** the byte-span of the version value. Key order, tabs, spacing, and trailing newlines are 100% preserved.
- **Python (`pyproject`)**: Native support for PEP 621 (`[project] version`) and Poetry (`[tool.poetry] version`), preserving comments and structure.
- **Android / Kotlin (`gradle`)**: Replaces `versionName` with target SemVer and increments `versionCode` integer on every release.
- **Custom / Universal (`regex`)**: Escape hatch for version strings anywhere (e.g. `version.txt`, Dockerfiles, documentation).

### Automatic Lockfile Detection

When `post_bump` lifecycle hooks run (such as `cargo check`, `npm install`, or `uv lock`), `cutver` automatically inspects and stages modified lockfiles:
- **Rust**: `Cargo.lock`
- **JavaScript / TypeScript**: `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lock`
- **Python**: `uv.lock`, `poetry.lock`, `Pipfile.lock`

---

## How It Compares

| Feature | `cutver` | `semantic-release` | `cargo-release` | `changesets` |
| :--- | :---: | :---: | :---: | :---: |
| **Runtime Dependencies** | **None** (Native binary) | Node.js + plugins | Rust toolchain | Node.js |
| **Polyglot / Multi-language** | **Yes** | Ecosystem plugins | Rust only | JS / TS only |
| **Format-Preserving (Comments/Order)** | **Yes** | Varies | Partial | Partial |
| **Two-Phase Atomic Rollback** | **Yes** (RAII transaction) | No | Partial | No |
| **Preflight Process-Tree Kill** | **Yes** | No | No | No |
| **Automatic Lockfile Staging** | **Yes** (Cargo, Bun, UV, Pnpm, etc.) | Varies | Yes (Cargo only) | Yes (NPM only) |
| **Dynamic Templating** | **MiniJinja** (`group_by_type`, `group_by_scope`) | Plugin templates | Limited | Limited |
| **Terminal UX & Accessibility** | **Yes** (OSC 8, footnotes, NVDA safe) | No | No | No |
| **Browser Dispatch (`open`)** | **Yes** (macOS, Windows, Linux, WSL) | No | No | No |
| **First-Party GitHub Actions** | **`cutver/setup`, `cutver/release`** | Actions available | None | Action available |
| **AI Coding Agent Skills** | **Yes (`cutver/skills`)** | None | None | None |
| **Native Floating Major Tags** | **Yes (`v1`, `v2`)** | Plugin / Script | Script | Script |
| **Single Declarative Config** | **`cutver.toml`** | Multiple files/plugins | `Cargo.toml` | `.changeset/` |

---

## Documentation

- **Complete Technical Reference**: [`docs/references.md`](docs/references.md) — Exhaustive specification for `cutver.toml`, all manifest options, preflight timeouts, lifecycle hooks, and CLI arguments.
- **Architecture & Internals**: [`docs/design.md`](docs/design.md) — Two-phase release pipeline, atomic byte-span scanner, and failure rollback guarantees.
- **Official GitHub Actions**:
  - [`cutver/setup`](https://github.com/cutver/setup) — GitHub Action to install and cache Cutver CLI.
  - [`cutver/release`](https://github.com/cutver/release) — GitHub Action to run Cutver releases in CI/CD.
- **AI Agent Skills**: [`cutver/skills`](https://github.com/cutver/skills) — Autonomous release engineering skills for Pi, Claude Code, Cursor, and GitHub Copilot.
- **Changelog**: [`CHANGELOG.md`](CHANGELOG.md) — Release notes and version history.

---

## License

MIT © [Cutver Authors](https://github.com/cutver/cutver) & [Row0902](https://github.com/Row0902)
