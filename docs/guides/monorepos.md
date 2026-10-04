# Monorepo and Multi-Package Strategies

Coordinate multi-package and polyglot repositories without version drift or messy diffs.

Monorepos combine multiple ecosystems: Rust crates, Node/TypeScript apps, Tauri desktop packages, mobile Gradle builds, and shared documentation. `cutver` manages all targets concurrently from a single `cutver.toml`.

---

## Workspace Modes

`cutver` supports two monorepo release models configured under `[workspace]`:

| Mode | Configuration | Release Behavior | Best Suited For |
| :--- | :--- | :--- | :--- |
| **Unified** | `mode = "unified"` | All packages share one synchronized version number. A single bump updates every declared manifest simultaneously with one shared tag. | Applications with paired frontend and backend, unified SDKs, tools with embedded UIs. |
| **Independent** | `mode = "independent"` | Each package tracks its own lifecycle and version. Commits are filtered by path, generating package-prefixed tags (e.g. `core/v1.2.0`). | Multi-package library collections, microservices, mono-repos with independent release cycles. |

---

## Polyglot Unified Workspace

Declare all target manifests under `[[manifest]]`. By convention, the first declared manifest serves as the primary source of truth, or you can flag one with `primary = true`.

```toml
# cutver.toml
[version]
strategy = "conventional"

[workspace]
mode = "unified"
members = ["crates/*", "packages/*"]

# Primary manifest: source of truth for workspace version
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
primary = true

# Secondary manifests: synchronized simultaneously
[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "apps/web/package.json"
kind = "json"
field = "version"

[[manifest]]
path = "src-tauri/tauri.conf.json"
kind = "json"
field = "version"

[[manifest]]
path = "android/app/build.gradle.kts"
kind = "gradle"
version_name_field = "versionName"
version_code_field = "versionCode"

[preflight]
default_timeout = 600
cargo_test = "cargo test --workspace"
web_test   = "bun run test --filter=web"
build      = "cargo build --workspace --release"

[hooks]
post_bump = "cargo check --workspace && bun install"

[git]
tag_prefix = "v"
commit_message = "chore(release): v{version}"
require_clean_tree = true
require_branch = "main"

[publish]
push = true
```

---

## Drift Detection across Monorepo Packages

In polyglot setups, manual releases frequently leave behind forgotten files. `cutver doctor` evaluates all declared manifests across your repository in parallel:

```bash
cutver doctor
```

```text
Evaluating manifests:
  ✓ Cargo.toml (0.10.0) [primary]
  ✓ package.json (0.10.0)
  ✓ apps/web/package.json (0.10.0)
  ✓ src-tauri/tauri.conf.json (0.10.0)
  ✓ android/app/build.gradle.kts (versionName: 0.10.0, versionCode: 10)

Result: All 5 manifests are synchronized.
```

If any file falls out of sync, `cutver doctor` reports the mismatch and exits with code `2`.

---

## Lockfile Coordination in Monorepos

When package manifests change, their lockfiles require corresponding updates. Running packager tools can introduce untracked file noise.

`cutver` solves this with declarative `[hooks] post_bump` and an automatic lockfile staging filter:

```toml
[hooks]
post_bump = "cargo check --workspace && pnpm install --lockfile-only"
```

### Safety Guarantee

`cutver` strictly filters files modified during `post_bump`. Only recognized lockfile paths are staged:
- `Cargo.lock`
- `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lock`, `bun.lockb`
- `uv.lock`, `poetry.lock`, `Pipfile.lock`, `pdm.lock`
- `gradle.lockfile`

Any other file modified by hook scripts remains unstaged in your working tree, guaranteeing that release commits remain surgical and clean.

---

## Adding New Packages

When onboarding a new subpackage to a monorepo, let `cutver init` discover and register it:

```bash
cutver init --update
```

`cutver` scans your project tree, discovers new package manifests, and appends them to `cutver.toml` while preserving all existing preflight checks, hooks, and git settings.

---

## Related Documentation

- [Getting Started with cutver](getting-started.md): Standard release workflow and dry-run walkthrough.
- [Language Manifests and Surgical Diff Guarantees](../reference/manifests.md): Detailed behavior for Cargo, JSON, PyProject, Gradle, and Regex.
- [Complete cutver.toml Configuration Reference](../reference/configuration.md): Exhaustive table and key definitions.
