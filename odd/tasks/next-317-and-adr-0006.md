# Next session: #317 and ADR-0006

Handoff from the session that cut `v0.22.0-beta.1` (`main` at `135a34d`). Two items, and they are different
kinds of thing: **#317 is a decision the maintainer owns** with three routes and a live contradiction to
settle; **ADR-0006 is specified work** with several rules half-built. Do not read this as a queue of tasks.

## State: #317, `[plugins.<name>.config]`

Parsed, preserved, and never delivered. The chain, with evidence:

- `PluginConfig` carries it: `src/plugin/types.rs:197-214`, field `:212-213` (`Option<toml::Table>`,
  `#[serde(default)]`), defaulted `:228`. Reached from `Config` at `src/config/types.rs:134`.
- The unknown-key gate accepts the key but never descends into it: `src/config/validation.rs:187-198`
  (`PLUGIN_KEYS` includes `"config"` at `:197`), `check_plugin_item` `:355-368`.
- **Delivery does not happen.** Dispatch builds the envelope from `call` + `payload` only
  (`src/plugin/manager/core.rs:264`), and the envelope has four fields — `protocol, capability, operation,
  payload` (`crates/cutver-pdk/src/invocation.rs:15-24`). The factory consumes runtime, source, hash,
  capabilities, permissions, timeout and command, not this table (`src/plugin/manager/factory.rs:55-77`,
  `:87-118`). The accessor `PluginManager::config` (`src/plugin/manager/core.rs:76-78`) has three callers
  and every one only tests presence: `src/bump/plan/resolve.rs:147`,
  `src/changelog/render/body.rs:68`, `src/manifest/dispatch.rs:39`.
- Every test pins **parsing or preservation**, none pins delivery or ignoring: `src/plugin/types.rs:290-303`,
  `src/config/types.rs:599-627`, `tests/plugin_install.rs:322-348`,
  `src/plugin/manage/update.rs:374,400-401`.

### The contradiction to settle first

Two documents in this repository assert different things about the same key:

- `docs/guides/plugin-github-publish.md:60-62` says `config.repository` and `config.draft` are required and
  effective, and `tests/release_pipeline_contract.rs:448-460` asserts `draft == Some(true)` from this
  repository's own table.
- `docs/reference/configuration-tables.md:126-137` and `cutver.toml:76-78` say the host does not deliver the
  table and no host behaviour depends on it.

Both cannot be true. There is a third fact that decides it and is **not verified here**: `cutver.toml:71-72`
grants the plugin `filesystem = ["."]` with a comment about "the two files the plugin reads". If the
published `github-publish` reads `cutver.toml` itself through that grant, the `draft` guarantee is real and
the reference page is wrong; if it does not, the guide is wrong and the plugin publishes a public release.
Closing that needs the `cutver/plugins` repository read, which was not in this workspace.

## Decisions the maintainer owns

1. **Which route #317 takes.** Costs, from the code:
   - *Deliver it* — `crates/cutver-pdk/src/invocation.rs:15-24` gains a field and `PROTOCOL_VERSION`
     (`:11`) moves, so it is a wire-contract change: `tests/plugin_request_goldens.rs:15-31` pins the
     envelope byte for byte with sha256 digests shared with `cutver/plugins`, the committed twin
     `tests/fixtures/plugin_wire_contract/changelog_request.json` moves with it, the strict decode
     (`src/plugin/dto.rs:56-60`) rejects a mismatched `protocol`, and the PDK is a separately published
     member (`Cargo.toml:16,19`, pinned per release at `cutver.toml:59-62`). A value only lands after an
     external release.
   - *Remove it* — `src/plugin/types.rs:212-213,228`, `src/config/validation.rs:197`, `cutver.toml:74-81`
     and four doc passages go; `tests/release_pipeline_contract.rs:448,478` stop compiling and
     `tests/plugin_install.rs:322-348` with `src/plugin/manage/update.rs:374-401` lose their subject; the
     plugin loses its only configuration route with no replacement.
   - *Declare it inert* — the deprecation channel cannot express it, because a `Deprecation` row demands a
     `canonical` replacement (`src/config/deprecations.rs:21-37`, table `:44-62`). The precedent is
     doctor's per-grant effect (`src/cli/doctor/plugins.rs:56-80`, note at `:78`), which is filesystem-only.
2. **What happens to the guide's `draft`/`repository` claim** once the route is chosen — it is either true
   and the reference is wrong, or inert and the guide is a lie about a guarantee.
3. **`cutver-gh`**: a shipped binary or a readable template? External dispatch already resolves
   `cutver-<subcmd>` (`src/cli/runner/external.rs:10-30`), and `cutver-gh` has no code in this repository.
4. **Whether any CI-file parsing may ever enter the core** (ADR `:85-87`, `:136-141`); `CONTRACT.md:222`
   requires that decision be recorded in the pull request and the changelog.

## State: ADR-0006, `docs/adr/0006-progressive-onboarding-and-composable-primitives.md`

`Status: proposed` (`:3`), `Implementation: not started` (`:13-18`), one dated status note at `:8-12`
whose order is `#404 → honesty pass → rule 1 → rule 2 → adapter`.

| Rule | Says | State today |
| --- | --- | --- |
| 1 `:69-76` | smaller scaffold that grows by subcommand, `init --dry-run` and `--json` | **partly.** `init --json` shipped (`src/cli/args/commands.rs:42-44`, object `src/init/scaffold/runner.rs:113-128`). `--dry-run` does not exist: the only `dry_run` in the arg tree belongs to `bump` (`:56`) and `publish` (`:111`), and `run_init` writes unconditionally (`runner.rs:170-186,194-206`). |
| 2 `:78-83` | `cutver gate add\|remove\|list` in core | **absent.** No `Gate` variant in `src/cli/args/commands.rs:27-44`; nothing writes `[gates] require` (`src/bump/exec/guard.rs:105-107` is read-only). |
| 3 `:85-87` | `doctor` reads no CI files | **satisfied trivially** — no `src/**` reference to workflows; the adapter half is unbuilt. |
| 4 `:89-93` | honesty before disclosure; machine output before automation | **partly.** #404 landed; the three inert-key items are unmarked: this table, `[project] name` (`src/config/types.rs:138-139`), `[version] strategy`'s dead values. |

The scaffold also does not match rule 1's shape: `src/init/scaffold/config_gen.rs:43-112` emits
`[[manifest]]`, `[preflight]`, `[changelog]`, `[git]`, `[hooks]` and `[publish]`, and no `[project]`, though
the schema has one (`src/config/types.rs:138-139`).

### Stale statements to correct when it starts

`ADR:50-52` says `publish`, `attest` and `init` reject `--json` as a usage error — false since #404;
`ADR:73-74` "neither exists today" is half-false; `ADR:31-34` cites `tests/release_pipeline_contract.rs:20-32`
where the pin now lives at `:824-835`. `odd/tasks/machine-surface-release-tail.md:5` still says "Blocks:
ADR-0006 rule 4" while its own `:68` says the prerequisite landed. Nothing names ADR-0006 in `CONTRACT.md`
and no test pins its status, so it can drift silently.

## Work that is specified and waiting

1. `init --dry-run` — ADR `:72-74`, `:117-119`. `src/init/scaffold/tests.rs:387-392` pins its **absence** from
   `NEXT_STEPS`; that test changes with the work.
2. `cutver gate add|remove|list` — comment- and key-order-preserving edits, plus a channel-neutral
   next-steps block (ADR `:78-83`).
3. The honesty pass on `[project] name` and `[version] strategy`'s dead values (ADR `:42-47`, `:89-93`).
4. The missing guide for wiring `[gates] require` into a workflow. Its two trap facts are in the tree:
   `.github/workflows/release.yml:682-689` (`include-hidden-files: true`, for `actions/upload-artifact`
   skipping hidden paths) and `:694-697` (the publishing job's `needs:` line, pinned by
   `tests/release_pipeline_contract.rs:824-835`). The beta.1 notes name this as the documentation gap
   (`.github/releases/v0.22.0-beta.1.md:344-350`).
5. Citation sweep: `docs/reference/configuration-tables.md:131` and `:135`, `docs/reference/plugins.md:123`,
   `docs/guides/plugin-github-publish.md:59`, `docs/guides/plugin-overview.md:155` (points at a page that
   only links).

## Rails

- **Source ratchet** `tests/source_line_budget.rs`: `CEILING = 400` (`:24`); the allowlist (`:28-42`) carries
  `config/{discovery,path,types,validation}.rs` and they may not grow. **`src/init/` has no allowlist entry**,
  so `src/init/scaffold/tests.rs` at 392 has 8 lines of headroom and `config_gen.rs` ~258, `runner.rs` ~318.
- **Docs ceiling 200** (`tests/ci_workflow_contract.rs:302`, mirrored in `ci.yml`), and two pages this work
  would touch are **full**: `docs/reference/plugins.md` 200/200 and
  `docs/internals/machine-contract-surfaces.md` 200/200. `CONTRACT.md:141` adds the survivor rule: a
  compaction may not drop a statement.
- **Invisible on a Windows host**: `tests/e2e_init.rs`, `tests/e2e_doctor.rs`, `tests/e2e_bump.rs`,
  `tests/e2e_lifecycle.rs`, `tests/changelog_plugin.rs`, `tests/manifest_plugin.rs`,
  `tests/versioning_plugin.rs` and the `e2e_changelog_*` suites are `#![cfg(unix)]`. For `init` the
  cross-platform carriers are `tests/attest_init_json.rs` and `src/init/scaffold/tests.rs`; treat
  "0 tests executed" as unverified, never as green.

## Log

**L1** (user, verbatim): «cuando pase verde mergea y cierra dejando todo listo para el 317 y el ADR 006 en otra session»

**L2** (how this was produced): a read-only exploration of `main` at `135a34d`, every claim carrying a
`path:line`. It was not re-verified line by line by the session that wrote this document; the one question it
could not close — whether the published `github-publish` reads `cutver.toml` through its filesystem grant —
needs the `cutver/plugins` repository.
