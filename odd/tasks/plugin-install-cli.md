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
- **D11 — Hardening absorbed into slice 3a**: `HttpsUrl::parse` must reject a backslash anywhere in the
  URL (Pillar II.3) and a malformed or out-of-range port, while still accepting what it accepted before
  — over-rejection is as much a defect as under-rejection. Found by the slice-2 verifier, which accepted
  `https://github.com/a\b.wasm`, `https://github.com:notaport/x.wasm` and `https://.com/x.wasm`.
  Slice 2 was committed exactly as verified rather than mutated after its verification.
- **D13 — Reading and deleting are different questions.** The artifact rule ("an existing local file
  wins, otherwise the artifact lives inside the cache root") answers *where a load reads from*, and
  lives in exactly one canonical, lazy function (`cache::read_path`) used by `list`, `remove` and the
  loader. Deletion is a separate, containment-checked intent: `remove` may only delete inside the cache
  root, keeps a `source` that is the user's own local file, and keeps anything that escapes the root
  (absolute, or `..` after lexical normalization). Rationale: the first slice-3a attempt reused the
  read path for deletion, and because `PathBuf::join` replaces the base with an absolute argument, a
  `remove` could delete a file outside the cache root and report it as a cached artifact. Lexical
  normalization is deliberate (no `canonicalize`), and deleting a symlink only unlinks the link.

### Slice 3b design (parent-owned, 2026-10-10)

- **D14 — The existing-entry check is local and comes first.** Phase 1 reads `cutver.toml` and fails
  closed on an existing `[plugins.<name>]` entry *before* any network fetch. `--force` is the only
  override, and it overrides only this rule — never the digest gate. Consequence: "already
  configured" is a fast, offline, binary-testable failure.
- **D15 — `source` is `<name>-<version>.wasm`.** An always cache-relative artifact name derived from
  the validated `PluginName` and the selected semver, exactly as issue #169 specifies: two versions
  coexist on disk and `list` can name which one is installed. An absolute path is never written.
- **D16 — `events`/`manifest_match` come from flags and are required when the capability needs them.**
  `lifecycle.v1` requires at least one `--event`, because the manager treats an empty `events` list
  as "matches every event" — a wildcard the registry entry cannot express. `manifest.v1` requires at
  least one `--manifest-match`, because an empty `manifest_match` matches nothing at all.
  `changelog.v1`/`versioning.v1` need neither. Failing closed names the exact missing key and the
  capability that required it.
- **D17 — Transport is an injected trait; the production impl is the only `ureq` caller.**
  `registry/transport.rs` defines `Transport { fn fetch(&self, url, cap_bytes) -> Result<Vec<u8>, PluginError> }`
  plus `UreqTransport` (global timeout, size cap, non-2xx fail-closed). `fetch.rs` gains
  `_with(base, name, transport)` variants while the verified slice-2 API (`entry_url(name)`,
  `fetch_entry(name)`, `resolve_entry(name)`) stays byte-compatible, so no slice-2 test changes.
  Tests inject a fixture transport → no HTTP fixture, no network (D9 preserved).
- **D18 — `--registry <BASE>` is https-only and normalized to exactly one trailing slash.** Validated
  through the existing `HttpsUrl` (an `http://` base fails closed) with a new typed
  `RegistryIndexUrl`; a base missing the slash gets exactly one appended.
- **D19 — Digest verification precedes the final cache location, through one canonical verifier.**
  The downloaded bytes are checked with the existing `driver::wasm::verify_sha256` (Pillar V.6: one
  digest authority, not a second hasher in `install.rs`) *before* the artifact reaches its final
  name. Apply order: create the cache root → temp file inside the target directory → verify SHA-256
  → atomic rename into place → `atomic::write_atomic` the config → RAII guard deletes the installed
  artifact if the config write fails. `--dry-run` stops after phase 1 and performs zero I/O mutation;
  it does not download, because the digest comes from the registry entry.
- **D20 — The plugin error vocabulary is decomposed, and `Transport` owns its own errors.** Slice 3b's
  seven variants took `src/plugin/error.rs` to 414 total lines (357 production), past Pillar V.4's
  mandatory-decomposition trigger at ~350 and its 400 hard ceiling, so the file became a facade over
  `error/transport.rs` (`TransportError`, 33 lines) and `error/install.rs` (`InstallError`, 53) with
  the pins moved to `error/tests.rs` (54). `PluginError` gained two `#[error(transparent)]` variants,
  so no rendered message changed and every pre-existing variant kept its path. The same review moved
  the `Transport` trait's error type to `TransportError`: the HTTP adapter no longer fabricates domain
  errors (Pillar I.1), `registry/fetch.rs` re-labels them onto the existing `Registry*` variants, and
  `install/apply.rs` wraps them as `PluginError::Transport`. The variant that was named
  `RegistryIndexUrl` is now `InstallError::IndexUrl`; the validated *type* `RegistryIndexUrl` is
  unchanged.
- **D21 — `PluginError`'s size is a cross-module invariant, not a local detail.** `bump::Error` embeds
  `PluginError` by value (`src/bump/types.rs`), so every byte added here is added there, and
  `clippy::result_large_err` fires across `src/bump/exec/**` once `bump::Error` passes 128 bytes — a
  failure that appears in files the change never touched. The pin test
  `plugin_error_stays_within_the_bump_result_budget` fails the build if either type grows past 96/128,
  so a new plugin variant must be compact or boxed, and a relaxed pin requires a deliberate decision.
- **D22 — A pre-existing artifact that cannot be read fails closed.** `read_original` used to degrade a
  read failure to `None`, which let the rollback guard `remove_file` an artifact whose bytes it had
  never backed up: data loss on a `--force` reinstall whose config write fails. It now returns
  `InstallError::ArtifactRead` before the rename, and a Unix-gated regression test pins it. The
  guard's `Drop` stays best-effort and is documented as un-propagatable.
- **D23 — The `[plugins]` parent is implicit when `add` creates it, and no output restores its header.**
  `insert_plugins_table` used `entry("plugins").or_insert(Item::Table(Table::new()))`, which
  materialised an *explicit* `[plugins]` header, and `remove` only deleted the child key, so an
  `add` → `remove` round trip left an empty `[plugins]` behind and never restored the original config.
  Rule 1: when the `plugins` key is absent, `add` inserts it with `set_implicit(true)` so toml_edit
  renders no header of its own; when the key already exists — including a hand-written explicit
  `[plugins]` — it is left exactly as written, and a non-table `plugins` still fails closed. Rule 2:
  an explicit empty table is kept, because the user wrote it, while an implicit empty parent carries no
  user intent and must leave no `[plugins]` behind. The fix lands entirely in `add`: `remove` needs no
  change, because toml_edit renders an empty implicit table as no output, so deleting the child already
  restores the original bytes. That was confirmed empirically — with `remove` at `HEAD`, the round trip
  is byte-identical — and the rule is recorded so a later change cannot reintroduce an explicit parent.
  Found by the live-registry run below, not by the hermetic suite, which asserted on content (the table
  is gone, unrelated bytes stay) and never on whole-file byte identity.

## Slices

| # | Slice | Deliverable | State |
| --- | --- | --- | --- |
| 1 | **Cache root** | `PluginCacheRoot` newtype + fail-closed resolution + `CUTVER_PLUGIN_CACHE_DIR` override; driver consumes it; lean and `plugins` builds green. (The `--features plugins` CI job already landed in #171.) | **done** — `d42b625` |
| 2 | **Registry client** | `src/plugin/registry.rs`: typed entry, release selection (highest satisfying **stable** semver unless `--version` asks for a pre-release), digest-shape and capability validation, fetch with timeout + size cap. Fixture tests, no network. | **done** — `0f96632` |
| 3a | **`list` / `remove`** | CLI scaffolding for the `plugin` subcommand (D12), read-only `list` with three digest states and `--json`, surgical `remove` with the shared-source rule and `--keep-artifact`, artifact deletion containment (D13), and the D11 URL hardening. | **done** — `1203692` |
| 3b | **`add`** | Transport injection (D10/D17), artifact download with a size cap, the digest gate before the final cache location (D19), temp file inside the target directory plus atomic move, the RAII rollback guard, `--version`, `--registry` (D18), `--event`/`--manifest-match` requirement check (D16), `--dry-run`, `--force` (D14), and `source = <name>-<version>.wasm` (D15). | **done** — `988d7fa` |
| 4 | **Docs** | Plugin reference: install flow, digest verification, cache location, and the command surface. Also unblocks #164. | **done** — #183 (landed in `d6085e0`) |
| 5 | **Artifact variant (decision A)** | `release.yml` second artifact `cutver-plugins` + `:plugins` container tag (`--build-arg CUTVER_FEATURES="--features plugins"`, already threaded). **Separate authorization** granted 2026-10-10; the design, the acceptance list and the live state live in `odd/tasks/plugins-release-artifact.md`. | **done** — #185 (`b1aba1d`) |
| 6 | **Implicit `[plugins]` parent** | D23: `add` creates the parent implicitly, an empty implicit parent leaves no `[plugins]` in the output, and a byte-for-byte `add` → `remove` round trip is pinned. | **done** — #184 (`b1681dc`) |

## Acceptance (from the issue, unchanged)

- [x] `cutver plugin add github-releases` installs the published artifact into the cache and writes a
      `[plugins.github-releases]` entry whose `hash` matches the registry `sha256`. *(Verified against
      the **live registry** after merge — see the `live` evidence row: 211,299 bytes downloaded, and the
      stored artifact's `sha256sum` equals both the config `hash` and the registry's declared sha256.
      This replaces the old "not yet" note.)*
- [ ] The installed artifact is byte-identical to the release asset and loads under `--features plugins`.
      *(Byte-identity is now proven, including the live artifact's `sha256sum`. What remains unproven is
      **execution inside a real release pipeline**: no released `cutver-plugins` binary has loaded the
      indexed asset, because decision A's artifact variant is still phase 2.)*
- [x] A tampered artifact, a digest mismatch, an unknown capability, a malformed URL and an
      unreachable registry each fail closed with what/where/how, leaving no cache file and no config change.
- [x] No flag and no code path can skip digest verification.
- [x] `--dry-run` performs no I/O mutation and prints the resolved version, URL, digest and config diff.
- [x] `remove` deletes only that table and only an unreferenced artifact; an entry sharing the `source` keeps its file.
- [x] `list` reports declared versus installed versus digest-matched state and mutates nothing.
- [x] A failed config write leaves neither an installed artifact nor a modified `cutver.toml`.
- [ ] The cache root resolves on Linux, macOS and Windows, and the integration suite is hermetic under `tempfile::tempdir()`.
      *(Linux is exercised for real; `%USERPROFILE%` only through the injected lookup.)*
- [x] CI builds, tests and lints with `--features plugins`, and the wasm suite runs. *(Proved on the PR
      chain: `Test` and `Test with plugins feature` green on ubuntu-latest, macos-latest and
      windows-latest for #179-#182.)*
- [x] `cargo fmt -- --check` and `cargo clippy --workspace --all-targets --features plugins -- -D warnings` are clean.
- [x] The plugin reference documents install, digest verification and the cache location. *(Slice 4:
      `docs/reference/plugins.md`, linked from the README, the CLI reference and the configuration
      reference.)*

## Evidence

| Date | Slice | Evidence |
| --- | --- | --- |
| 2026-10-10 | — | Decisions A/B authorized; branch `feat/plugin-install-cli` created off `main` @ `5d4887c`; `test-plugins` CI job (#171) and the `Containerfile` feature knob (#175) already on `main`. |
| 2026-10-10 | 1 | Commit `d42b625` — `feat(plugin): fail closed on an unresolvable plugin cache root`. New `src/plugin/cache.rs` (210 lines, 11 unit tests), `UnresolvableCacheRoot` in `src/plugin/error.rs`, driver net −5 lines, `manager.rs` byte-identical (D6). RED observed before GREEN: with the legacy `"."` fallback reinstated, 4 cache tests fail and the root printed as `./.cache/cutver/plugins`. Independent verification PASS: `fmt` + `clippy` clean in both feature configurations, `cargo test --lib` 346 passed / `--features plugins` 351 passed, plus a falsification run with `HOME`, `USERPROFILE` and the override scrubbed from the process that confirmed a fail-closed error and no working-directory root. Design change during verification: `CUTVER_PLUGIN_CACHE_DIR` is used **verbatim** as the cache directory (the error text already promised this); only the home branches append `.cache/cutver/plugins`. |
| 2026-10-10 | 2 | Commit `0f96632` — `feat(plugin): add a typed registry client behind the plugins feature`. Facade `src/plugin/registry.rs` (34 lines) over `digest` (60), `entry` (206), `fetch` (82), `release` (81), `tier` (63), `url` (115); 12 new `Registry*` variants in `src/plugin/error.rs` (286 lines, every one with `Where:` and `Fix:`); `tests/plugin_registry.rs` (21 tests) plus 11 fixtures. `manager.rs` and `driver/**` untouched. `ureq` added as an optional dep: `[[package]]` count 329 → 329, the only lock delta being cutver's own `"ureq"` line, and the lean tree contains neither `ureq` nor `extism`. Independent verification PASS: gates clean in both feature configurations (351 lib + 21 registry tests), falsification from a scratch crate outside the repo reproduced digest, URL, selection, name-mismatch and unknown-capability behaviour, and found D11 (a backslash and malformed ports survive `HttpsUrl::parse`) as an accepted-but-should-not input, carried into slice 3. |
| 2026-10-10 | 3a | Commit `1203692` — `feat(plugin): add plugin list and a surgical plugin remove`. New `src/cli/args/plugin.rs` (26), `src/cli/runner/plugin.rs` (229), `src/plugin/manage.rs` (17) over `manage/status.rs` (167) and `manage/remove.rs` (333); 6 new `PluginError` variants; `tests/plugin_manage.rs` (283, 12 tests). The first verification round returned **FAIL** with three defects, all fixed and re-verified **PASS**: (1) `remove` resolved its deletion target with `artifact_path` instead of the canonical rule, so a relative local `source` was reported absent while the file stayed on disk, and an **absolute** `source` made `PathBuf::join` replace the base — `remove` deleted a file outside the cache root; (2) the D11 hardening over-rejected bracketed IPv6 authorities (`https://[::1]/x.wasm`) that it had accepted before, while accepting an unterminated `https://[::1`; (3) `PluginArtifactDelete` had no test at all. Fixes: one canonical lazy `read_path` used by `list`, `remove` and the loader with the duplicated rule removed from the driver; a separate containment-checked delete path (D13); bracketed-IPv6 authority parsing; and a hermetic deletion-failure test using a directory at the artifact path. Gates: fmt and clippy clean in both feature configurations, 355 lib tests (376 with `plugins`), 12 `plugin_manage` + 21 registry tests, `cargo build --locked` clean, lean tree free of `sha2`/`ureq`/`extism`, and the lean binary hides the subcommand and fails closed. |
| 2026-10-10 | 3b | Commit `988d7fa` — `feat(plugin): add \`cutver plugin add\` with a verified digest gate`. New `src/plugin/install.rs` (13) over `install/plan.rs` (229) and `install/apply.rs` (186); `src/plugin/registry/transport.rs` (83) with the injectable `Transport` trait and `UreqTransport`; `src/plugin/error.rs` decomposed into a facade (311 total / 309 production) over `error/transport.rs` (33), `error/install.rs` (53) and `error/tests.rs` (54); `src/cli/args/plugin.rs` (54); `src/cli/runner/plugin/add.rs` (124); `tests/plugin_install.rs` (433, 14 hermetic tests, zero network). Two independent verification rounds. **Round 1**: all six gates exit 0 and all eleven behaviour probes held, but it returned **FAIL** on Pillar V.4 — `error.rs` at 414/357 against HEAD's 332/303 — and found two defects on the way. **Defect 1, with a corrected misdiagnosis**: the writer reported 24 `clippy::result_large_err` errors in `src/bump/exec/**` as pre-existing and "proved" it by re-running clippy with only that lint allowed, which removes the very signal. Falsified against a clean `HEAD` worktree: `PluginError` 96 → 104 bytes and `bump::Error` 128 → 144, because `bump::Error` embeds `PluginError` by value (`src/bump/types.rs:84`) and 128 is exactly clippy's threshold. Fixed by carrying `Capability` plus two `&'static str`s in `PluginFlagRequired` and by a size pin (D21). **Defect 2, data loss**: `read_original` degraded a read failure to `None`, so the rollback guard would delete a pre-existing artifact whose bytes it never backed up; it now fails closed with `InstallError::ArtifactRead` before the rename, and a Unix-gated regression test pins it. The writer had declared that no deterministic trigger existed; that was wrong on this uid-1000 host, so the test was added by the parent and its RED observed against the degrading implementation (D22). **Round 2** (after the D20 decomposition): **PASS** — six gates exit 0, `error.rs` production 309 and every submodule under budget, all 36 pre-existing variants byte-identical in path and message, `tests/plugin_manage.rs` and `tests/plugin_registry.rs` byte-unchanged and passing (12 + 21), `PluginError` 96 / `bump::Error` 128 re-measured from an independent probe crate, and the regression test's power reproduced independently in a scratch copy. |
| 2026-10-10 | chain | Delivered as a **feature-branch chain** on `cutver/cutver` with `Refs #169` (nonclosing while slice 4 was pending): **#179** draft tracker carrying slice 1 (330 changed lines, 5 files) and three stacked children — **#180** slice 2 (1210 lines, 24 files) → **#181** slice 3a (1592, 17) → **#182** slice 3b (1523, 17). Every PR diff was verified clean and shows only its own slice, with no polluted base. One honest slicing pass was attempted (slice 2 pure-validation vs client; slice 3a scaffolding+`list` vs `remove`+D13; slice 3b plan vs apply): every cohesive cut landed at 580-830 lines or fragmented a deliverable that cannot be verified alone, so the 4-slice split stands and the overage is stated in each PR body rather than golfed. No `size:exception` was requested — this repository has no such policy, no `type:*` labels and no PR template, and the previously merged PRs carry zero labels. CI on all four: 7/7 green, `Test` and `Test with plugins feature` on ubuntu-latest, macos-latest and windows-latest plus `Publish dry-run`. The tracker carries slice 1 because GitHub refuses to open a PR with an empty diff, so a tracker sitting on `main` cannot be opened at all. |
| 2026-10-10 | 4 | New `docs/reference/plugins.md` (146 lines, inside the ≤150 target) documenting the feature gate, the quick path, the three commands and their flags, the exact `[plugins.<name>]` keys, the two digest gates, the cache resolution order and a troubleshooting table. Navigation kept honest: the README docs table gained a Reference row, the CLI reference (113 → 132 lines) gained a `### cutver plugin` section because it claims to cover every command and mentioned none, and the configuration reference (147 → 157) gained a `### [plugins.<name>]` pointer because it claims to be the canonical reference for every table and had no plugin table. Every command and flag is transcribed from the built binary's `--help` output rather than from memory, and all relative `.md` links were checked programmatically and resolve. No test-first, and stated as such: this is a passive documentation slice, verified structurally (line budgets, link resolution, and a factual read-back against the binary) rather than by a failing test. |
| 2026-10-10 | live | **Live-registry verification** (parent-run, after #169 merged into `main`). `plugin add github-releases --dry-run` resolved `0.1.0` from `https://raw.githubusercontent.com/cutver/registry/main/plugins/` and printed `https://github.com/cutver/plugins/releases/download/github-releases-v0.1.0/github-releases.wasm` plus the digest `a828cbabd247afdaf78fec358ae5a27271097c7c2ef6fc9ec0fe04af22edf841`, with zero mutation. The real install downloaded **211,299 bytes**, wrote the config entry and stored `<cache>/github-releases-0.1.0.wasm`; `sha256sum` of that file equals both the `hash` in `cutver.toml` and the registry's declared sha256, which is also the constant pinned in `tests/fixtures/registry/github-releases.toml`; `plugin list` reported `installed: yes` and `digest: matched`; `plugin remove` deleted the table and the artifact. The same run exposed D23. |
| 2026-10-10 | 6 | `fix(plugin): keep the [plugins] parent implicit on add and remove`. `insert_plugins_table` now inserts an implicit parent only when the key is absent and leaves an existing one untouched; the drop of an empty implicit parent lives in toml_edit's rendering rather than in `remove`. Three tests in `tests/plugin_install.rs`; two were RED against the unfixed code (the run rendered an explicit `[plugins]` header, and the round trip left that two-line parent behind instead of restoring the original bytes), while the third pins that a hand-written explicit parent survives both operations and already passed pre-fix. All six gates exit 0: `cargo fmt -- --check`, clippy lean and `--features plugins`, `cargo test --features plugins`, `cargo test`, `cargo build --locked`. `src/plugin/manage/remove.rs` was left at `HEAD` after an empirical check: with an explicit drop of the empty implicit parent disabled the round trip is still byte-identical, because toml_edit renders an empty implicit table as no output, so that file did not need a change. |
| 2026-10-10 | follow-ups | Both follow-ups merged into `main`: **#184** (`fix(plugin): keep the [plugins] parent implicit on add and remove`) as `b1681dc`, and **#185** (`ci(release): publish the plugin surface as its own artifact`) as `b1aba1d`, which is `main`'s tip. CI on `main` is green on `d6085e0`, `b1681dc` and `b1aba1d`. #184 is one production file — `set_implicit(true)` in `src/plugin/install/plan.rs` when the `plugins` key is absent — with `src/plugin/manage/remove.rs` byte-identical to `HEAD`, proved empirically rather than assumed. #185's slice keeps its own record in `odd/tasks/plugins-release-artifact.md`. Issue **#169 is CLOSED as completed**, with two residuals stated in the closing comment: execution of the published artifact inside a real release pipeline, and Windows/macOS cache-root resolution exercised only through the injected lookup. |

The live run is also where the empty `[plugins]` parent surfaced; D23 fixes it in this work unit and
pins the round trip byte-for-byte, so the finding is recorded as resolved rather than carried as debt.

## Recorded debt

- **`src/plugin/error.rs` is 309 production lines** against Pillar V.4's ≤300 *target* (HEAD was already
  303). The mandatory decomposition trigger (~350) and the hard ceiling (400) are satisfied, and the
  slice-3b vocabulary now lives in its own modules. Reaching ≤300 would mean moving the pre-existing
  `Registry*` group into a sub-error as well, re-opening verified slice-2 code and its 21 tests for no
  behavioural gain; it is recorded instead of done.
- **`src/cli/runner/plugin/add.rs::run_add` counts 47 non-blank lines (~35 excluding brace-only lines)
  with match arms nested three levels deep**, at or over Pillar V.1/V.4 depending on how "effective" is
  counted. It deliberately mirrors the already-verified `run_remove` shape from slice 3a; changing only
  one of the two would leave the pair inconsistent, so both wait for one follow-up that touches neither
  slice's behaviour.
- **The `Transport` trait is compile-checked, not exercised, over real HTTPS** (D9/D10): the production
  `ureq` implementation has no behavioural test, because an https-only client cannot be pointed at a
  loopback fixture. Stated rather than hidden.
- **Acceptance items left unchecked are exactly the ones the hermetic suite cannot reach**: the real
  `github-releases` download, loading the real asset, Windows/macOS cache-root resolution, and CI for
  this slice.

## Out of scope

Registry schema extension with `events`/`manifest_match` (follow-up in `cutver/registry`),
`plugin update`, pinning policy, a config-pinned internal registry, `doctor` plugin health, a
registry publishing path, signing, and any trust root beyond the pinned digest.
