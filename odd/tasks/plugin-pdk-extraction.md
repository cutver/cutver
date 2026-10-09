# Feature: Extract the Contract Crate (`cutver-pdk`) into the Core

> **Status**: In Progress
> **Target Version**: v0.13.0
> **Law**: [CONTRACT.md](../../CONTRACT.md) (Pillars I, III, V)
> **Unblocks**: `cutver plugin schema` (layer 3), third-party plugin authors, `cutver/registry`

---

## Why

The plugin wire contract is currently defined **twice**:

| Where | What it defines |
| --- | --- |
| `src/plugin/types.rs` + `src/plugin/dto/` | the canonical types |
| `cutver/plugins/crates/cutver-pdk` | a copy, kept in sync by hand |

Layer 1 added `tests/plugin_wire_contract.rs`, which is a **detector with a hole**: it fails when the
core changes the contract, but it cannot fail when someone updates cutver's golden fixture and forgets
the plugin's copy. The plugin would keep passing against its own stale types until a real user hit it.

Layer 2 closes the hole by construction: the contract lives in one crate, and cutver itself depends on
it. The plugin then consumes literally the same types, so there is nothing left to drift.

Layer 2 also unblocks layer 3. `cutver plugin schema` must be a subcommand of the core, so the contract
has to live in the core for the core to emit it. Building the schema generator in the plugins
repository first would mean moving it later.

---

## Design

`crates/cutver-pdk` becomes a workspace member of this repository and owns **only the wire contract**:

- `Capability` and `ParseCapabilityError`
- `PluginOperation` and `ParsePluginOperationError`
- `ChangelogRenderRequest`, `ChangelogRenderResponse`, `PluginCommitEntry`, `PluginContributor`
- `ManifestReadRequest`, `ManifestReadResponse`, `ManifestWriteRequest`, `ManifestWriteResponse`
- `PreBumpPayload`, `PreBumpResponse`, `PostBumpPayload`, `PostReleasePayload`
- `VersioningComputeRequest`, `VersioningComputeResponse`
- `PluginInvocation` (typed: `Capability` + `PluginOperation` + `Value`)

**`PluginCall` stays in the core.** It couples a capability to its operation for host dispatch, which
is a host-side convenience, not part of the wire. Keeping it out keeps the PDK a pure contract.

**Host configuration types stay in the core**: `PluginName`, `RuntimeKind`, `PermissionsConfig`,
`PluginConfig`. They are `cutver.toml` surface, not inter-process wire.

`src/plugin/types.rs` and `src/plugin/dto.rs` become **facade re-exports**. This is the whole trick:
every existing import goes through `crate::plugin::types::*` or `crate::plugin::dto::*`, so all 166
usages keep resolving without a single edit. Confirmed beforehand: those modules expose no
`pub(crate)` items, only `pub`.

The dependency gap is declared the publishable way:

```toml
cutver-pdk = { version = "0.1.0", path = "crates/cutver-pdk" }
```

Local builds resolve by path; `cargo publish` resolves by version.

---

## Acceptance Criteria
- [ ] `crates/cutver-pdk` exists as a workspace member and depends only on `serde` and `serde_json`.
- [ ] The wire types are defined exactly once, in the PDK. The core keeps no duplicate definition.
- [ ] `crate::plugin::types::*` and `crate::plugin::dto::*` still resolve for every existing caller, with zero edits to those callers.
- [ ] `PluginCall` and the host configuration types remain in the core.
- [ ] The DTO serde tests travel with the types.
- [ ] Zero clippy warnings with and without `--features plugins`, canonical formatting, green suite.

---

## Task Breakdown
- [ ] Task 1: Create `crates/cutver-pdk` with the contract types and their tests.
- [ ] Task 2: Convert the repository to a workspace and point the core at the PDK.
- [ ] Task 3: Turn `src/plugin/types.rs` and `src/plugin/dto.rs` into facade re-exports and remove the duplicated definitions.
- [ ] Task 4: Full verification suite.

---

## Follow-on work units (not this one)

- **WU-B**: publish both crates. The release workflow must publish `cutver-pdk` **before** `cutver`, or
  cutver cannot resolve its dependency. This is the risky part: that workflow has already failed twice
  in this repository's history (missing Rust toolchain, stale floating action tag).
- **WU-C**: switch `cutver/plugins` to the published PDK and delete its local copy.
- **WU-D**: layer 3, `cutver plugin schema`, generating JSON Schema from the PDK types for polyglot
  plugin authors.

---

## Evidence & Verification
- **Branch**: `feat/extract-cutver-pdk`
- **Validation**: pending
