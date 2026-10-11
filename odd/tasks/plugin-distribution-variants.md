# ODD feature: distribute the plugin variant through the tap and the bucket (#169)

> Issue: [cutver/cutver#169](https://github.com/cutver/cutver/issues/169) · Branch:
> `feat/plugin-distribution-variants` · Base: `main` @ `417b1b8`

## Goal

The release publishes both artifact sets, but the two package-manager channels — the Homebrew tap and
the Scoop bucket — carry only the lean build. A developer on macOS or Windows installs `cutver`, reads
the plugin docs, runs `cutver plugin add <name>`, and hits a binary that cannot load plugins, whose
remedy is a Rust toolchain. This slice gives that user a path: a sibling formula and a sibling manifest
that install the same `cutver` binary, plugin surface included.

## Why

- **Decision A says the lean build is the default** so nobody pays 4× for a feature they do not use.
  This slice refines that per channel: lean where size and surface are load-bearing (the default
  container image, CI downloads, `cargo install`), and *available* where the channel is a developer
  convenience — with the lean still the default there, and the plugin build one explicit choice away.
- **The binary name stays `cutver` in both variants**, so every command in the docs keeps working; only
  the *package* is named differently.
- The measured difference is ~5 MB compressed against ~1.25 MB lean, plus the wasmtime/extism stack.
  That is the honest difference the docs must state.

## Design (parent-owned)

- **D1 — A sibling formula and manifest, same binary name.** `Formula/cutver-plugins.rb` and
  `bucket/cutver-plugins.json` point at the `cutver-plugins-<version>-<slug>` archives and install or
  shim the binary as `cutver`. The lean formula and manifest are untouched.
- **D2 — Installing both is refused explicitly where the manager can.** The plugin formula declares
  `conflicts_with "cutver"`, so Homebrew errors clearly instead of clashing on `bin/cutver`. Scoop has
  no conflicts field, so its manifest carries a `notes` line: install one, and `scoop reset
  cutver-plugins` if both were installed.
- **D3 — The plugin formula asserts its own plugin surface.** Its `test do` runs `cutver plugin --help`
  in addition to `--version`, so a formula that silently lost the feature fails its own audit — the
  same property the release smoke asserts for the artifacts.
- **D4 — The automation mirrors the existing one and touches nothing proven.** The `publish` job gains
  the second formula and manifest writes; the existing writes, the greps, the cosign flow and every
  build/smoke job stay byte-identical. The new greps must select the `cutver-plugins-*` lines and must
  never select a lean one.
- **D5 — The docs say there are two and how to choose.** The README install section lists both formulae
  and both manifests, says to pick one, and states the difference. The plugin reference gains the
  package-manager rows so a plugin user finds the path from the plugin docs.
- **D6 — The residual is stated, not hidden.** The tap and bucket writes cannot be rehearsed without a
  real release; what *is* verifiable now is the template logic against the published v0.13.0 assets —
  the URLs exist and their SHA-256 match the signed `SHA256SUMS`.

## Acceptance

- [ ] The plugin formula is generated with the four plugin archive URLs and their SHA-256 read from
      `dist/SHA256SUMS`.
- [ ] The plugin Scoop manifest is generated with the two plugin Windows zips.
- [ ] Both install or shim the binary as `cutver`.
- [ ] The plugin formula declares `conflicts_with "cutver"` and its test asserts the plugin surface.
- [ ] The Scoop manifest carries the "install one" note.
- [ ] The new greps select plugin lines and never a lean one (falsified against a simulated
      `SHA256SUMS`).
- [ ] The existing lean writes and every proven job are canonically unchanged.
- [ ] The README explains the two variants, that you pick one, and the difference.
- [ ] The plugin reference names the package-manager paths for plugin users.
- [ ] The generated URLs and hashes match the real v0.13.0 release assets.

## Out of scope

Changing the lean formula or manifest; a versioned `cutver@plugins` formula; changing the release
build/smoke/publish ordering or the artifact sets; `cargo-binstall` support.

## Evidence

| Date | Slice | Evidence |
| --- | --- | --- |
| 2026-10-11 | design | `release.yml`'s `publish` job already carries the two write blocks (the formula heredoc and the Scoop JSON edit); the tap holds exactly one formula (`cutver.rb`) and the bucket exactly one `cutver.json`; the README's Quick Install section is the install surface; the v0.13.0 release published 8 lean + 8 plugin archives, and the signed `SHA256SUMS` covers both sets (8 `cutver-plugins-*` lines, verified). |
