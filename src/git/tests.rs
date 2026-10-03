use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::log::{commits_since, list_authors_since, raw_commits_since, resolve_author};
use super::ops::{
    commit, commit_ext, commit_message, is_floating_major_tag, push, push_tag_force, stage, tag, update_floating_tag,
};
use super::remote::normalize_repo_url;
use super::status::{current_branch, is_clean, parse_branch, rev_parse, status_files};
use super::tags::{latest_tag, list_tags, remote_tag_exists, tag_exists, tag_name};
use super::types::{CommitSha, RawCommit, TagName, TagPrefix};

static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn tmp_id(prefix: &str) -> String {
    let n = TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("{}-{}-{}", prefix, std::process::id(), n)
}

pub fn init_test_repo(dir: impl AsRef<Path>) {
    let dir = dir.as_ref();
    let run = |args: &[&str]| {
        let status = Command::new("git")
            .current_dir(dir)
            .args(args)
            .status()
            .expect("failed to execute git");
        assert!(status.success(), "git command failed: {:?}", args);
    };
    run(&["init"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test User"]);
    run(&["config", "commit.gpgsign", "false"]);
    run(&["config", "tag.gpgsign", "false"]);
}

fn tmp_repo() -> PathBuf {
    let dir = std::env::temp_dir().join(tmp_id("cutver-git"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    for a in [
        &["init", "-q"] as &[&str],
        &["config", "user.email", "t@e.com"],
        &["config", "user.name", "T"],
        &["config", "commit.gpgsign", "false"],
        &["config", "tag.gpgsign", "false"],
    ] {
        assert!(
            Command::new("git")
                .current_dir(&dir)
                .args(a)
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(dir.join("x"), "a").unwrap();
    git(&dir, &["add", "x"]);
    git(&dir, &["commit", "-m", "i", "-q"]);
    dir
}

fn git(repo: &PathBuf, args: &[&str]) {
    assert!(
        Command::new("git")
            .current_dir(repo)
            .args(args)
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn commit_message_and_tag_name_build() {
    assert_eq!(
        commit_message("chore(release): v{version}", "1.2.3"),
        "chore(release): v1.2.3"
    );
    assert_eq!(tag_name("v", "1.2.3"), "v1.2.3");
    assert_eq!(tag_name("", "1.2.3"), "1.2.3");
}

#[test]
fn is_clean_detects_dirty() {
    assert!(is_clean("") && is_clean("   \n") && !is_clean(" M src/main.rs") && !is_clean("?? src/main.rs"));
}

#[test]
fn parse_branch_trims_newline() {
    assert!(parse_branch("main\n") == "main" && parse_branch("feature/x") == "feature/x");
}

#[test]
fn stage_dry_run_reports_command_and_empty_returns_none() {
    let report = stage(".", &["a.txt".into(), "b.txt".into()], true).unwrap().unwrap();
    assert!(report.starts_with("git add") && report.contains("a.txt") && report.contains("b.txt"));
    assert!(stage(".", &[], true).unwrap().is_none());
}

#[test]
fn commit_and_tag_dry_run_report_commands() {
    assert_eq!(
        commit(".", "chore: v1.0.0", true).unwrap().unwrap(),
        r#"git commit -m "chore: v1.0.0""#
    );
    assert_eq!(
        commit_ext(".", "chore: v1.0.0", true, true).unwrap().unwrap(),
        r#"git commit --allow-empty -m "chore: v1.0.0""#
    );
    assert_eq!(
        tag(".", "v1.0.0", "1.0.0", true).unwrap().unwrap(),
        r#"git tag -a v1.0.0 -m "Release 1.0.0""#
    );
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn tag_detects_existing_and_skips_at_head() {
    let dir = tmp_repo();
    git(&dir, &["tag", "v1"]);
    assert!(tag_exists(&dir, "v1").unwrap() && !tag_exists(&dir, "v2").unwrap());
    assert_eq!(
        tag(&dir, "v1", "1", false).unwrap(),
        Some("skipped (already points at HEAD): v1".into())
    );
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn tag_errors_when_points_elsewhere() {
    let dir = tmp_repo();
    let first = rev_parse(&dir, "HEAD").unwrap();
    fs::write(dir.join("x"), "b").unwrap();
    git(&dir, &["commit", "-am", "c2", "-q"]);
    git(&dir, &["tag", "v1", &first]);
    assert!(tag(&dir, "v1", "1", false).is_err());
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn test_list_tags_filtered_and_unfiltered() {
    let dir = tmp_repo();
    assert_eq!(list_tags(&dir, None).unwrap(), Vec::<String>::new());
    assert_eq!(list_tags(&dir, Some("v")).unwrap(), Vec::<String>::new());

    git(&dir, &["tag", "v1.0.0"]);
    git(&dir, &["tag", "v1.1.0"]);
    git(&dir, &["tag", "2.0.0"]);
    git(&dir, &["tag", "release/v0.1.0"]);

    let all = list_tags(&dir, None).unwrap();
    assert_eq!(all, vec!["2.0.0", "release/v0.1.0", "v1.0.0", "v1.1.0"]);

    let all_empty_prefix = list_tags(&dir, Some("")).unwrap();
    assert_eq!(all_empty_prefix, vec!["2.0.0", "release/v0.1.0", "v1.0.0", "v1.1.0"]);

    let v_tags = list_tags(&dir, Some("v")).unwrap();
    assert_eq!(v_tags, vec!["v1.0.0", "v1.1.0"]);

    let rel_tags = list_tags(&dir, Some("release/")).unwrap();
    assert_eq!(rel_tags, vec!["release/v0.1.0"]);

    let none_matching = list_tags(&dir, Some("not-found")).unwrap();
    assert!(none_matching.is_empty());
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn latest_tag_and_commits_since() {
    let dir = tmp_repo();
    // Initially no tags
    assert_eq!(latest_tag(&dir, None).unwrap(), None);
    assert_eq!(latest_tag(&dir, Some("v")).unwrap(), None);

    // All commits since initial
    let all_commits = commits_since(&dir, None).unwrap();
    assert_eq!(all_commits, vec!["i"]);

    // Tag initial commit as v1.0.0
    git(&dir, &["tag", "v1.0.0"]);
    assert_eq!(latest_tag(&dir, None).unwrap(), Some("v1.0.0".to_string()));
    assert_eq!(latest_tag(&dir, Some("v")).unwrap(), Some("v1.0.0".to_string()));
    assert_eq!(latest_tag(&dir, Some("release/")).unwrap(), None);

    // No commits since v1.0.0 yet
    let since_v1 = commits_since(&dir, Some("v1.0.0")).unwrap();
    assert!(since_v1.is_empty());

    // Add a commit with multiline message
    fs::write(dir.join("x"), "c2").unwrap();
    git(
        &dir,
        &["commit", "-am", "feat: new feature\n\nDetailed explanation", "-q"],
    );

    // Add another commit
    fs::write(dir.join("x"), "c3").unwrap();
    git(&dir, &["commit", "-am", "fix: small bug", "-q"]);

    let new_commits = commits_since(&dir, Some("v1.0.0")).unwrap();
    assert_eq!(new_commits.len(), 2);
    assert_eq!(new_commits[0], "fix: small bug");
    assert_eq!(new_commits[1], "feat: new feature\n\nDetailed explanation");

    let raw_commits = raw_commits_since(&dir, Some("v1.0.0")).unwrap();
    assert_eq!(raw_commits.len(), 2);
    assert_eq!(raw_commits[0].message, "fix: small bug");
    assert_eq!(raw_commits[0].author_name, "T");
    assert_eq!(raw_commits[0].author_email, "t@e.com");
    assert_eq!(raw_commits[0].hash.len(), 40);
    assert_eq!(raw_commits[0].short_hash.len(), 7);
    assert_eq!(raw_commits[1].message, "feat: new feature\n\nDetailed explanation");
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn current_branch_resolves_and_detects_detached_head() {
    let dir = tmp_repo();
    let branch = current_branch(&dir).unwrap();
    assert!(!branch.is_empty());
    git(&dir, &["checkout", "--detach", "HEAD", "-q"]);
    assert!(current_branch(&dir).is_err());
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn push_dry_run_formatting() {
    let dir = tmp_repo();
    let branch = current_branch(&dir).unwrap();

    assert_eq!(
        push(&dir, Some("main"), true, true).unwrap(),
        Some("git push origin main --tags".into())
    );
    assert_eq!(
        push(&dir, None, true, true).unwrap(),
        Some(format!("git push origin {branch} --tags"))
    );
    assert_eq!(
        push(&dir, Some("main"), false, true).unwrap(),
        Some("git push origin main".into())
    );
    assert_eq!(
        push(&dir, None, false, true).unwrap(),
        Some(format!("git push origin {branch}"))
    );

    // Detached HEAD falls back to HEAD
    git(&dir, &["checkout", "--detach", "HEAD", "-q"]);
    assert_eq!(
        push(&dir, None, true, true).unwrap(),
        Some("git push origin HEAD --tags".into())
    );
    assert_eq!(
        push(&dir, None, false, true).unwrap(),
        Some("git push origin HEAD".into())
    );
}

#[test]
fn push_dry_run_non_git_repo_falls_back_to_head() {
    assert_eq!(
        push(Path::new("/nonexistent-dir-cutver"), None, true, true).unwrap(),
        Some("git push origin HEAD --tags".into())
    );
    assert_eq!(
        push(Path::new("/nonexistent-dir-cutver"), Some("main"), true, true).unwrap(),
        Some("git push origin main --tags".into())
    );
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn push_executes_to_remote() {
    let dir = tmp_repo();
    let remote = std::env::temp_dir().join(tmp_id("cutver-git-remote"));
    let _ = fs::remove_dir_all(&remote);
    fs::create_dir_all(&remote).unwrap();
    assert!(
        Command::new("git")
            .current_dir(&remote)
            .args(["init", "--bare", "-q"])
            .status()
            .unwrap()
            .success()
    );
    git(&dir, &["remote", "add", "origin", remote.to_str().unwrap()]);
    git(&dir, &["tag", "v1.0.0"]);

    assert_eq!(push(&dir, None, true, false).unwrap(), None);

    let out = Command::new("git")
        .current_dir(&remote)
        .args(["tag", "-l", "v1.0.0"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "v1.0.0");

    let branch = current_branch(&dir).unwrap();
    let out_branch = Command::new("git")
        .current_dir(&remote)
        .args(["branch", "-l", &branch])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out_branch.stdout).contains(&branch));

    let _ = fs::remove_dir_all(&remote);
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn status_files_detects_changes_and_ignores_untracked() {
    let dir = tmp_repo();
    assert!(status_files(&dir).unwrap().is_empty());
    fs::write(dir.join("x"), "modified").unwrap();
    fs::write(dir.join("y.txt"), "untracked").unwrap();
    let files = status_files(&dir).unwrap();
    assert!(files.contains(&"x".to_string()));
    // Untracked files must NEVER be returned
    assert!(!files.contains(&"y.txt".to_string()));
}

#[test]
fn test_normalize_repo_url() {
    assert_eq!(
        normalize_repo_url("git@github.com:Row0902/cutver.git"),
        "https://github.com/Row0902/cutver"
    );
    assert_eq!(
        normalize_repo_url("git@gitlab.com:org/sub/repo.git"),
        "https://gitlab.com/org/sub/repo"
    );
    assert_eq!(
        normalize_repo_url("https://github.com/Row0902/cutver.git"),
        "https://github.com/Row0902/cutver"
    );
    assert_eq!(
        normalize_repo_url("https://github.com/Row0902/cutver"),
        "https://github.com/Row0902/cutver"
    );
}

#[test]
fn test_resolve_author() {
    assert_eq!(
        resolve_author("Rowell Urbaez Reyes", "167712855+Row0902@users.noreply.github.com"),
        "Row0902"
    );
    assert_eq!(resolve_author("Rowell", "Row0902@users.noreply.github.com"), "Row0902");
    assert_eq!(resolve_author("Alice Smith", "alice@example.com"), "Alice Smith");
    assert_eq!(
        resolve_author("Bob", "12345+octocat@USERS.NOREPLY.GITHUB.COM"),
        "octocat"
    );
    assert_eq!(resolve_author("Fallback", "@users.noreply.github.com"), "Fallback");
    assert_eq!(
        resolve_author("Fallback", "12345+@users.noreply.github.com"),
        "Fallback"
    );
    assert_eq!(
        resolve_author("Charlie", "<167712855+Row0902@users.noreply.github.com>"),
        "Row0902"
    );
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn test_list_authors_since_and_between() {
    let dir = tmp_repo();
    let authors = list_authors_since(&dir, None).unwrap();
    assert_eq!(authors, vec!["T".to_string()]);

    git(&dir, &["tag", "v1.0.0"]);
    let authors_since = list_authors_since(&dir, Some("v1.0.0")).unwrap();
    assert!(authors_since.is_empty());

    // New commit by another author
    fs::write(dir.join("x"), "c2").unwrap();
    Command::new("git")
        .current_dir(&dir)
        .args(["commit", "-am", "c2", "--author=Alice <a@e.com>", "-q"])
        .status()
        .unwrap();

    let authors_new = list_authors_since(&dir, Some("v1.0.0")).unwrap();
    assert_eq!(authors_new, vec!["Alice".to_string()]);

    // Commit with GitHub noreply email (with ID+)
    fs::write(dir.join("x"), "c3").unwrap();
    Command::new("git")
        .current_dir(&dir)
        .args([
            "commit",
            "-am",
            "c3",
            "--author=Full Name <167712855+Row0902@users.noreply.github.com>",
            "-q",
        ])
        .status()
        .unwrap();

    // Commit with GitHub noreply email (without ID+)
    fs::write(dir.join("x"), "c4").unwrap();
    Command::new("git")
        .current_dir(&dir)
        .args([
            "commit",
            "-am",
            "c4",
            "--author=Octo Cat <octocat@users.noreply.github.com>",
            "-q",
        ])
        .status()
        .unwrap();

    // Commit with normal email to be mapped by .mailmap
    fs::write(dir.join("x"), "c5").unwrap();
    Command::new("git")
        .current_dir(&dir)
        .args(["commit", "-am", "c5", "--author=Old Name <mapped@e.com>", "-q"])
        .status()
        .unwrap();

    // Add .mailmap mapping Old Name to New Name
    fs::write(dir.join(".mailmap"), "New Name <mapped@e.com>\n").unwrap();

    // Another commit by Row0902 to test deduplication
    fs::write(dir.join("x"), "c6").unwrap();
    Command::new("git")
        .current_dir(&dir)
        .args([
            "commit",
            "-am",
            "c6",
            "--author=Row0902 <167712855+Row0902@users.noreply.github.com>",
            "-q",
        ])
        .status()
        .unwrap();

    let authors_after = list_authors_since(&dir, Some("v1.0.0")).unwrap();
    assert_eq!(
        authors_after,
        vec![
            "Row0902".to_string(),
            "New Name".to_string(),
            "octocat".to_string(),
            "Alice".to_string(),
        ]
    );
}

#[test]
fn test_tag_name_and_prefix_domain_newtypes() {
    use std::str::FromStr;

    // Valid tag names
    let tag = TagName::parse("v1.2.3").unwrap();
    assert_eq!(tag.as_str(), "v1.2.3");
    assert_eq!(&*tag, "v1.2.3");
    assert_eq!(tag.as_ref(), "v1.2.3");
    assert_eq!(tag.to_string(), "v1.2.3");
    assert_eq!(TagName::from_str("release/v2.0.0").unwrap().as_str(), "release/v2.0.0");
    assert_eq!(TagName::try_from("v1.0.0".to_string()).unwrap().as_str(), "v1.0.0");

    // Invalid tag names
    assert!(TagName::parse("").is_err());
    assert!(TagName::parse("   ").is_err());
    assert!(TagName::parse("tag with spaces").is_err());
    assert!(TagName::parse("tag\twith\ttab").is_err());
    assert!(TagName::parse("tag\nwith\nnewline").is_err());
    assert!(TagName::parse("tag\x00null").is_err());
    assert!(TagName::parse("tag..name").is_err());
    assert!(TagName::parse("/starts/slash").is_err());
    assert!(TagName::parse("ends/slash/").is_err());
    assert!(TagName::parse("tag.lock").is_err());
    assert!(TagName::parse("tag@{upstream}").is_err());
    assert!(TagName::parse("tag~1").is_err());
    assert!(TagName::parse("tag^2").is_err());
    assert!(TagName::parse("tag:colon").is_err());
    assert!(TagName::parse("tag?question").is_err());
    assert!(TagName::parse("tag*star").is_err());
    assert!(TagName::parse("tag[bracket").is_err());
    assert!(TagName::parse("tag\\backslash").is_err());
    assert!(TagName::parse("@").is_err());
    assert!(TagName::parse("tag.").is_err());

    // TagPrefix formatting & methods
    let prefix_v = TagPrefix::new("v");
    assert_eq!(prefix_v.as_str(), "v");
    assert_eq!(&*prefix_v, "v");
    assert_eq!(prefix_v.as_ref(), "v");
    assert_eq!(prefix_v.to_string(), "v");

    let empty_prefix = TagPrefix::default();
    assert_eq!(empty_prefix.as_str(), "");

    let formatted = prefix_v.format_tag("1.2.3");
    assert_eq!(formatted.as_str(), "v1.2.3");

    let rel_prefix = TagPrefix::new("release/");
    let rel_tag = rel_prefix.format_tag("1.0.0");
    assert_eq!(rel_tag.as_str(), "release/1.0.0");

    // TagName::strip_prefix
    assert_eq!(formatted.strip_prefix(&prefix_v), Some("1.2.3"));
    assert_eq!(formatted.strip_prefix(&empty_prefix), Some("v1.2.3"));
    assert_eq!(formatted.strip_prefix(&rel_prefix), None);

    // TagName::normalize_version
    assert_eq!(formatted.normalize_version(&prefix_v), "1.2.3");
    assert_eq!(
        TagName::parse("v1.2.3").unwrap().normalize_version(&empty_prefix),
        "1.2.3"
    );
    assert_eq!(
        TagName::parse("V1.2.3").unwrap().normalize_version(&empty_prefix),
        "1.2.3"
    );
    assert_eq!(
        TagName::parse("1.2.3").unwrap().normalize_version(&empty_prefix),
        "1.2.3"
    );
    assert_eq!(rel_tag.normalize_version(&rel_prefix), "1.0.0");
    let rel_v_tag = rel_prefix.format_tag("v1.0.0");
    assert_eq!(rel_v_tag.normalize_version(&rel_prefix), "1.0.0");

    // TagName::is_floating_major
    assert!(TagName::parse("v1").unwrap().is_floating_major(&prefix_v));
    assert!(TagName::parse("v10").unwrap().is_floating_major(&prefix_v));
    assert!(!TagName::parse("v1.0.0").unwrap().is_floating_major(&prefix_v));
    assert!(!TagName::parse("v1.2").unwrap().is_floating_major(&prefix_v));
    assert!(!TagName::parse("app-v1").unwrap().is_floating_major(&prefix_v));

    let app_prefix = TagPrefix::new("app-");
    assert!(TagName::parse("app-1").unwrap().is_floating_major(&app_prefix));
    assert!(!TagName::parse("app-1.0.0").unwrap().is_floating_major(&app_prefix));

    assert!(TagName::parse("v1").unwrap().is_floating_major(&empty_prefix));
    assert!(TagName::parse("V2").unwrap().is_floating_major(&empty_prefix));
    assert!(TagName::parse("1").unwrap().is_floating_major(&empty_prefix));
    assert!(!TagName::parse("1.0.0").unwrap().is_floating_major(&empty_prefix));
}

#[test]
fn test_is_floating_major_tag() {
    assert!(is_floating_major_tag("v1", Some("v")));
    assert!(is_floating_major_tag("v2", Some("v")));
    assert!(is_floating_major_tag("v10", Some("v")));
    assert!(!is_floating_major_tag("v1.0.0", Some("v")));
    assert!(!is_floating_major_tag("v1.2", Some("v")));
    assert!(!is_floating_major_tag("app-v1", Some("v")));

    // Custom prefix
    assert!(is_floating_major_tag("app-1", Some("app-")));
    assert!(!is_floating_major_tag("app-1.0.0", Some("app-")));
    assert!(!is_floating_major_tag("v1", Some("app-")));

    // Empty prefix or None
    assert!(is_floating_major_tag("v1", None));
    assert!(is_floating_major_tag("V2", None));
    assert!(is_floating_major_tag("1", None));
    assert!(!is_floating_major_tag("1.0.0", None));
    assert!(is_floating_major_tag("v1", Some("")));
    assert!(is_floating_major_tag("1", Some("")));
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn test_latest_tag_ignores_floating_tags() {
    let dir = tmp_repo();
    // Create initial tag v1.0.0
    git(&dir, &["tag", "v1.0.0"]);
    // Create floating tag v1 pointing to same commit
    git(&dir, &["tag", "v1"]);
    // latest_tag should resolve v1.0.0, not v1
    assert_eq!(latest_tag(&dir, Some("v")).unwrap(), Some("v1.0.0".to_string()));

    // Make another commit and tag v1.1.0
    fs::write(dir.join("x"), "update").unwrap();
    git(&dir, &["commit", "-am", "second", "-q"]);
    git(&dir, &["tag", "v1.1.0"]);
    // Update floating tag v1 to point to this new commit
    git(&dir, &["tag", "-f", "v1"]);

    // latest_tag should resolve v1.1.0, not v1
    assert_eq!(latest_tag(&dir, Some("v")).unwrap(), Some("v1.1.0".to_string()));

    // When only a floating tag exists and no semver tag
    let dir2 = tmp_repo();
    git(&dir2, &["tag", "v1"]);
    assert_eq!(latest_tag(&dir2, Some("v")).unwrap(), None);
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn test_update_floating_tag_and_push_tag_force() {
    let dir = tmp_repo();
    // dry run
    let cmd = update_floating_tag(&dir, "v1", "HEAD", true).unwrap().unwrap();
    assert_eq!(cmd, r#"git tag -f -a v1 -m "v1""#);

    // live run
    update_floating_tag(&dir, "v1", "HEAD", false).unwrap();
    assert!(tag_exists(&dir, "v1").unwrap());

    // push_tag_force dry run
    let push_cmd = push_tag_force(&dir, "v1", true).unwrap().unwrap();
    assert_eq!(push_cmd, "git push origin +refs/tags/v1:refs/tags/v1");
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn test_remote_tag_exists_handles_missing_remote_and_tags() {
    let dir = tmp_repo();
    // No remotes configured
    assert_eq!(remote_tag_exists(&dir, "origin", "v1.0.0").unwrap(), None);

    // Remote bare repo setup
    let remote = std::env::temp_dir().join(tmp_id("cutver-git-remote-tag"));
    let _ = fs::remove_dir_all(&remote);
    fs::create_dir_all(&remote).unwrap();
    assert!(
        Command::new("git")
            .current_dir(&remote)
            .args(["init", "--bare", "-q"])
            .status()
            .unwrap()
            .success()
    );
    git(&dir, &["remote", "add", "origin", remote.to_str().unwrap()]);

    // Tag does not exist yet
    assert_eq!(remote_tag_exists(&dir, "origin", "v1.0.0").unwrap(), None);

    // Push lightweight tag to remote
    git(&dir, &["tag", "v1.0.0"]);
    git(&dir, &["push", "origin", "v1.0.0"]);
    let commit_hash = rev_parse(&dir, "v1.0.0").unwrap();

    // Delete tag locally to verify it detects it remotely even when absent locally
    git(&dir, &["tag", "-d", "v1.0.0"]);
    assert!(!tag_exists(&dir, "v1.0.0").unwrap());

    let remote_commit = remote_tag_exists(&dir, "origin", "v1.0.0").unwrap();
    assert_eq!(remote_commit, Some(commit_hash.clone()));

    // Also test annotated tag with peeling ^{}
    git(&dir, &["tag", "-a", "v2.0.0", "-m", "release v2.0.0"]);
    git(&dir, &["push", "origin", "v2.0.0"]);
    let v2_commit = rev_parse(&dir, "v2.0.0^{commit}").unwrap();
    git(&dir, &["tag", "-d", "v2.0.0"]);

    let remote_v2_commit = remote_tag_exists(&dir, "origin", "v2.0.0").unwrap();
    assert_eq!(remote_v2_commit, Some(v2_commit));

    let _ = fs::remove_dir_all(&remote);
}

#[test]
fn test_commit_sha_domain_newtype() {
    use std::str::FromStr;

    // Valid 40-character SHA-1
    let sha40 = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";
    let commit_sha40 = CommitSha::parse(sha40).unwrap();
    assert_eq!(commit_sha40.as_str(), sha40);
    assert_eq!(&*commit_sha40, sha40);
    assert_eq!(commit_sha40.as_ref(), sha40);
    assert_eq!(commit_sha40.to_string(), sha40);
    assert_eq!(commit_sha40.short(), "4b825dc");

    // Case normalization to lowercase
    let upper40 = "4B825DC642CB6EB9A060E54BF8D69288FBEE4904";
    let parsed_upper = CommitSha::parse(upper40).unwrap();
    assert_eq!(parsed_upper.as_str(), sha40);

    // Valid 64-character SHA-256
    let sha64 = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let commit_sha64 = CommitSha::parse(sha64).unwrap();
    assert_eq!(commit_sha64.as_str(), sha64);
    assert_eq!(commit_sha64.short(), "0123456");

    // FromStr and TryFrom
    assert_eq!(CommitSha::from_str(sha40).unwrap().as_str(), sha40);
    assert_eq!(CommitSha::try_from(sha40).unwrap().as_str(), sha40);
    assert_eq!(CommitSha::try_from(sha40.to_string()).unwrap().as_str(), sha40);

    // Invalid length
    assert!(CommitSha::parse("").is_err());
    assert!(CommitSha::parse("4b825dc").is_err());
    assert!(CommitSha::parse("a".repeat(39)).is_err());
    assert!(CommitSha::parse("a".repeat(41)).is_err());
    assert!(CommitSha::parse("a".repeat(63)).is_err());
    assert!(CommitSha::parse("a".repeat(65)).is_err());

    // Invalid characters
    let mut invalid_char = sha40.to_string();
    invalid_char.replace_range(0..1, "g");
    assert!(CommitSha::parse(&invalid_char).is_err());

    let mut space_char = sha40.to_string();
    space_char.replace_range(0..1, " ");
    assert!(CommitSha::parse(&space_char).is_err());

    // Interop with RawCommit
    let raw = RawCommit {
        hash: sha40.to_string(),
        short_hash: "4b825dc".to_string(),
        author_name: "Author".to_string(),
        author_email: "author@example.com".to_string(),
        message: "feat: something".to_string(),
    };
    assert_eq!(raw.commit_sha().unwrap().as_str(), sha40);

    let raw_invalid = RawCommit {
        hash: "invalid-hash".to_string(),
        short_hash: "invalid".to_string(),
        author_name: "Author".to_string(),
        author_email: "author@example.com".to_string(),
        message: "feat: something".to_string(),
    };
    assert!(raw_invalid.commit_sha().is_err());
}
