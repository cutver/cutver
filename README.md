<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/cutver-mark-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/cutver-mark-light.svg">
  <img alt="Cutver logo: a geometric C monogram combined with an emerald green verified release checkmark, separated by an atomic 45-degree cut" src="assets/cutver-mark-dark.svg" width="84" height="84" />
</picture>

# cutver

A release tool for repositories where the version lives in more than one file. It bumps every manifest you
declare, runs your preflights, writes the changelog, commits, tags and publishes — from one `cutver.toml`, in
any language.

**And it stops before the tag if you ask it to.** `cutver bump --defer-publish` writes the release commit and
stops before the tag. You read the notes, rewrite them, then `cutver publish` creates the tag and publishes it.
The part that goes out to users is the part you had a chance to fix.

[![CI](https://github.com/cutver/cutver/actions/workflows/ci.yml/badge.svg)](https://github.com/cutver/cutver/actions/workflows/ci.yml)
[![Release](https://github.com/cutver/cutver/actions/workflows/release.yml/badge.svg)](https://github.com/cutver/cutver/actions/workflows/release.yml)
[![GitHub Release](https://img.shields.io/github/v/release/cutver/cutver?logo=github&color=blue&style=flat-square)](https://github.com/cutver/cutver/releases)
[![released with: cutver](https://img.shields.io/badge/released%20with-cutver-0969da?style=flat-square&logo=git&logoColor=white)](https://github.com/cutver/cutver)
[![Cosign Signed](https://img.shields.io/badge/cosign-signed_binaries-blue?style=flat-square&logo=sigstore)](https://github.com/cutver/cutver/releases)
[![Crates.io](https://img.shields.io/crates/v/cutver.svg?logo=rust&color=orange&style=flat-square)](https://crates.io/crates/cutver)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg?style=flat-square)](LICENSE)

<p align="center">
  <a href="#the-two-phase-release">Two-phase release</a> •
  <a href="#plugins">Plugins</a> •
  <a href="#installation">Install</a> •
  <a href="#documentation">Docs</a>
</p>

<p align="center"><sub><a href="https://github.com/user-attachments/assets/5567bfea-a752-4ced-96ee-7fd793ce2562">57-second film</a> of four release commands and a dry run • <a href="docs/accessibility.md">transcript and audio description</a></sub></p>

</div>

---

## The two-phase release

One command cuts the release, a second one publishes it, and everything between them is yours. That gap is
where the release notes stop being output and start being writing.

```bash
cutver plan                          # what the next release would be; writes, commits and pushes nothing
cutver bump auto --defer-publish     # preflights, manifests, changelog, release commit — no tag
git show HEAD:CHANGELOG.md           # the notes cutver rendered, to review
cutver publish --tag v1.3.0 --notes-file - <<'EOF'
## Demo 1.3.0

`demo` now greets you on start. Rewritten by hand before it was published.
EOF
```

Real output from that run in a scratch repository (`…` elides build noise):

```text
$ cutver bump auto --defer-publish
[main c22b42f] chore(release): v1.3.0 [skip ci]
…
✔ Release Plan: 1.2.0 -> 1.3.0 (minor (deduced from 1 feature))
…
│   └── tag: v1.3.0 [DEFERRED]
└── Publish
    ├── push: true
    └── deferred: cutver publish --tag v1.3.0

$ cutver publish --tag v1.3.0 --notes-file -
…
 * [new tag]         v1.3.0 -> v1.3.0
✔ Published: v1.3.0 at c22b42fafb5f3356f02c498f92bbbe6a661be6df
├── Tag
│   └── v1.3.0
└── Push
    └── git push origin refs/tags/v1.3.0
```

`--notes-file -` reads the notes from standard input, so nothing lands in the tree and the clean-tree guard
still applies to everything else. Without `--defer-publish`, the same steps run as the tail of one process:
the split is a choice at cut time, not a different release. `--dry-run` on either command computes the whole
thing and writes nothing. [Release pipeline order and atomic guarantees](docs/internals/pipeline.md) has the
step list and where transaction safety ends.

---

## Plugins

The core implements every stage, so a release needs no plugin at all. Four stages are also typed slots, and a
plugin that declares the slot takes it over.

| Slot | What the plugin decides |
| :--- | :--- |
| `manifest.v1` | Read and write a manifest format the core does not know. |
| `lifecycle.v1` | Run on `on_pre_bump`, `on_post_bump` or `on_post_release`. |
| `changelog.v1` | Render the release notes. |
| `versioning.v1` | Decide the next version from your commits. |

A plugin is a WebAssembly module (`runtime = "wasm"`) or a native process (`runtime = "process"`), and both
send and receive the same versioned JSON envelope, so one capability works in either.

A WASM plugin has no authority to begin with: no filesystem, no network and no environment until
`[plugins.<name>.permissions]` grants them, and it cannot exercise a capability it did not declare. The
process runtime has no sandbox at all; its only grant is `permissions.env`, and `timeout_seconds` bounds one
call.

`cutver plugin add` installs from the registry and checks the digest before the artifact reaches the cache,
then again on every load. The runtime is a Cargo feature: a lean build has no `plugin` subcommand and fails
closed naming the flag. Authoring starts at [Writing a Plugin](docs/guides/plugin-authoring.md); the
[plugin reference](docs/reference/plugins.md) has every `[plugins.<name>]` key, and
[the reference plugin](docs/guides/plugin-github-publish.md) is this repository's own release path.

---

## Installation

Every package manager installs one build: `cutver`, with the WebAssembly runtime and the `plugin` commands.
The lean build — the same tool without the runtime — ships as a release asset for a runner that must stay
small, and `cargo install cutver --features plugins` builds the plugin-enabled one.

| Platform | Install |
| :--- | :--- |
| Windows (Scoop) | `scoop install cutver` |
| macOS / Linux (Homebrew) | `brew install Row0902/tap/cutver` |
| Rust (Cargo) | `cargo install cutver --features plugins` |
| Containers | `docker pull ghcr.io/cutver/cutver:plugins` |

`cutver-plugins` still resolves for one release and then goes away. The
[Installation Guide](docs/guides/installation.md) covers every package manager, the lean releases, the bucket
and tap setup, and how to verify an install. Once it is installed, `cutver init` discovers your manifests and
writes `cutver.toml`, the starter changelog and the release template; [Getting Started](docs/guides/getting-started.md)
walks the first release from there.

---

## Documentation

**Start here** — [Getting Started](docs/guides/getting-started.md) · [Visual Tour](docs/guides/visual-tour.md) · [Frequently Asked Questions](docs/guides/faq.md) · [Accessibility](docs/accessibility.md)

**Configuration** — [cutver.toml Reference](docs/reference/configuration.md) · [Language Manifests & Diff Guarantees](docs/reference/manifests.md) · [Release Notes Templates](docs/templates/release-notes.md)

**CLI** — [Commands & Exit Codes](docs/reference/cli.md) · [Machine Contract](docs/internals/machine-contract.md)

**Plugins** — [Plugin Overview](docs/guides/plugin-overview.md) · [Writing a Plugin](docs/guides/plugin-authoring.md) · [The Process Runtime](docs/guides/plugin-process-runtime.md) · [Installing & Managing Plugins](docs/reference/plugins.md) · [The Reference Plugin](docs/guides/plugin-github-publish.md) · [Plugin Engine Architecture](docs/rfcs/0001-wasm-plugin-architecture.md)

**Repositories** — [Monorepos & Multi-Package Strategies](docs/guides/monorepos.md) · [Installation Guide](docs/guides/installation.md)

**CI** — [GitHub Actions](docs/integrations/github-actions.md) · [Production Release Workflow](docs/integrations/production-release-workflow.md) · [Migrating from the release action](docs/integrations/migrating-from-the-release-action.md) · [Containers](docs/integrations/containers.md) · [AI Coding Skills](docs/integrations/agent-skills.md)

**Internals** — [Architecture & Design Decisions](docs/internals/design.md) · [Pipeline Execution & Atomic Guarantees](docs/internals/pipeline.md) · [The Local Gate & Source Line Budget](docs/internals/local-gate.md)

**Contributing** — [Contributing to cutver](CONTRIBUTING.md) · [Official Insignia](docs/reference/insignia.md)

---

## License

MIT © [Cutver Authors](https://github.com/cutver/cutver) & [Row0902](https://github.com/Row0902)
