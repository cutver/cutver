# The Reference Plugin: `github-publish`

This repository cuts its own release through a plugin: `github-publish` creates the GitHub release, and
`.github/workflows/release.yml` fills it and then promotes it. The declaration in `cutver.toml` and that
workflow are therefore a working example rather than a sketch.

Everything here is scoped to what this repository can prove: the declaration, the six release jobs and
the contract tests that pin them. The plugin itself is not built here, so the page names only the keys
this repository sets and says where the rest lives ([below](#what-this-page-cannot-verify)).

## What it does

`github-publish` is a `lifecycle.v1` plugin (`cutver.toml:74`) that owns one artefact: the GitHub
release for the tag `cutver` just created. It declares two slots (`cutver.toml:75`):

| Slot | When it runs here | What it does |
| :--- | :--- | :--- |
| `on_pre_bump` | After preflight, before any manifest or changelog byte is written ([plugin overview](plugin-overview.md):43). | Checks the token and looks for a tag collision against the API; the `--dry-run` rehearsal exercises it (`cutver.toml:59-62`). |
| `on_post_release` | Last step of `cutver publish`, after the tag exists and `[publish].commands` ran ([plugin overview](plugin-overview.md):45; `src/bump/exec/publish.rs:422`). | Creates the release **as a draft** from the notes and title it was handed (`cutver.toml:60-62`). |

Two consequences everything else depends on:

- The title comes from `title_template` and the body from the notes `cutver publish` streamed in. The
  pinned version publishes that entry verbatim and fails closed when the host sends none
  (`cutver.toml:68-71`).
- A draft is not a release: nothing is visible until the workflow promotes it, after the assets exist.

## The declaration in `cutver.toml`

```toml
[plugins.github-publish]
runtime = "wasm"
source = "github-publish-0.2.1.wasm"
hash = "0a38c934200f833a35cae3a53c03c403566f42f4abc58a744a7b2d8a09874a9b"
capabilities = ["lifecycle.v1"]
events = ["on_pre_bump", "on_post_release"]

[plugins.github-publish.permissions]
network = ["api.github.com", "uploads.github.com"]
env = ["GITHUB_TOKEN", "GITHUB_REPOSITORY"]
filesystem = ["."]

[plugins.github-publish.config]
repository = "cutver/cutver"
draft = true
title_template = "cutver {version}"
```

Every key above, and what depends on it (`cutver.toml:66-94`):

| Key | Value here | What depends on it |
| :--- | :--- | :--- |
| `runtime` | `wasm` | The load-time digest gate is the WASM path: a declared `hash` is re-verified on every load (`src/plugin/driver/wasm.rs:195-197`). |
| `source` + `hash` | `github-publish-0.2.1.wasm` and its SHA-256 | One pin, in one file. The workflow hydrates it with `cutver plugin restore` and never names a version of its own (`tests/release_pipeline_contract.rs:59-74`). |
| `capabilities` | `["lifecycle.v1"]` | Lifecycle events only reach a plugin that declares this capability (`tests/release_pipeline_contract.rs:134-151`). |
| `events` | both slots | A non-empty `events` list is exactly the set of slots dispatched to the plugin ([plugin overview](plugin-overview.md):49-50). |
| `permissions.network` | `api.github.com`, `uploads.github.com` | Hosts the guest may reach. The assets here are uploaded by `gh release upload` in the workflow, not by the plugin (`release.yml:684`). |
| `permissions.env` | `GITHUB_TOKEN`, `GITHUB_REPOSITORY` | The only two keys the guest sees: both runtimes clear the rest of the environment (`release.yml:161-162`). The step must export them, or invoking the plugin fails closed (#300; `tests/release_pipeline_contract.rs:170-176`). |
| `permissions.filesystem` | `["."]` | The whole workspace tree, wider than the files the plugin reads; kept to match the configuration the sandbox validates (`cutver.toml:80-82`). |
| `config.repository` | `cutver/cutver` | The plugin needs it to address the API (`tests/release_pipeline_contract.rs:125-131`). |
| `config.draft` | `true` | Required, and its default is `false`: without it the plugin publishes a public, asset-less release before the binaries are built (`cutver.toml:86-89`; `tests/release_pipeline_contract.rs:116-132`). |
| `config.title_template` | `cutver {version}` | The plugin's default title is the bare tag; this names the release (`cutver.toml:90-91`). |

`checksums` and `generate_release_notes` are deliberately unset, and `cutver.toml:92-94` records why:
the assets are uploaded by the workflow, and this repository renders its own notes.

## The draft-then-promote flow

Six jobs, in dependency order (`tests/release_pipeline_contract.rs:19-22`):

| Job | What it does to the release |
| :--- | :--- |
| `version` | Builds the plugin-enabled CLI from this commit (`release.yml:103-110`), restores the declared plugin artifacts (`release.yml:116-122`), then runs `cutver bump --defer-publish` (`release.yml:133-146`). That dispatches `on_pre_bump` only, because `--defer-publish` leaves `on_post_release` to `cutver publish` ([plugin overview](plugin-overview.md):51-53) — no tag, no crate, no release (`tests/release_pipeline_contract.rs:179-202`). |
| `build`, `build-plugins` | Build and package both artifacts from the release commit. |
| `smoke`, `smoke-plugins` | Run the packaged binaries. The plugin smoke proves the plugin surface is compiled in, not that the plugin works (`.github/scripts/release-smoke/run-plugins.sh:70-76`). |
| `publish` | The only job that publishes, and it waits for the five jobs above (`release.yml:543-544`; `tests/release_pipeline_contract.rs:204-226`). |

Inside `publish`, in this order (`tests/release_pipeline_contract.rs:253-293`):

1. `cutver publish --tag … --commit … --notes-file -` creates the tag, pushes it, moves the floating
   major tag, runs `cargo publish`, and dispatches `on_post_release`, which creates the draft
   (`release.yml:625-637`).
2. The asset artifacts are downloaded, summed and signed (`release.yml:639-659`).
3. The release state is read, and `gh release upload "$TAG" dist/* --clobber` fills the draft
   (`release.yml:661-684`).
4. The promotion publishes by the channel `cutver bump` reported: `stable` runs
   `gh release edit "$TAG" --draft=false --prerelease=false --latest=true`, `alpha|beta|rc` runs the same
   with `--prerelease --latest=false`, and any other value fails closed instead of guessing — so a
   prerelease can never take the repository's Latest slot. A second read proves it is no longer a draft
   (`release.yml:789-844`).

### What the workflow deliberately does not repeat

- **The plugin pin.** `source` and `hash` live in `cutver.toml`; a second copy of the artifact name in
  the workflow drifted the moment the pin moved (`tests/release_pipeline_contract.rs:59-74`).
- **The release itself.** No `softprops/action-gh-release` and no `gh release create` anywhere: a second
  creator races the plugin for the same tag (`tests/release_pipeline_contract.rs:350-375`), and no
  workflow other than this one may write release metadata (`tests/release_metadata_ownership.rs:45-77`).
- **The title and the body.** The promotion passes `--draft=false` and nothing else: a `--title` or a
  body there gives the release a second owner, which is how `title_template` once came out dead
  (`release.yml:706-712`; `tests/release_pipeline_contract.rs:424-448`).

## How a release fails

| What is wrong | What you see | What is left behind |
| :--- | :--- | :--- |
| The build has no plugin runtime | `this build of cutver was compiled without the 'plugins' feature, so it cannot load WASM plugin 'github-publish'` (`src/plugin/error.rs:24-29`, raised at `src/plugin/manager/factory.rs:20-23`) | Nothing: `bump` fails closed on the declaration (`cutver.toml:64-65`). Both releasing jobs build with `--features plugins` for this reason (`release.yml:103-108`, `release.yml:596-608`). |
| The artifact is not in the cache | `failed to read WASM artifact 'github-publish-0.2.1.wasm': …` (`src/plugin/driver/wasm.rs:188-193`) | Nothing: the driver is created before any slot runs. The step that prevents it is `cutver plugin restore` (`release.yml:116-122`). |
| The cached bytes are not the pinned build | `failed integrity check: expected hash … got …` (`src/plugin/driver/wasm.rs:195-197`; `src/plugin/error.rs:113-117`) | Nothing. The pin is one file: moving it is a release-configuration change, not a retry. |
| A declared `permissions.env` key is unset | `plugin 'github-publish' declares the environment grant 'GITHUB_TOKEN', but cutver cannot read …` (`src/plugin/error.rs:133-138`) | Nothing: it fails when the slot invokes the plugin, before the plugin runs, and no release is created. Both releasing steps export both keys (`release.yml:161-162`, `release.yml:711-712`). Commands that never invoke the plugin — `cutver doctor`, `cutver plugin restore` — run without them (#300). |
| No notes reached the plugin | `cutver publish` refuses an empty stream before creating the tag (`release.yml:627-630`), and the plugin itself fails closed when the host sends none (`cutver.toml:68-71`) | The tag is not created, so nothing needs undoing. |
| The plugin never created its draft | The upload guard stops the job and says to check that `cutver publish` ran on the plugin-enabled build and that `on_post_release` succeeded (`release.yml:679-683`) | The tag and the crate exist; the release does not. The guard never creates a release to cover for the plugin (`release.yml:665-670`). |

## What this page cannot verify

- **The plugin's full key set.** Every key beyond the table above belongs to the plugin, not to
  `cutver`: the host parses `config` and reads nothing out of it
  ([the configuration tables reference](../reference/configuration-tables.md), where the plugin tables live). A key this repository does not set
  is not observable from here, so this page does not name one.
- **Where the plugin lives.** It is not built in this repository: `crates/` holds the guest SDK
  (`cutver-pdk`) and `src/plugin` is the host. The artifact arrives from the Cutver registry index
  (`src/plugin/registry/fetch.rs:18`), and `cutver plugin info github-publish` prints that entry's
  repository, tier, capabilities and releases (`src/plugin/registry/info.rs:19-21,39`). That entry is
  where the plugin's source and its own documentation live.
- **Whether the plugin's behaviour matches this page.** Every claim above is about this repository's
  declaration and workflow, pinned by contract tests that read those two files.

**The residual, in this repository's terms.** `cargo publish` runs inside `cutver publish`, while the
assets and the promotion come afterwards, so an upload failure leaves a published crate version beside
a draft nobody can see. Nothing was lost: re-run the upload and the promotion, which clobbers the
assets and treats an already-published release as a no-op
([pipeline guarantees](../internals/pipeline.md):96).

## Related documentation

- [Installing and Managing Plugins](../reference/plugins.md): the `[plugins.<name>]` keys, the install
  flow, the two digest gates and the cache.
- [Plugin Overview](plugin-overview.md): the capability set, the lifecycle slots and the wire contract.
- [Writing a Plugin](plugin-authoring.md): the registry entry a plugin needs before a release may pin it.
- [CLI Commands and Exit Codes](../reference/cli.md): `cutver publish`, `plugin` and their exit codes.
