# ODD feature: `cutver plugin add/remove/list` (#169)

> Issue: [cutver/cutver#169](https://github.com/cutver/cutver/issues/169) · Branch: `feat/plugin-install-cli` · Base: `main` @ `5d4887c`

## Goal

Make the sentence in the registry README true: `cutver plugin add <name>` resolves, downloads,
verifies, installs and configures a plugin from the `cutver/registry` index. Plus `remove` and a
read-only `list` that reports effective state.

## Why

The engine, the drivers, the lifecycle wiring, the PDK and the first indexed plugin all shipped.
Nothing consumes the index: installing a plugin today means hand-copying a `.wasm` into
`~/.cache/cutver/plugins/` and hand-writing the `[plugins.<name>]` table. The released binary cannot
even execute a wasm plugin (`release.yml` never passes `--features plugins`).

## Decisions (user-authorized 2026-10-10)

| ID | Decision | Consequence |
| --- | --- | --- |
| **A** | **Two artifacts, lean by default.** `cutver` stays lean (1.25 MB compressed) in every channel; a second `cutver-plugins` artifact carries the plugin surface, plus a `:plugins` container tag. | The plugin surface must be usable by real users, but nobody pays 4× for a feature they do not use. The artifact work itself is phase 2 (see Slice 5); #169 ships the CLI. |
| **B** | **`ureq` + rustls**, as an optional dependency behind `plugins`. | Blocking API, so no async runtime enters a synchronous codebase; rustls keeps the Alpine image and the musl matrix free of a system OpenSSL. **Zero new crates**: `ureq 3.4.2` and `rustls` are already in `Cargo.lock` as direct dependencies of `extism 1.30`. Downloads get a timeout and a size cap, and fail closed. |

## Parent-owned design

- **D1 — Cache root is a newtype resolved once at the boundary.** `PluginCacheRoot` replaces the
  `PluginCacheDir`-shaped helper in the driver. Resolution order: `CUTVER_PLUGIN_CACHE_DIR`
  (explicit, used by hermetic tests) → `$HOME` → `%USERPROFILE%`. The silent `.` fallback at
  `src/plugin/driver/wasm.rs:169` is deleted and becomes an actionable fail-closed error naming the
  variable to set. Layout stays `~/.cache/cutver/plugins` for compatibility.
- **D2 — Domain logic stays pure.** Registry entry parsing, release selection, digest-shape
  validation and the `cutver.toml` edit are pure functions in `src/plugin/registry.rs` and
  `src/plugin/install.rs`. HTTP and filesystem live only in the boundary that calls them
  (Pillar I.1).
- **D3 — Install is a two-phase mutation with an RAII guard.** Compute validates everything in
  memory and produces the exact target bytes plus the `toml_edit` edit (Pillar I.2). Apply streams
  the artifact into a temp file **inside the target directory** (same filesystem → atomic move),
  verifies SHA-256, moves it into place, then writes `cutver.toml` through
  `crate::atomic::write_atomic`. A drop guard removes the installed artifact if the config write
  fails, so the config can never point at a missing file and a failure can never leave a stray one.
- **D4 — No path may skip digest verification.** No `--insecure`, no "continue anyway", no fallback
  to whatever the URL returned. TLS is transport, not trust. The digest written to the config is
  re-verified by the loader on every load.
- **D5 — `--dry-run` mutates nothing.** It stops after compute and prints resolved version, URL,
  digest and the exact config diff — the `bump --dry-run` idiom.
- **D6 — `manager.rs` must not grow** (Pillar V.4, already over target per #161). All new code lives
  in new modules under the existing 400-line ceiling.
- **D7 — Work stays strictly sequential.** One cache file, one config file, single-threaded
  (Pillar I.3).
- **D8 — Slices map 1:1 to work-unit commits and to review-sized PRs** (Pillar IV.4). Each PR stays
  inside the 300–400 effective-line budget.
- **D9 — The registry index base stays `https`-only, on purpose.** The index is what *distributes the
  digest*, so TLS on the index is load-bearing trust, not merely transport: an http index would let a
  network attacker supply both the entry and a matching digest, defeating the whole model. Consequence:
  a loopback HTTP fixture cannot exercise the real transport, so the I/O is injected instead (D10).
  This is a deliberate, recorded deviation from the issue's "local HTTP fixture" wording; the intent
  behind it (hermetic tests, no network) is preserved and improved.
- **D10 — Transport injection.** The install path takes an injectable transport (the production `ureq`
  implementation plus a fixture implementation), so `add`/`remove`/`list`, the two-phase apply, the
  digest gate and the RAII rollback are all exercised hermetically with `tempfile::tempdir()` and zero
  network. The thin `ureq` call remains compile-checked rather than behaviourally proven end to end,
  and that residual is stated rather than hidden.
- **D11 — Hardening absorbed into slice 3**: `HttpsUrl::parse` must reject a backslash anywhere in the
  URL (Pillar II.3) and a malformed or out-of-range port. Found by the slice-2 verifier, which accepted
  `https://github.com/a\b.wasm`, `https://github.com:notaport/x.wasm` and `https://.com/x.wasm`.
  Slice 2 was committed exactly as verified rather than mutated after its verification.

## Slices

| # | Slice | Deliverable | State |
| --- | --- | --- | --- |
| 1 | **Cache root** | `PluginCacheRoot` newtype + fail-closed resolution + `CUTVER_PLUGIN_CACHE_DIR` override; driver consumes it; lean and `plugins` builds green. (The `--features plugins` CI job already landed in #171.) | **done** — `d42b625` |
| 2 | **Registry client** | `src/plugin/registry.rs`: typed entry, release selection (highest satisfying **stable** semver unless `--version` asks for a pre-release), digest-shape and capability validation, fetch with timeout + size cap. Fixture tests, no network. | **done** — `0f96632` |
| 3 | **`add` / `remove` / `list`** | CLI args, runner dispatch, two-phase install with the rollback guard, surgical `toml_edit` edits, `--dry-run`, `--json`. Integration tests against a local HTTP fixture and a temp cache root. | pending |
| 4 | **Docs** | Plugin reference: install flow, digest verification, cache location. Also unblocks #164. | pending |
| 5 | **Artifact variant (decision A)** | `release.yml` second artifact `cutver-plugins` + `:plugins` container tag (`--build-arg CUTVER_FEATURES="--features plugins"`, already threaded). **Phase 2 — separate authorization**, because `release.yml` has broken releases twice. | deferred |

## Acceptance (from the issue, unchanged)

- [ ] `cutver plugin add github-releases` installs the published artifact into the cache and writes a
      `[plugins.github-releases]` entry whose `hash` matches the registry `sha256`.
- [ ] The installed artifact is byte-identical to the release asset and loads under `--features plugins`.
- [ ] A tampered artifact, a digest mismatch, an unknown capability, a malformed URL and an
      unreachable registry each fail closed with what/where/how, leaving no cache file and no config change.
- [ ] No flag and no code path can skip digest verification.
- [ ] `--dry-run` performs no I/O mutation and prints the resolved version, URL, digest and config diff.
- [ ] `remove` deletes only that table and only an unreferenced artifact; an entry sharing the `source` keeps its file.
- [ ] `list` reports declared versus installed versus digest-matched state and mutates nothing.
- [ ] A failed config write leaves neither an installed artifact nor a modified `cutver.toml`.
- [ ] The cache root resolves on Linux, macOS and Windows, and the integration suite is hermetic under `tempfile::tempdir()`.
- [ ] CI builds, tests and lints with `--features plugins`, and the wasm suite runs.
- [ ] `cargo fmt -- --check` and `cargo clippy --workspace --all-targets --features plugins -- -D warnings` are clean.
- [ ] The plugin reference documents install, digest verification and the cache location.

## Evidence

| Date | Slice | Evidence |
| --- | --- | --- |
| 2026-10-10 | — | Decisions A/B authorized; branch `feat/plugin-install-cli` created off `main` @ `5d4887c`; `test-plugins` CI job (#171) and the `Containerfile` feature knob (#175) already on `main`. |
| 2026-10-10 | 1 | Commit `d42b625` — `feat(plugin): fail closed on an unresolvable plugin cache root`. New `src/plugin/cache.rs` (210 lines, 11 unit tests), `UnresolvableCacheRoot` in `src/plugin/error.rs`, driver net −5 lines, `manager.rs` byte-identical (D6). RED observed before GREEN: with the legacy `"."` fallback reinstated, 4 cache tests fail and the root printed as `./.cache/cutver/plugins`. Independent verification PASS: `fmt` + `clippy` clean in both feature configurations, `cargo test --lib` 346 passed / `--features plugins` 351 passed, plus a falsification run with `HOME`, `USERPROFILE` and the override scrubbed from the process that confirmed a fail-closed error and no working-directory root. Design change during verification: `CUTVER_PLUGIN_CACHE_DIR` is used **verbatim** as the cache directory (the error text already promised this); only the home branches append `.cache/cutver/plugins`. |
| 2026-10-10 | 2 | Commit `0f96632` — `feat(plugin): add a typed registry client behind the plugins feature`. Facade `src/plugin/registry.rs` (34 lines) over `digest` (60), `entry` (206), `fetch` (82), `release` (81), `tier` (63), `url` (115); 12 new `Registry*` variants in `src/plugin/error.rs` (286 lines, every one with `Where:` and `Fix:`); `tests/plugin_registry.rs` (21 tests) plus 11 fixtures. `manager.rs` and `driver/**` untouched. `ureq` added as an optional dep: `[[package]]` count 329 → 329, the only lock delta being cutver's own `"ureq"` line, and the lean tree contains neither `ureq` nor `extism`. Independent verification PASS: gates clean in both feature configurations (351 lib + 21 registry tests), falsification from a scratch crate outside the repo reproduced digest, URL, selection, name-mismatch and unknown-capability behaviour, and found D11 (a backslash and malformed ports survive `HttpsUrl::parse`) as an accepted-but-should-not input, carried into slice 3. |

## Out of scope

Registry schema extension with `events`/`manifest_match` (follow-up in `cutver/registry`),
`plugin update`, pinning policy, a config-pinned internal registry, `doctor` plugin health, a
registry publishing path, signing, and any trust root beyond the pinned digest.
