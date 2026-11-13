use super::status::{current_branch, rev_parse};
use super::tags::tag_exists;
use super::types::{Error, TagName, TagPrefix};
use std::path::Path;
use std::process::Command;

pub fn stage(repo: impl AsRef<Path>, paths: &[String], dry_run: bool) -> Result<Option<String>, Error> {
    if paths.is_empty() {
        return Ok(None);
    }
    let mut args = vec!["add".to_string()];
    args.extend(paths.iter().cloned());
    if dry_run {
        return Ok(Some(format!("git add {}", paths.join(" "))));
    }
    let status = Command::new("git")
        .current_dir(&repo)
        .args(&args)
        .status()
        .map_err(|e| Error::Command {
            command: args.join(" "),
            source: e,
        })?;
    if status.success() {
        Ok(None)
    } else {
        Err(Error::Status {
            command: args.join(" "),
            status,
        })
    }
}

pub fn commit(repo: impl AsRef<Path>, message: &str, dry_run: bool) -> Result<Option<String>, Error> {
    commit_ext(repo, message, dry_run, false)
}

pub fn commit_ext(
    repo: impl AsRef<Path>,
    message: &str,
    dry_run: bool,
    allow_empty: bool,
) -> Result<Option<String>, Error> {
    if dry_run {
        let flag = if allow_empty { " --allow-empty" } else { "" };
        return Ok(Some(format!(r#"git commit{flag} -m "{}""#, message)));
    }
    let mut args = vec!["commit"];
    if allow_empty {
        args.push("--allow-empty");
    }
    args.extend(["-m", message]);

    let status = Command::new("git")
        .current_dir(&repo)
        .args(&args)
        .status()
        .map_err(|e| Error::Command {
            command: args.join(" "),
            source: e,
        })?;
    if status.success() {
        Ok(None)
    } else {
        Err(Error::Status {
            command: args.join(" "),
            status,
        })
    }
}

pub fn tag(repo: impl AsRef<Path>, tag_name: &str, version: &str, dry_run: bool) -> Result<Option<String>, Error> {
    let msg = format!("Release {version}");
    if dry_run {
        return Ok(Some(format!(r#"git tag -a {tag_name} -m "{msg}""#)));
    }
    if tag_exists(&repo, tag_name)? {
        let tag_commit = rev_parse(&repo, &format!("{tag_name}^{{commit}}"))?;
        let head = rev_parse(&repo, "HEAD")?;
        if tag_commit == head {
            return Ok(Some(format!("skipped (already points at HEAD): {tag_name}")));
        }
        return Err(Error::Guard {
            detail: format!("tag '{tag_name}' already exists at {tag_commit}, not HEAD ({head})"),
        });
    }
    let status = Command::new("git")
        .current_dir(&repo)
        .args(["tag", "-a", tag_name, "-m", &msg])
        .status()
        .map_err(|e| Error::Command {
            command: "tag".into(),
            source: e,
        })?;
    if status.success() {
        Ok(None)
    } else {
        Err(Error::Status {
            command: "tag".into(),
            status,
        })
    }
}

pub fn push(repo: impl AsRef<Path>, branch: Option<&str>, dry_run: bool) -> Result<Option<String>, Error> {
    let repo = repo.as_ref();
    // A push must name the branch it sends. There is no safe fallback: `HEAD` is not a branch
    // ref, so git rejects it with 'fatal: You are not currently on a branch' after the local
    // commit and tag already exist (#263).
    let branch = match branch {
        Some(branch) => branch.to_string(),
        None => current_branch(repo).map_err(|source| Error::PushBranchUnresolved {
            reason: source.to_string(),
            where_: BRANCH_PUSH_WHERE.to_string(),
        })?,
    };
    run_push(
        repo,
        &branch_push_args(&branch),
        &format!("refs/heads/{branch}"),
        BRANCH_PUSH_WHERE,
        BRANCH_PUSH_FIX,
        dry_run,
    )
}

pub fn push_tag(repo: impl AsRef<Path>, tag_name: &str, dry_run: bool) -> Result<Option<String>, Error> {
    run_push(
        repo.as_ref(),
        &tag_push_args(tag_name),
        &format!("refs/tags/{tag_name}"),
        TAG_PUSH_WHERE,
        TAG_PUSH_FIX,
        dry_run,
    )
}

pub fn push_tag_force(repo: impl AsRef<Path>, tag_name: &str, dry_run: bool) -> Result<Option<String>, Error> {
    run_push(
        repo.as_ref(),
        &tag_force_push_args(tag_name),
        &format!("+refs/tags/{tag_name}:refs/tags/{tag_name}"),
        FLOATING_PUSH_WHERE,
        FLOATING_PUSH_FIX,
        dry_run,
    )
}

// Publishing sends three pushes in dependency order, so each failure names the step it stopped at
// and what the remote already holds.
const BRANCH_PUSH_WHERE: &str = "publish push, step 1 of 3 (branch)";
const TAG_PUSH_WHERE: &str = "publish push, step 2 of 3 (release tag)";
const FLOATING_PUSH_WHERE: &str = "publish push, step 3 of 3 (floating major tag)";

const BRANCH_PUSH_FIX: &str = "nothing was published and the floating major tag did not move; resolve the branch push (ruleset, permissions, or a diverging remote) and re-run.";

const TAG_PUSH_FIX: &str = "the branch is already published; once tag creation is allowed, push the release tag on its own — the floating major tag did not move and no publish command ran.";

const FLOATING_PUSH_FIX: &str = "the branch and release tag are already published; move the floating major tag by hand once the force push is allowed.";

/// The exact `git push` argv for one branch ref.
///
/// No `--tags`: a push sends exactly the refs it names, so a rejected tag can never take a branch
/// update down with it. No `+`: a branch update is never forced.
pub(crate) fn branch_push_args(branch: &str) -> Vec<String> {
    vec!["push".to_string(), "origin".to_string(), format!("refs/heads/{branch}")]
}

/// The exact `git push` argv for one release tag. Names that tag and nothing else.
pub(crate) fn tag_push_args(tag_name: &str) -> Vec<String> {
    vec![
        "push".to_string(),
        "origin".to_string(),
        format!("refs/tags/{tag_name}"),
    ]
}

/// The force refspec, reachable only from the floating-major-tag path and only after the release
/// push succeeded (Pillar II.2).
pub(crate) fn tag_force_push_args(tag_name: &str) -> Vec<String> {
    vec![
        "push".to_string(),
        "origin".to_string(),
        format!("+refs/tags/{tag_name}:refs/tags/{tag_name}"),
    ]
}

/// Runs one push. A dry run reports the command without sending anything.
fn run_push(
    repo: &Path,
    argv: &[String],
    refspec: &str,
    where_: &str,
    fix: &str,
    dry_run: bool,
) -> Result<Option<String>, Error> {
    let command = format!("git {}", argv.join(" "));
    if dry_run {
        return Ok(Some(command));
    }
    let status = Command::new("git")
        .current_dir(repo)
        .args(argv)
        .status()
        .map_err(|source| Error::PushRef {
            refspec: refspec.to_string(),
            detail: format!("the git process could not be started: {source}"),
            where_: where_.to_string(),
            fix: fix.to_string(),
        })?;
    if status.success() {
        Ok(None)
    } else {
        Err(Error::PushRef {
            refspec: refspec.to_string(),
            detail: format!("`{command}` exited with {status}"),
            where_: where_.to_string(),
            fix: fix.to_string(),
        })
    }
}

pub fn update_floating_tag(
    repo: impl AsRef<Path>,
    tag_name: &str,
    target_commit: &str,
    dry_run: bool,
) -> Result<Option<String>, Error> {
    if dry_run {
        return Ok(Some(format!(r#"git tag -f -a {tag_name} -m "{tag_name}""#)));
    }
    let status = Command::new("git")
        .current_dir(&repo)
        .args(["tag", "-f", "-a", tag_name, target_commit, "-m", tag_name])
        .status()
        .map_err(|e| Error::Command {
            command: format!("git tag -f -a {tag_name} {target_commit} -m {tag_name}"),
            source: e,
        })?;
    if status.success() {
        Ok(None)
    } else {
        Err(Error::Status {
            command: format!("git tag -f -a {tag_name} {target_commit} -m {tag_name}"),
            status,
        })
    }
}

pub fn is_floating_major_tag(tag: &str, prefix: Option<&str>) -> bool {
    let p = TagPrefix::new(prefix.unwrap_or_default());
    if let Ok(tag_name) = TagName::parse(tag) {
        tag_name.is_floating_major(&p)
    } else {
        false
    }
}

pub fn commit_message(template: &str, version: &str) -> String {
    template.replace("{version}", version)
}
