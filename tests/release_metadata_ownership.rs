//! Contract tests for #260: one owner for the release, one definition per artifact.
//!
//! `release-assets.yml` was a second path that built the same binaries, uploaded them and rewrote
//! the release title and body that the `github-publish` plugin had authored. It ran no smoke job,
//! so what it shipped had never executed. The file is deleted; these tests keep a future workflow
//! from taking the release metadata back from the plugin, and from re-deriving the packaging steps
//! a second time.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_text(path: &Path) -> String {
    let raw = std::fs::read_to_string(path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    // Git checks these files out with CRLF on Windows; line endings are a checkout setting.
    raw.replace("\r\n", "\n")
}

fn workflow_files() -> Vec<PathBuf> {
    let dir = repo_root().join(".github").join("workflows");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("failed to list {}: {err}", dir.display()))
        .map(|entry| entry.expect("a readable directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "yml"))
        .collect();
    files.sort();
    assert!(
        files
            .iter()
            .any(|path| path.file_name().is_some_and(|name| name == "release.yml")),
        "the release workflow must exist: every assertion here is relative to it"
    );
    files
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .expect("a workflow file has a name")
        .to_string_lossy()
        .to_string()
}

/// #260 (acceptance 4): no workflow other than `release.yml` sets a release title or body.
///
/// The plugin renders the entry and the host emits the version heading; a workflow that writes
/// either gives the release a second owner, which is how `title_template` once came out dead.
#[test]
fn only_the_release_workflow_owns_the_release_metadata() {
    for path in workflow_files() {
        let name = file_name(&path);
        if name == "release.yml" {
            continue;
        }
        let text = repo_text(&path);
        for (number, line) in text.lines().enumerate() {
            let code = line.trim();
            if code.starts_with('#') {
                continue;
            }
            for marker in [
                "action-gh-release",
                "body_path:",
                "gh release create",
                "gh release edit",
            ] {
                assert!(
                    !code.contains(marker),
                    ".github/workflows/{name}:{} sets or uploads release metadata with `{marker}`: \
                     the plugin owns the release, and a second writer rewrites what it rendered",
                    number + 1
                );
            }
        }
    }
}

/// #260 (acceptance 1, 2, 3): the backfill path is gone, so the packaging steps exist once.
///
/// The AC counts `Package (unix)` as one definition; the honest count is one definition *per
/// artifact matrix* inside `release.yml` — the CLI and the plugin build. What must not come back is
/// a second workflow carrying its own copy, because that copy has no smoke and rewrites the release.
#[test]
fn the_packaging_steps_live_only_in_the_release_workflow() {
    assert!(
        !repo_root()
            .join(".github")
            .join("workflows")
            .join("release-assets.yml")
            .exists(),
        "`release-assets.yml` must stay deleted: it built and uploaded binaries that no smoke job \
         ever executed, and rewrote the title and body the plugin authored"
    );

    let mut in_release = 0;
    for path in workflow_files() {
        let name = file_name(&path);
        let packaged = repo_text(&path)
            .lines()
            .filter(|line| line.contains("Package (unix)"))
            .count();
        if name == "release.yml" {
            in_release = packaged;
            continue;
        }
        assert_eq!(
            packaged, 0,
            ".github/workflows/{name} declares its own packaging steps: the release assets have one \
             definition, in `release.yml`, where the smoke jobs live"
        );
    }
    assert!(
        in_release >= 2,
        "`release.yml` must keep one packaging step per artifact matrix (CLI and plugins), found \
         {in_release}: a matrix that lost its packaging step ships no binary"
    );
}
