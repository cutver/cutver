# Installing cutver

Pick the build that fits your workflow, then verify it in one command.

Two builds ship for every platform:

- **Recommended: `cutver-plugins` (~5 MB)** — adds the full WebAssembly plugin runtime (`cutver plugin add/remove/list`).
- **Lean build: `cutver` (~1.25 MB)** — the core engine alone, for minimal, zero-overhead CI runners.

Every published artifact is cosign-signed and listed on the [releases page](https://github.com/cutver/cutver/releases).

---

## Windows (Scoop)

```bash
scoop bucket add row https://github.com/Row0902/scoop-bucket

# Pick one:
scoop install cutver            # Lean (~1.25 MB)
scoop install cutver-plugins    # Plugin runtime (~5 MB)
```

---

## macOS and Linux (Homebrew)

```bash
# Pick one:
brew install Row0902/tap/cutver            # Lean (~1.25 MB)
brew install Row0902/tap/cutver-plugins    # Plugin runtime (~5 MB)
```

---

## Rust Cargo

```bash
# Lean build
cargo install cutver

# With WebAssembly plugin runtime
cargo install cutver --features plugins
```

---

## OCI Containers (Docker / Podman)

```bash
# Lean image
docker pull ghcr.io/cutver/cutver:latest

# With plugin runtime
docker pull ghcr.io/cutver/cutver:plugins
```

The `:latest` tag follows `main` as well as release tags, so it can be ahead of the latest stable
release; pin an explicit version tag when that matters. Mount your workspace and run any command
inside the image — for example:

```bash
docker run --rm -v "$PWD:/workspace" ghcr.io/cutver/cutver:latest doctor
```

---

## Verify the Install

```bash
cutver --version
cutver doctor
```

A lean build that is asked for a plugin command fails with `PluginManagerRequired`. Install the
plugin build instead of looking for a bypass flag — there is none.

---

## Related Documentation

- [Getting Started Walkthrough](getting-started.md): the first release end to end.
- [Container Integration Guide](../integrations/containers.md): Podman, Docker, `wslc`, and GitLab CI.
- [Installing and Managing Plugins](../reference/plugins.md): lean versus plugin builds, digests, and cache location.
