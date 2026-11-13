//! Contract tests binding `cutver.toml` to `.github/workflows/release.yml`, the workflow that cuts
//! a release: the crate version, the git tag and the draft release.
//!
//! The release workflow cannot be run from a test, but every coupling that would silently break a
//! release is text-visible. Each assertion names the release failure it prevents, so a future edit
//! to either file fails here instead of during a dispatch.
//!
//! The OCI half of the same release is pinned in `container_workflow_contract.rs`.
//!
//! Guards PR #229 (risk item 5: delivery/CI).

use std::path::PathBuf;

use cutver::config;
use cutver::plugin::types::{Capability, RuntimeKind};
use cutver::plugin::{PluginConfig, PluginName};

const PLUGIN: &str = "github-publish";
/// Every job id in the file, in declaration order.
const JOBS: [&str; 6] = ["version", "build", "build-plugins", "smoke", "smoke-plugins", "publish"];
/// The jobs whose green result the publish job waits on: the bump, both builds and both smokes.
const VERIFIED_BEFORE_PUBLISHING: [&str; 5] = ["version", "build", "build-plugins", "smoke", "smoke-plugins"];

fn repo_text(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    // Git checks these files out with CRLF on Windows. Line endings are a checkout setting, not
    // content, so every assertion in this file compares normalised text.
    raw.replace("\r\n", "\n")
}

/// One job body, anchored on its two-space job key so a same-named value elsewhere in the file (a
/// `needs:` entry, a step name, an output) cannot stand in for the job. The last job runs to EOF.
fn job<'a>(workflow: &'a str, id: &str) -> &'a str {
    let marker = format!("\n  {id}:\n");
    let start = workflow
        .find(&marker)
        .unwrap_or_else(|| panic!("release.yml must declare the `{id}` job"))
        + marker.len();
    let rest = &workflow[start..];
    let end = JOBS
        .iter()
        .filter_map(|next| rest.find(&format!("\n  {next}:\n")))
        .min()
        .unwrap_or(rest.len());
    &rest[..end]
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

/// The release must be cut by the CLI built from the commit being released.
///
/// `cutver/setup` resolves `version: latest`, i.e. the previously published binary, so a defect in
/// the sandbox or in the release path could only ship in a release it was unable to cut. #251 did
/// exactly that: 0.17.0 could not read `cutver.toml` inside the plugin sandbox, `github-publish`
/// 0.2.x failed closed, and 0.18.0 was blocked until the pin was rolled back to 0.1.2 by hand.
/// These assertions fail if the workflow goes back to depending on an external binary.
#[test]
fn the_release_is_cut_by_the_cli_built_from_the_released_commit() {
    let workflow = repo_text(".github/workflows/release.yml");
    assert!(
        !workflow.contains("uses: cutver/release@v1"),
        "the release must not run the published action: `cutver/release` resolves `version: latest`, \
         so a fix in the release path could only ship in a release it was unable to cut"
    );

    // Scoped to the job that cuts the release: a literal elsewhere in the file keeps the test green
    // while the mechanism is dead.
    let version = job(&workflow, "version");
    let built = version.find("Build the CLI from this commit").expect("build step");
    let on_path = version.find("Put the built CLI first on PATH").expect("PATH step");
    let restored = version.find("run: cutver plugin restore").expect("plugin restore step");
    let bump = version
        .find("cutver bump \"${{ inputs.bump || 'auto' }}\"")
        .expect("bump step");
    assert!(
        built < on_path && on_path < restored && restored < bump,
        "the plugin-enabled CLI must be built, put on PATH and hydrated with the declared plugins \
         before the bump step runs: every step in between shells out to whatever `cutver` resolves"
    );
    // Later jobs may install a published CLI on purpose: the smoke test proves the artifact a user
    // downloads works. What must never happen is an installed binary *cutting* the release.
    assert!(
        !version[..bump].contains("uses: cutver/setup"),
        "no published CLI may be installed before the release step: the binary that cuts release N must be N's code"
    );
}

/// The plugin only owns the release if it is a draft: its `draft` field defaults to false, and a
/// non-draft release is published the moment `on_post_release` fires, before its assets exist.
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
    let version = job(&workflow, "version");
    assert!(
        version.contains("cargo build --locked --release --features plugins"),
        "the version job must build the plugin-enabled CLI: a lean build fails closed on the \
         [plugins.github-publish] declaration"
    );
    assert!(
        version.contains("if [ \"${{ inputs.dry_run }}\" = \"true\" ]"),
        "the `workflow_dispatch` boolean must drive the flag, or `dry_run: true` still releases"
    );
    assert!(
        version.contains("args+=(--dry-run)"),
        "`--dry-run` must be the branch `inputs.dry_run` controls"
    );
    for key in ["GITHUB_TOKEN:", "GITHUB_REPOSITORY:"] {
        assert!(
            version.contains(key),
            "the bump step must export `{key}`, or the declared permission arrives empty"
        );
    }
}

/// #259: the version job must leave no trace a failed smoke would have to undo. A tag cannot be
/// unpublished, a crate version cannot be reused, and a public release without its assets is
/// exactly what `draft = true` exists to prevent.
#[test]
fn the_version_job_publishes_nothing_irreversible() {
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    for forbidden in ["cutver publish", "cargo publish", "git tag"] {
        assert!(
            !version.contains(forbidden),
            "the version job must not run `{forbidden}`: an irreversible step there means a run whose \
             smoke fails has already published a tag, a crate version or a public release"
        );
    }
    assert!(
        version.contains("--defer-publish"),
        "the bump step must pass `--defer-publish`, or `cutver bump` runs the tag, the registry and \
         `on_post_release` inside the version job again"
    );
    assert!(
        version.contains("id: cutver"),
        "the job outputs must come from the report-reading step, not from the bump step"
    );
}

/// The only job that publishes must run after every verification job, on one exact `needs:` line:
/// a leg added here but left out there runs red while the release goes public anyway.
#[test]
fn the_publish_job_waits_for_every_verification_job() {
    let workflow = repo_text(".github/workflows/release.yml");
    let publish = job(&workflow, "publish");
    let needs = format!("    needs: [{}]", VERIFIED_BEFORE_PUBLISHING.join(", "));
    assert!(
        publish.contains(&format!("\n{needs}\n")),
        "the publish job must declare exactly `{needs}`: a smoke that fails has to leave the tag, \
         the crate and the draft unpublished"
    );
    for id in VERIFIED_BEFORE_PUBLISHING {
        assert!(
            workflow.contains(&format!("\n  {id}:\n")),
            "the publish job needs `{id}`, which must still exist as a job"
        );
    }
    assert!(
        publish.contains("if: needs.version.outputs.released == 'true'"),
        "a dry run must publish nothing: the job stays gated on the reported `released`"
    );
}

/// The tag no longer exists while the build and smoke jobs run, so they must check out the release
/// commit. A `ref: needs.version.outputs.tag` there resolves to nothing and the run dies before the
/// verification it exists to perform.
#[test]
fn the_built_artifacts_come_from_the_release_commit() {
    let workflow = repo_text(".github/workflows/release.yml");
    for id in ["build", "build-plugins", "smoke", "smoke-plugins"] {
        let slice = job(&workflow, id);
        let checkout = slice
            .find("uses: actions/checkout@v7")
            .unwrap_or_else(|| panic!("the `{id}` job must check out"));
        let rest = &slice[checkout..];
        let step = &rest[..rest.find("\n      - ").unwrap_or(rest.len())];
        assert!(
            step.contains("ref: ${{ needs.version.outputs.commit }}"),
            "the `{id}` job must check out the release commit: {step}"
        );
        assert!(
            !step.contains("needs.version.outputs.tag"),
            "the `{id}` job must not check out by tag: that ref does not exist until the publish \
             job creates it: {step}"
        );
    }
}

/// The irreversibility order #259 is about: the tag and the crate first, the assets into the draft,
/// and only then the draft made public. A public release without its assets cannot be undone.
#[test]
fn the_tag_the_crate_and_the_draft_are_published_after_verification() {
    let workflow = repo_text(".github/workflows/release.yml");
    let publish = job(&workflow, "publish");
    let closing = publish.find("cutver publish --tag").expect("the closing phase");
    let assets = publish
        .find("Download all asset artifacts")
        .expect("asset download step");
    let upload = publish
        .find("gh release upload \"$TAG\" dist/* --clobber")
        .expect("upload step");
    let promote = publish
        .find("gh release edit \"$TAG\" --draft=false")
        .expect("promotion step");
    assert!(
        closing < assets && assets < upload && upload < promote,
        "`cutver publish` creates the tag and the crate, the assets land in its draft, and only then \
         is the draft promoted"
    );
    assert!(
        publish.contains("--commit \"${{ needs.version.outputs.commit }}\""),
        "`cutver publish` must name the commit the verification jobs built, or it can tag a commit \
         nobody tested"
    );
    assert!(
        publish.contains("name: release-notes") && publish.contains("--notes-file RELEASE_NOTES.md"),
        "the publish job must download the rendered notes and hand them to `cutver publish`: they \
         are the body `on_post_release` releases, not a second copy of the changelog"
    );
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

/// The version job renders the entry once and saves it as an artifact. The publish job downloads it
/// and hands the path to `cutver publish`, whose `on_post_release` plugin creates the draft from it;
/// the promotion that follows only flips the draft state. Repeating the body there gave the release
/// a second owner — that is how `title_template` once came out dead.
#[test]
fn release_notes_artifact_is_kept_but_does_not_feed_the_promotion() {
    let workflow = repo_text(".github/workflows/release.yml");
    let version = job(&workflow, "version");
    assert!(
        version.contains("name: release-notes\n          path: RELEASE_NOTES.md"),
        "the version job must save RELEASE_NOTES.md as the `release-notes` artifact"
    );
    assert!(
        version.contains("cutver changelog latest --template .github/templates/cutver/RELEASE.md > RELEASE_NOTES.md"),
        "the version job must render the notes from the template cutver.toml names"
    );
    let promote_at = workflow
        .find("gh release edit \"$TAG\" --draft=false")
        .expect("the promotion step");
    let promotion = workflow[promote_at..].split('\n').next().unwrap_or_default();
    assert_eq!(
        promotion, "gh release edit \"$TAG\" --draft=false",
        "the promotion must only publish the draft: a `--title` or a body there overrides the plugin"
    );
}

/// The plugin authenticates with whatever token reaches it, so that token decides who the release
/// is attributed to. `secrets.GITHUB_TOKEN` publishes as github-actions[bot], which is the
/// attribution regression issue #166 removed. Both jobs that hand the plugin a token mint it
/// themselves and fail closed without it: an optional token is how 0.16.0 shipped under the wrong
/// identity, and how the tag push once reached the ruleset unauthenticated.
#[test]
fn the_plugin_authenticates_with_the_app_token() {
    let workflow = repo_text(".github/workflows/release.yml");
    const MANDATORY: &str = "GITHUB_TOKEN: ${{ steps.release-token.outputs.token }}";
    for id in ["version", "publish"] {
        let slice = job(&workflow, id);
        assert!(
            slice.contains("id: release-token"),
            "the `{id}` job must mint the app token it hands the plugin"
        );
        assert!(
            slice.contains(MANDATORY),
            "the `{id}` job must hand the plugin `{MANDATORY}`, with no fallback"
        );
    }
    assert!(
        !workflow.contains("GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}"),
        "a bare secrets.GITHUB_TOKEN attributes the release to github-actions[bot]"
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

/// Delivery (S1/S2): the bump push must authenticate as the app, inside the version job.
///
/// The ruleset requires `CI gate` on `main` and the bump commit is created in this same job, so the
/// push can never carry a check run: it has to authenticate as the one bypass actor, the
/// `cutver-release` app. The token is mandatory, not opportunistic: an empty token or a fallback to
/// `secrets.GITHUB_TOKEN` only moves the failure to the push, where the ruleset rejects it after the
/// tag has already landed. Scoped to the job, because a literal elsewhere in the file keeps the test
/// green while the mechanism is dead.
#[test]
fn the_version_job_checkout_persists_the_app_token_in_that_job() {
    const LITERAL: &str = "token: ${{ steps.release-token.outputs.token }}";
    let workflow = repo_text(".github/workflows/release.yml");
    let start = workflow.find("\n  version:\n").expect("the version job must exist");
    let end = workflow[start + 1..]
        .find("\n  build:\n")
        .map(|i| start + 1 + i)
        .expect("the build job marks the end of the version job");
    let job = &workflow[start..end];

    let token_at = job
        .find("id: release-token")
        .expect("the version job must mint the app token");
    let checkout_at = job
        .find("uses: actions/checkout@v7")
        .expect("the version job must check out");
    assert!(
        token_at < checkout_at,
        "the app token must be minted before the version job's checkout"
    );

    // The minting step must not be skippable or tolerated when it fails: a soft failure leaves the
    // token empty, which is how the push used to arrive at the ruleset unauthenticated.
    let token_step_start = job[..token_at]
        .rfind("\n      - ")
        .map(|i| i + 1)
        .expect("the token minting step must start with a step marker");
    let token_step_rest = &job[token_step_start..];
    let token_step = &token_step_rest[..token_step_rest.find("\n      - ").unwrap_or(token_step_rest.len())];
    assert!(
        token_step.contains("uses: actions/create-github-app-token"),
        "`id: release-token` must belong to the app-token step: on any other step the output is empty and the checkout silently falls back: {token_step}"
    );
    assert!(
        !token_step.contains("continue-on-error"),
        "the token step must fail the job when the app credentials are missing, not continue: {token_step}"
    );

    let after = &job[checkout_at..];
    let block = &after[..after.find("\n      - ").unwrap_or(after.len())];
    assert!(
        block.contains(LITERAL),
        "the version job's own checkout must pass `{LITERAL}`: {block}"
    );
    assert!(
        !block.contains("secrets.GITHUB_TOKEN"),
        "the version job's checkout must not fall back to secrets.GITHUB_TOKEN: the ruleset only lets the app push, so the fallback hides the failure until the push is rejected: {block}"
    );
    assert!(
        !block.contains("persist-credentials: false"),
        "`persist-credentials: false` removes the credential `git push origin main --tags` needs"
    );

    // The guard is what turns a late, confusing rejection into an early actionable error: with a
    // guard-less step an empty token reaches the checkout and the run fails only at the push, after
    // the commit exists and the tag has landed.
    let guard_at = job
        .find("name: Require the release bot token")
        .expect("the version job must guard the app token before it builds");
    let guard_rest = &job[guard_at..];
    let guard = &guard_rest[..guard_rest.find("\n      - ").unwrap_or(guard_rest.len())];
    assert!(
        guard.contains("exit 1"),
        "the guard must fail the job when the token is empty: {guard}"
    );
}
