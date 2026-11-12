//! Contract tests binding `cutver.toml` to `.github/workflows/release.yml`.
//!
//! The release workflow cannot be run from a test, but every coupling that would silently break a
//! release is text-visible. Each assertion names the release failure it prevents, so a future edit
//! to either file fails here instead of during a dispatch.
//!
//! Guards PR #229 (risk item 5: delivery/CI).

use std::path::PathBuf;

use cutver::config;
use cutver::plugin::types::{Capability, RuntimeKind};
use cutver::plugin::{PluginConfig, PluginName};

const PLUGIN: &str = "github-publish";

fn repo_text(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    // Git checks these files out with CRLF on Windows. Line endings are a checkout setting, not
    // content, so every assertion in this file compares normalised text.
    raw.replace("\r\n", "\n")
}

fn declared_plugin() -> PluginConfig {
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cutver.toml");
    let cfg = config::load(config_path).expect("the repository cutver.toml must load");
    let name = PluginName::new(PLUGIN).unwrap();
    cfg.plugins
        .get(&name)
        .cloned()
        .unwrap_or_else(|| panic!("cutver.toml must declare [plugins.{PLUGIN}]"))
}

/// The workflow must not repeat what `cutver.toml` already declares. A second copy of the
/// artifact name drifted the moment the pin moved, and the release then looked for a file nobody
/// wrote. `cutver plugin restore` reads `source` and `hash` from the configuration, writes into
/// the cache root the host resolves, and verifies the digest as it downloads.
#[test]
fn workflow_restores_the_declared_plugins_instead_of_pinning_them_again() {
    let workflow = repo_text(".github/workflows/release.yml");
    assert!(
        workflow.contains("cutver plugin restore"),
        "release.yml must hydrate the plugin cache with `cutver plugin restore`"
    );
    assert!(
        !workflow.contains(&format!("{PLUGIN}-v")),
        "release.yml must not name a plugin version of its own: cutver.toml owns the pin"
    );
}

/// The plugin only owns the release if it is a draft: its `draft` field defaults to false, and a
/// non-draft release is published during the version job, before any binary is built.
#[test]
fn plugin_creates_the_release_as_a_draft() {
    let plugin = declared_plugin();
    let plugin_config = plugin.config.expect("the publish plugin needs a config table");
    assert_eq!(
        plugin_config.get("draft").and_then(|value| value.as_bool()),
        Some(true),
        "`draft = true` is required: the default publishes a public, asset-less release before build"
    );
    assert!(
        plugin_config
            .get("repository")
            .and_then(|value| value.as_str())
            .is_some(),
        "the plugin needs `config.repository` to address the API"
    );
}

/// Events are compared by exact string, and lifecycle events only reach `lifecycle.v1` plugins.
#[test]
fn declared_capability_events_and_permissions_are_the_ones_the_host_uses() {
    let plugin = declared_plugin();
    assert_eq!(plugin.runtime, RuntimeKind::Wasm);
    assert!(plugin.capabilities.contains(&Capability::LifecycleV1));
    assert_eq!(
        plugin.events,
        vec!["on_pre_bump".to_string(), "on_post_release".to_string()]
    );
    for key in ["GITHUB_TOKEN", "GITHUB_REPOSITORY"] {
        assert!(
            plugin.permissions.env.iter().any(|declared| declared == key),
            "`{key}` must be declared in permissions.env to reach the guest"
        );
    }
    assert!(plugin.permissions.network.iter().any(|host| host == "api.github.com"));
}

/// A lean binary cannot create the wasm driver, and a boolean input only forwards if it is wired.
#[test]
fn workflow_requests_the_plugin_build_and_forwards_dry_run() {
    let workflow = repo_text(".github/workflows/release.yml");
    for needle in ["plugins: \"true\"", "dry_run:", "dry-run: ${{ inputs.dry_run }}"] {
        assert!(workflow.contains(needle), "release.yml must keep `{needle}`");
    }
    for key in ["GITHUB_TOKEN:", "GITHUB_REPOSITORY:"] {
        assert!(
            workflow.contains(key),
            "the release step must export `{key}`, or the declared permission arrives empty"
        );
    }
}

/// The plugin owns release creation, so the workflow must fill that draft and publish it — not
/// create a second release. A leftover `softprops/action-gh-release` would race the plugin for the
/// same tag and fail with 422.
#[test]
fn the_plugin_owns_release_creation_and_the_workflow_publishes_its_draft() {
    let workflow = repo_text(".github/workflows/release.yml");
    assert!(
        !workflow.contains("softprops/action-gh-release"),
        "release.yml must not create the release: the github-publish plugin does, as a draft"
    );
    for needle in [
        "gh release view \"$TAG\" --json isDraft",
        "gh release upload \"$TAG\" dist/* --clobber",
        "gh release edit \"$TAG\"",
        "--draft=false",
        "--notes-file RELEASE_NOTES.md",
    ] {
        assert!(workflow.contains(needle), "release.yml must keep `{needle}`");
    }
    // Assets must be uploaded before the draft becomes visible.
    let upload = workflow.find("gh release upload").expect("upload step");
    let publish = workflow.find("--draft=false").expect("publish step");
    assert!(
        upload < publish,
        "assets must be uploaded before the release is published"
    );
}

/// The guard must accept a draft, accept an already-published release as a retry, and fail only
/// when no release exists — while tolerating unexpected output rather than blocking a release on a
/// stray byte from `gh`.
#[test]
fn publish_guard_distinguishes_draft_published_and_missing() {
    let workflow = repo_text(".github/workflows/release.yml");
    for needle in [
        "if .isDraft then \"draft\" else \"published\" end",
        "draft|published) echo",
        "published)",
        "draft) ;;",
        "Could not read the release for $TAG",
        "Unexpected release state for $TAG",
        "Refusing to promote $TAG",
    ] {
        assert!(workflow.contains(needle), "release.yml must keep `{needle}`");
    }
    // An auth failure must surface gh's own message, never a fabricated "no release" verdict.
    assert!(
        workflow.contains("--json isDraft --jq 'if .isDraft then \"draft\" else \"published\" end' 2>&1"),
        "the state read must capture stderr so a 401 is diagnosable"
    );
    assert!(
        !workflow.contains("grep -qx true"),
        "a byte-exact match on `.isDraft` blocks the release on trailing whitespace"
    );
}

/// Both gh steps must authenticate and receive the tag; the publish job infers the repository from
/// the checkout, so a missing GH_TOKEN fails at the API rather than at the guard.
#[test]
fn publish_steps_pass_gh_token_and_tag() {
    let workflow = repo_text(".github/workflows/release.yml");
    assert_eq!(
        workflow
            .matches("GH_TOKEN: ${{ steps.release-token.outputs.token || secrets.GITHUB_TOKEN }}")
            .count(),
        2,
        "the upload and the publish step must both authenticate, falling back to GITHUB_TOKEN"
    );
    assert_eq!(
        workflow.matches("TAG: ${{ needs.version.outputs.tag }}").count(),
        2,
        "both gh steps must receive the tag as TAG"
    );
}

/// The notes the publish step reads must be the artifact the version job produced, downloaded into
/// the job's workspace root.
#[test]
fn release_notes_artifact_matches_the_publish_path() {
    let workflow = repo_text(".github/workflows/release.yml");
    assert!(
        workflow.contains("name: release-notes\n          path: RELEASE_NOTES.md"),
        "the version job must save RELEASE_NOTES.md as the `release-notes` artifact"
    );
    assert!(
        workflow.contains("name: release-notes\n          path: ./"),
        "the publish job must download `release-notes` into the workspace root"
    );
    assert!(workflow.contains("--notes-file RELEASE_NOTES.md"));
}

/// The plugin authenticates with whatever token reaches it, so that token decides who the release
/// is attributed to. `secrets.GITHUB_TOKEN` publishes as github-actions[bot], which is the
/// attribution regression issue #166 removed.
#[test]
fn the_plugin_authenticates_with_the_app_token() {
    let workflow = repo_text(".github/workflows/release.yml");
    assert!(
        workflow.contains("GITHUB_TOKEN: ${{ steps.release-token.outputs.token || secrets.GITHUB_TOKEN }}"),
        "the plugin must receive the app token, falling back to GITHUB_TOKEN only when it is absent"
    );
    assert!(
        !workflow.contains("GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}"),
        "a bare secrets.GITHUB_TOKEN attributes the release to github-actions[bot]"
    );
    // The token has to be generated in the same job as the release step: a version job without it
    // falls back silently, which is exactly how 0.16.0 was published under the wrong identity.
    let version_job = workflow
        .split("\n  version:\n")
        .nth(1)
        .and_then(|rest| rest.split("\n  build:\n").next())
        .expect("release.yml must keep a version job");
    assert!(
        version_job.contains("id: release-token"),
        "the version job must generate the app token the plugin uses"
    );
}

/// The plugin sets the release title from `title_template` when it creates the draft. The
/// promotion used to pass its own `--title`, which is what made that option dead: the release
/// shipped as `Release v0.17.0` while the configuration asked for `cutver 0.17.0`.
#[test]
fn the_promotion_does_not_override_the_title() {
    let workflow = repo_text(".github/workflows/release.yml");
    assert!(
        !workflow.contains("--title \"Release"),
        "the promotion must not pass its own title: cutver.toml owns it through title_template"
    );
    assert!(
        repo_text("cutver.toml").contains("title_template"),
        "the title must be declared where the plugin reads it"
    );
}
