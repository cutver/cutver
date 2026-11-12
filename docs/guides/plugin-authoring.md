# Writing a Plugin

A Cutver plugin is a WebAssembly module that exports one function, `invoke`, and answers a JSON
envelope. This guide takes you from an empty directory to a release another user can install with
`cutver plugin add`.

> **The sandbox is the point.** A plugin gets no network, no filesystem and no environment access
> unless the user grants it in `cutver.toml`. Design for the grants, not around them: a plugin that
> needs the network declares `permissions.network`, and the user sees that before installing.

## 1. Scaffold the crate

```toml
# Cargo.toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
cutver-pdk = "0.1.0"
extism-pdk = { version = "1.4.1", default-features = false }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

`cutver-pdk` is types only: the envelope, the capability and operation enums, and the request and
response DTOs. Input and output are Extism's — a `String` in, a `FnResult<String>` out. The
`default-features = false` is deliberate: the HTTP stack is a feature, and a plugin that does not
declare `permissions.network` should not link it.

## 2. Export `invoke`

The host calls one symbol. Its name is a constant, not a convention:

```text
src/plugin/driver/wasm.rs:16   pub const INVOKE_EXPORT: &str = "invoke";
```

```rust
use cutver_pdk::{Capability, ChangelogRenderRequest, ChangelogRenderResponse, PluginInvocation};
use extism_pdk::{FnResult, plugin_fn};
use serde_json::Error as JsonError;

/// Pure and testable: no PDK types cross this boundary.
pub fn render_invocation(input: &str) -> Result<String, JsonError> {
    let invocation: PluginInvocation = serde_json::from_str(input)?;

    // Fail closed on a capability or operation you do not implement.
    if invocation.capability != Capability::ChangelogV1 {
        return Err(serde_json::Error::io(std::io::Error::other("unsupported capability")));
    }

    let request: ChangelogRenderRequest = serde_json::from_value(invocation.payload)?;
    let response = ChangelogRenderResponse { body: request.version.clone() };
    serde_json::to_string(&response)
}

#[plugin_fn]
pub fn invoke(input: String) -> FnResult<String> {
    Ok(render_invocation(&input)?)
}
```

Keeping the body in a plain function with `&str` in and `String` out is what lets you unit-test it
natively; the `#[plugin_fn]` wrapper only adapts Extism.

## 3. Decode, dispatch, answer

The envelope has three fields (`crates/cutver-pdk/src/invocation.rs:7-14`):

```json
{ "capability": "changelog.v1", "operation": "render", "payload": { } }
```

| Capability | Operations on the wire | Response DTO |
| :--- | :--- | :--- |
| `manifest.v1` | `read`, `write` | `ManifestReadResponse { version }`, `ManifestWriteResponse { content }` |
| `changelog.v1` | `render` | `ChangelogRenderResponse { body }` |
| `versioning.v1` | `compute` | `VersioningComputeResponse { next_version, rationale? }` |
| `lifecycle.v1` | `on_pre_bump`, `on_post_bump`, `on_post_release` | `PreBumpResponse { allow, reason? }`; the two `on_post_*` operations expect no response |

A plugin that handles several operations matches on `invocation.operation` and returns an
`unsupported operation` error for the rest, exactly as `github-publish` does for anything other
than `OnPreBump` and `OnPostRelease`.

## 4. Build for `wasm32-wasip1`

```bash
rustup target add wasm32-wasip1
cargo build --release --target wasm32-wasip1
```

> **Gotcha that bites at release time.** Cargo names the artifact after the crate with hyphens
> turned into underscores: `my-plugin` builds `my_plugin.wasm`. Cutver installs
> `source = "<name>-<version>.wasm"` with the hyphen, so rename it when you publish.

## 5. Declare it in `cutver.toml`

`cutver plugin add my-plugin` writes this table for you; add the permissions by hand, because the
plugin cannot grant itself anything:
```toml
[plugins.my-plugin]
runtime = "wasm"
source = "my-plugin-0.1.0.wasm"
hash = "<64 lowercase hex characters>"
capabilities = ["changelog.v1"]
permissions.network = ["api.example.com"]
permissions.env = ["MY_TOKEN"]
```

`runtime` is `wasm` or `process`. `permissions` accepts `network`, `env` and `filesystem`, mapped by
the host to the sandbox's allowed hosts, config keys and allowed paths
(`src/plugin/driver/wasm.rs:116`). A missing grant is a denied capability at runtime, not a warning.
## 6. Publish, then register

The install-time gate compares the bytes it downloaded with the digest in the registry, before the
artifact is renamed into the cache (`src/plugin/install/apply.rs:57`). So publish first and take the
digest from the published asset:

```bash
sha256sum my-plugin.wasm | awk '{print $1}'
```

Then submit an entry to the [Cutver registry](https://github.com/cutver/registry) — a TOML file
whose stem must equal `name`, with one `[[release]]` table per version:

```toml
name = "my-plugin"
description = "What it does, in one sentence."
author = "your-handle"
tier = "community"
repository = "https://github.com/you/my-plugin"
capabilities = ["changelog.v1"]

[[release]]
version = "0.1.0"
url = "https://github.com/you/my-plugin/releases/download/v0.1.0/my-plugin.wasm"
sha256 = "5f44775e7b6f14a0b7cc57c19a3c694b5f3c177971f797e1a61177bcc0ae5b23"
```

The `url` must be https and end in `.wasm`, and `sha256` must be exactly 64 lowercase hex
characters; either fails closed at parse time.

## What this guide does not cover

The design-level contract is in the [plugin architecture RFC](../rfcs/0001-wasm-plugin-architecture.md),
and the operational side in the [plugin reference](../reference/plugins.md). A plugin replacing a
manifest format uses the same envelope with the `manifest.v1` DTOs.
