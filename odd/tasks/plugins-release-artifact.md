# ODD feature: ship the plugin surface as its own artifact (#169, decision A)

> Issue: [cutver/cutver#169](https://github.com/cutver/cutver/issues/169) (decision A) · Branch:
> `feat/plugins-release-artifact` · Base: `main` @ `d6085e0`

## Goal

Every acceptance item of #169 is unusable by a real user while `release.yml` builds without
`--features plugins`: the published binary has no `plugin` command and cannot execute a wasm plugin.
This slice publishes the plugin surface as a second artifact, so the lean default stays lean and
plugin users stop hand-building from source.

## Why

- **Decision A** (user-authorized 2026-10-10): two artifacts, lean by default. `cutver` stays lean in
  every channel; a second `cutver-plugins` archive carries the plugin surface, plus a `:plugins`
  container tag. Nobody pays 4× for a feature they do not use.
- Measured in the size spike: the plugin feature takes the release binary from ~1.25 MB to ~5 MB
  compressed.
- `release.yml` has broken releases twice, so the governing rule here is: **add jobs, never edit the
  proven ones**, and never make `publish` depend on something whose failure mode is unclear.

## Design (parent-owned)

- **D1 — A mirror build job.** New `build-plugins` mirrors the proven `build` matrix 1:1 (same eight
  targets, same packaging shape) but builds with `--features plugins`, names its archives
  `cutver-plugins-<version>-<slug>` and uploads them as `asset-plugins-<slug>`. The existing `build`
  job, its matrix and its steps are **not** touched.
- **D2 — `publish` needs no logic change.** It already downloads `pattern: asset-*` with
  `merge-multiple: true` into `dist/` and signs `cutver-*` into `SHA256SUMS`, so the plugin archives are
  published and summed automatically. Only `publish.needs` grows. The Homebrew and Scoop steps grep
  `cutver-<version>-<slug>`; that pattern cannot match `cutver-plugins-<version>-<slug>` because the
  version does not follow `cutver-` there. This must be asserted, not assumed.
- **D3 — No plugin artifact is published without being executed.** New `smoke-plugins` mirrors the
  proven `smoke` matrix (same eight native runners) and runs a new
  `.github/scripts/release-smoke/run-plugins.sh <binary> <version>`, which delegates to the existing
  `run.sh` (one source for the base contract) and then asserts the plugin surface is really inside the
  artifact: `plugin list --json` must exit 0 and report zero plugins against the fixture project. This
  is exactly the property #169's acceptance table flagged as broken.
- **D4 — The image gains a `:plugins` variant without touching the proven job.** A new job builds the
  same `Containerfile` with `--build-arg CUTVER_FEATURES="--features plugins"` (the knob landed in #175)
  and pushes `:<version>-plugins`, `:<major>.<minor>-plugins` and `:plugins`; a plugins smoke runs the
  new script inside the image at the pushed digest. The existing lean job stays untouched.
- **D5 — The docs say where the plugin surface actually ships.** `docs/reference/plugins.md` stops
  implying that everyone must build from source: it names the published plugin archives and the
  `:plugins` image tag, and keeps `--features plugins` as the path for versions that predate them.
- **D6 — The residual is stated, not hidden.** `release.yml` is `workflow_dispatch`-only, so its job
  graph cannot be exercised without cutting a real release; the next real release is the first
  end-to-end execution of these jobs, exactly like #174's smoke job. What *is* verifiable here: the new
  script's logic against a locally built plugin-enabled binary and RED against a lean one, POSIX-sh
  validity, and both workflows' YAML validity plus their job-graph structure.

## Cost accepted

Eight extra release builds per release (wasmtime/cranelift under `lto = "fat"`). Rejected alternative:
building the plugin artifact for a subset of targets, which would make some platforms second-class.

## Acceptance

- [x] A `cutver-plugins-<version>-<slug>` archive is produced for every target of the lean matrix.
- [x] `publish.needs` includes both new jobs; no other `publish` step changes.
- [x] The plugin archives reach `dist/` and `SHA256SUMS` through the existing globs.
- [x] The Homebrew and Scoop greps still resolve the lean archives and cannot match the plugin ones.
- [x] `run-plugins.sh` passes against a plugin-enabled binary and **fails** against a lean one.
- [x] `:plugins` image tags are pushed and the plugins smoke runs inside the image.
- [x] `docs/reference/plugins.md` names the artifacts and the image tag.
- [x] Both workflows are valid YAML and their job graph matches this design (`yq` assertions).

All eight are verified **structurally**: the job graph, the matrices, the globs and the greps were
proven against the files, and the new gate was executed for real against both binaries. None of them is
proven by an actual release; see D6 and the residual in the evidence row.

## Known edges

- **Container tag collision on a contrived branch name.** The proven lean job tags
  `type=ref,event=branch`, so a `workflow_dispatch` run from a branch literally named `plugins` or
  `<version>-plugins` would emit a lean image under a plugin tag. On the normal path — the
  `release: published` event, where the ref is a tag — it cannot happen. Fixing it means touching the
  lean job's tag list, which this slice deliberately does not do; the one-line remedy if that job is
  ever edited is to scope or drop `type=ref,event=branch`.
- **Shared GHA cache scope.** Both container jobs use the default `type=gha` scope, so a concurrent
  run could race on the cache save. It cannot affect image correctness, and scoping the new job's
  cache would be a one-line change inside it; it is recorded rather than changed after verification.

## Out of scope

Changing the lean artifact in any way (matrix, packaging, smoke); a Homebrew formula or Scoop manifest
entry for the plugin variant; signing beyond the existing SHA256SUMS + cosign flow; any change to the
plugin CLI itself.

## Evidence

| Date | Slice | Evidence |
| --- | --- | --- |
| 2026-10-10 | design | Decision A recorded in `odd/tasks/plugin-install-cli.md`; `release.yml` verified to build with `cargo build --locked --release --target …` and no `--features plugins`; the `Containerfile` `ARG CUTVER_FEATURES` knob from #175 verified present; `yq` available for YAML assertions, `shellcheck` not available on this host. |
| 2026-10-10 | 5 | This work unit adds `build-plugins` + `smoke-plugins` to `release.yml` (470 → 652 lines), `build-and-push-plugins` + `smoke-plugins` to `container.yml` (117 → 218), a new `.github/scripts/release-smoke/run-plugins.sh` (78 lines, POSIX sh), and the artifact wording in `docs/reference/plugins.md` (146 → 150 lines, inside the ≤150 target). **The proven jobs are provably untouched**: a canonical `yq -o=json` diff against `HEAD` shows `build` (11 steps) and `smoke` (5 steps) identical, `publish` differing only by the two `needs` entries, and both lean container jobs identical; `Containerfile`, `run.sh`, `Cargo.toml`, `Cargo.lock`, `src/**` and `tests/**` are byte-unchanged. **The new gate was executed, not just reviewed**: RED against a lean binary (`FAIL: plugin list --json failed; this artifact lacks the plugin surface`), GREEN against a plugin-enabled one, and a triangulation proving the assertion is about an **empty list** rather than exit code — with a declared `[plugins.demo]` the command exits 0 while printing a non-empty list and the script's check does not match. The D2 claim was falsified in the right direction: against a simulated `SHA256SUMS`, each Homebrew/Scoop pattern selects exactly its own lean line and **zero** plugin lines, while `sha256sum cutver-*` does pick the plugin archives up, which is the intended path into the release. POSIX validity checked with `sh -n`, `dash -n` and a real run under `dash` (`shellcheck` is not installed on this host and was not run). Independent verification returned **PASS on all nine claims** and independently reproduced the RED/GREEN, the triangulation, the grep falsification and the canonical job diff. **Residual (D6): the release job graph is verified structurally only** — `release.yml` is `workflow_dispatch`-only, so the next real release is its first end-to-end execution, exactly like #174's smoke job. |
