# Capability Reference: The Request and the Answer, Field by Field

`cutver` writes one JSON request and reads one JSON answer back. The four-field envelope is the same for
every capability ([plugin overview](plugin-overview.md)); what changes per capability is `payload` and the
shape of the answer. This page is that pair for all four, with the fields in declaration order, so a plugin
author in any language can build the decoder from this page instead of from the Rust types.

## Which operation carries which pair

| Capability | Operation | Request type | Answer type |
| :--- | :--- | :--- | :--- |
| `manifest.v1` | `read` | `ManifestReadRequest` | `ManifestReadResponse` |
| `manifest.v1` | `write` | `ManifestWriteRequest` | `ManifestWriteResponse` |
| `changelog.v1` | `render` | `ChangelogRenderRequest` | `ChangelogRenderResponse` |
| `versioning.v1` | `compute` | `VersioningComputeRequest` | `VersioningComputeResponse` |
| `lifecycle.v1` | `on_pre_bump` | `PreBumpPayload` | `PreBumpResponse` |
| `lifecycle.v1` | `on_post_bump` | `PostBumpPayload` | any JSON value, discarded |
| `lifecycle.v1` | `on_post_release` | `PostReleasePayload` | any JSON value, discarded |

The capability and the operation travel together in the envelope and are coupled by the host
(`src/plugin/types.rs:90-123`); the payload and answer types are declared in `cutver-pdk`, one module per
capability. The envelope itself carries no permission channel and no field a plugin can use to name another
capability.

## One rule decides every optional field

A field marked **optional** below is declared `Option<T>` with
`#[serde(default, skip_serializing_if = "Option::is_none")]`. Three consequences, and a plugin may rely on
each of them:

- **An absent optional field is omitted from the request, never sent as `null`.** A decoder that tests
  `value == None` sees no key at all, so a plugin must treat a missing key as absent rather than malformed.
  Asserted on the struct in the PDK's own tests for `versioning.v1` and `lifecycle.v1`
  (`crates/cutver-pdk/src/versioning.rs:59`, `crates/cutver-pdk/src/lifecycle.rs:131`) and byte for byte in
  `tests/plugin_request_goldens.rs`, whose seven requests contain no `null`.
- **A field marked "present" is always there**, as `[]` or `false` when it has no value: `commits`,
  `contributors`, `is_breaking`, `is_first_contribution`, `is_prerelease`, `modified_files`, `dry_run`. These
  carry `#[serde(default)]` without `skip_serializing_if`, because a plugin may read them unconditionally.
- **Everything not marked** is required in both directions. An answer missing one does not decode, and
  `cutver` reports it as `InvalidPayload` naming the plugin and the capability.

The same rule applies to the answer in the other direction: an optional field you omit is not sent as `null`,
and the ones marked present must be there even when empty.

## `manifest.v1`

`read` replaces the host's parser for one manifest format; `write` replaces its writer.

| `ManifestReadRequest` | Type | | `ManifestReadResponse` | Type |
| :--- | :--- | :--- | :--- | :--- |
| `path` | string | | `version` | string |
| `content` | string | | | |

| `ManifestWriteRequest` | Type | | `ManifestWriteResponse` | Type |
| :--- | :--- | :--- | :--- | :--- |
| `path` | string | | `content` | string |
| `content` | string | | | |
| `current_version` | string | | | |
| `next_version` | string | | | |

`path` is workspace-relative with forward slashes and `content` is the file's bytes as text, so a plugin
never resolves a path against the caller's directory (`crates/cutver-pdk/src/manifest.rs:5-37`). The answer
to `write` is the whole new file content, not a diff.

## `changelog.v1`

`render` produces the body of the release entry; the host owns the heading and the file.

| `ChangelogRenderRequest` | Type | |
| :--- | :--- | :--- |
| `root_dir` | string | |
| `version` | string | |
| `tag_name` | string | |
| `previous_tag` | string | optional |
| `release_date` | string | |
| `commits` | array of `PluginCommitEntry` | present |
| `repository` | string | optional |
| `compare_url` | string | optional |
| `is_prerelease` | bool | present |
| `contributors` | array of `PluginContributor` | present |

| `PluginCommitEntry` | Type | | `PluginContributor` | Type |
| :--- | :--- | :--- | :--- | :--- |
| `sha` | string | | `name` | string |
| `message` | string | | `is_first_contribution` | bool, present |
| `type` | string | optional | | |
| `scope` | string | optional | | |
| `author_name` | string | optional | | |
| `pr_number` | string | optional | | |
| `is_breaking` | bool | present | | |

`is_breaking` is per commit, not a property of the release: a release with one breaking commit carries
`is_breaking: true` on that entry only (`crates/cutver-pdk/src/changelog.rs:5-73`). The answer is
`{ "body": "<markdown>" }`; a worked request and answer are in the
[plugin overview](plugin-overview.md).

## `versioning.v1`

`compute` decides the next version; the host writes it.

| `VersioningComputeRequest` | Type | |
| :--- | :--- | :--- |
| `root_dir` | string | |
| `current_version` | string | |
| `bump_level` | string | `major`, `minor`, `patch` or `auto` |
| `tag_prefix` | string | |
| `previous_tag` | string | optional |
| `commits` | array of string | present |

| `VersioningComputeResponse` | Type | |
| :--- | :--- | :--- |
| `next_version` | string | |
| `rationale` | string | optional |

`commits` holds commit **messages**, not entries: this capability runs before the changelog context exists
and receives no sha, type or author (`crates/cutver-pdk/src/versioning.rs:5-30`). `rationale` is carried
into the bump description, but the string itself is not printed: the plugin path pins the level to `patch`
with every count at zero (`src/bump/plan/resolve.rs:89-98`), and `BumpRationale::summary` only quotes its
sample at `major` level, so what the tree shows is `patch (default - no breaking changes or features)`
(`src/conventional/types.rs:63-77`).

## `lifecycle.v1`

Three slots, one request each. The first can stop the release; the other two observe it.

| `PreBumpPayload` | Type | | `PreBumpResponse` | Type |
| :--- | :--- | :--- | :--- | :--- |
| `root_dir` | string | | `allow` | bool, required |
| `current_version` | string | | `reason` | string, optional |
| `next_version` | string | | | |
| `bump_level` | string | | | |
| `tag_name` | string | | | |
| `dry_run` | bool | present | | |

| `PostBumpPayload` | Type | | `PostReleasePayload` | Type |
| :--- | :--- | :--- | :--- | :--- |
| `root_dir` | string | | `root_dir` | string |
| `current_version` | string | | `version` | string |
| `next_version` | string | | `tag_name` | string |
| `tag_name` | string | | `commit_sha` | string |
| `modified_files` | array of string | present | `dry_run` | bool, present |
| `dry_run` | bool | present | `notes` | string, optional |

`allow` is required: an answer that omits it does not decode, so a plugin that wants the release to continue
must say so. `reason` is quoted by `cutver` when `allow` is false. `notes` in `on_post_release` is the
rendered release entry, heading included — `cutver` produces it so the plugin publishes what the changelog
says instead of re-deriving it — and it is **absent** when the project declares no `[changelog] path`,
because the entry does not exist and inventing one is the failure mode the field prevents
(`crates/cutver-pdk/src/lifecycle.rs:50-70`).

The two `on_post_*` answers are decoded as `serde_json::Value` and bound to nothing
(`src/plugin/manager/core.rs:207-217`): any JSON value is accepted, including `null`, and none of it reaches
the release. The `protocol` key is still checked in those answers like in any other.

## What this page does not cover

- **The envelope**, its `protocol` key and the rules for answering a different generation of the contract:
  [the plugin overview](plugin-overview.md).
- **Declaring a capability** in `[plugins.<name>]`, including the closed set and the refusal when a call
  arrives for one the entry did not declare: the [plugin reference](../reference/plugins.md).
- **The exact bytes.** This page lists fields; `tests/plugin_request_goldens.rs` pins the serialized request
  for one instance of each of the seven shapes, with a `sha256` per request so the `cutver/plugins`
  repository can reconcile its decoder with one `sha256sum`.
- **A residual worth knowing:** nothing in the suite compares these tables to the structs. The bytes are
  pinned by a test; the field lists are pinned by a reader. The declarations they describe are in
  `crates/cutver-pdk/src/`, one module per capability.
