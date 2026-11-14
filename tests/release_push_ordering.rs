//! Publish ordering against a remote that rejects tag refs.
//!
//! Issue #263: a real release dispatch ran `git push origin main --tags`, GitHub rejected the tag
//! ref, and because both refs travelled in one receive-pack transaction the branch update rolled
//! back with it. The floating `v0` tag had already been force-moved locally and pushed, so it was
//! left pointing at a commit that never reached `main`.
//!
//! A bare remote with a `pre-receive` hook that rejects `refs/tags/*` reproduces that shape
//! locally: the branch push must succeed on its own, the release tag push must then fail, and the
//! floating major tag must not have moved. `pre-receive` also proves the refs arrive one transaction
//! at a time, because a batched push would hand both refs to the hook at once.
//!
//! The hook is a `#!/bin/sh` script, which git runs through its own shell on every supported
//! platform, so this test is not unix-gated.

mod common;

use std::fs;
use std::path::Path;

use common::*;
use cutver::bump::run as bump_run;
use cutver::semver_bump::Bump;

const TOML: &str = r#"[version]
current_source = "package.json"

[[manifest]]
path = "package.json"
kind = "json"
field = "version"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
path = "CHANGELOG.md"

[git]
tag_prefix = "v"
floating_major_tag = true
require_clean_tree = true
require_branch = "main"

[publish]
push = true
commands = ["echo published {version} > published.txt"]
"#;

/// Rejects every tag ref the remote is asked to accept, and accepts everything else.
fn reject_tag_refs(remote: &Path) {
    let hooks = remote.join("hooks");
    fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("pre-receive");
    fs::write(
        &hook,
        "#!/bin/sh\n\
         while read -r old new ref; do\n\
         \x20 case \"$ref\" in\n\
         \x20   refs/tags/*) echo \"rejecting tag ref $ref\" >&2; exit 1 ;;\n\
         \x20 esac\n\
         done\n\
         exit 0\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook, perms).unwrap();
    }
}

fn rev(repo: &Path, reff: &str) -> String {
    let out = run_git(repo, &["rev-parse", reff]);
    assert!(
        out.status.success(),
        "git rev-parse {reff} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn a_rejected_release_tag_push_leaves_the_branch_published_and_the_floating_tag_unmoved() {
    assert!(
        git_available(),
        "git CLI is required for e2e tests but was not found in PATH"
    );
    let guard = FixtureGuard::new("release-push-ordering");
    let fixture = guard.fixture();
    let remote = Fixture::new("remote-rejects-tags");
    run_git_ok(&remote.dir, &["init", "--bare", "-q"]);
    reject_tag_refs(&remote.dir);

    fixture.write("package.json", PACKAGE_JSON);
    fixture.write("Cargo.toml", CARGO_TOML);
    fixture.write("CHANGELOG.md", CHANGELOG_MD);
    fixture.write("cutver.toml", TOML);
    init_git_repo(fixture);
    run_git_ok(&fixture.dir, &["checkout", "-B", "main"]);
    initial_commit(fixture);
    run_git_ok(&fixture.dir, &["remote", "add", "origin", remote.dir.to_str().unwrap()]);
    run_git_ok(&fixture.dir, &["push", "origin", "refs/heads/main"]);

    // The floating major tag already exists locally, on the released commit that is published.
    run_git_ok(&fixture.dir, &["tag", "-a", "v1", "-m", "v1"]);
    let pre_release_commit = rev(&fixture.dir, "HEAD");

    let cfg = common::load_config(guard.fixture());

    // The plan names the same three pushes, in the same order, that the real run sends: the branch,
    // the release tag, then the only forced refspec in the pipeline.
    let dry = bump_run(&cfg, Bump::Minor, true, &[]).unwrap();
    assert_eq!(
        dry.publish_push_command.as_deref(),
        Some(
            "git push origin refs/heads/main && git push origin refs/tags/v1.3.0 && \
             git push origin +refs/tags/v1:refs/tags/v1"
        )
    );
    assert_eq!(dry.tag, "v1.3.0");
    assert_eq!(dry.floating_tag.as_deref(), Some("v1"));

    let err = bump_run(&cfg, Bump::Minor, false, &[]).expect_err("the tag push is rejected by the remote");

    // The failure names the ref that was rejected, not an opaque `push --tags` command, and names
    // the commit and tag that already exist locally (#265).
    let local_release_commit = rev(&fixture.dir, "HEAD");
    match &err {
        cutver::bump::Error::Push { commit, tag, source } => {
            let text = source.to_string();
            assert!(text.contains("refs/tags/v1.3.0"), "unexpected push error: {text}");
            assert_eq!(
                commit, &local_release_commit,
                "the push failure must name the release commit"
            );
            assert_eq!(tag, "v1.3.0", "the push failure must name the release tag");
            assert!(
                err.to_string().contains("Where:") && err.to_string().contains("Fix:"),
                "the push failure must be actionable: {err}"
            );
        }
        other => panic!("expected a push failure, got: {other:?}"),
    }

    // The branch moved on its own, before the tag was attempted.
    assert_eq!(rev(&fixture.dir, "HEAD"), local_release_commit);
    assert_ne!(local_release_commit, pre_release_commit);
    assert_eq!(
        rev(&remote.dir, "refs/heads/main"),
        local_release_commit,
        "the branch push must stand on its own, so a rejected tag cannot roll it back"
    );

    // The rejected tag never reached the remote, and no floating tag moved.
    let remote_refs = run_git(&remote.dir, &["for-each-ref", "--format=%(refname)"]);
    let remote_refs = String::from_utf8_lossy(&remote_refs.stdout);
    assert!(
        !remote_refs.contains("refs/tags/"),
        "the remote must have no tags after the rejection: {remote_refs}"
    );
    assert_eq!(
        rev(&fixture.dir, "v1^{commit}"),
        pre_release_commit,
        "the floating major tag must move only after the release push succeeded"
    );

    // Nothing after the push ran.
    assert!(
        !fixture.dir.join("published.txt").exists(),
        "publish commands must not run when the release push fails"
    );
}
