# Installing cutver

One name, one build — then verify it in one command.

A package manager gives you `cutver` with the WebAssembly runtime already linked in, so
`cutver plugin add/remove/list` works out of the box. There is no second name to choose between: the
plugin build is the functional superset, and a package manager is the "just make it work" channel.

A **lean build** — the core engine alone, no WebAssembly runtime linked in at all, materially smaller —
still ships as a release asset on the [releases page](https://github.com/cutver/cutver/releases), for
minimal CI runners that download their binary directly. Package managers do not install it, and
`cargo install cutver` is the lean build too.

Sizes are not quoted here because they are per-target and change every release: the release workflow
measures every archive it uploads and writes the bytes and MiB of each one into that run's job summary.
Every published artifact is cosign-signed and listed on the
[releases page](https://github.com/cutver/cutver/releases).

---

## Windows (Scoop)

```bash
scoop bucket add row https://github.com/Row0902/scoop-bucket
scoop install cutver
```

---

## macOS and Linux (Homebrew)

```bash
brew install Row0902/tap/cutver
```

`cutver-plugins` still resolves from both managers for one release and is deprecated: it installs this
same build. Install `cutver` instead; the shim goes away in the next release.

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

The `:latest` tag follows `main` and stable release tags, so it can be ahead of the latest stable
release; a prerelease image carries only its own version and moves neither `latest` nor a major tag, so
pin an explicit version tag when that matters. Mount your workspace and run any command
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

A lean build — the `cutver-<version>-<slug>` release archive, or a `cargo install cutver` — cannot run
a `cutver plugin <subcommand>`: it prints that this build was compiled without the `plugins` feature,
names the fix, and exits `1`. (`cutver plugin --help` still prints help.) Install a packaged build
instead of looking for a bypass flag — there is none. A `kind = "plugin"` manifest is a different
failure, named in [the manifest reference](../reference/manifests.md).

---

## Related Documentation

- [Getting Started Walkthrough](getting-started.md): the first release end to end.
- [Container Integration Guide](../integrations/containers.md): Podman, Docker, `wslc`, and GitLab CI.
- [Installing and Managing Plugins](../reference/plugins.md): one package name, the lean build, digests, and cache location.
