//! The raw `git log` runner and its range semantics.
//!
//! Everything that turns a `<from>..<to>` range into parsed [`RawCommit`]s lives here: the shared
//! range prefix, the two runner policies, the `\x1f`/`\0` decoder, and the message projections.
//! The other submodules build their queries on top of [`range_args`] and [`run_log_checked`].

use super::super::command::run_git;
use super::super::types::{Error, RawCommit};
use std::io;
use std::path::Path;

pub fn raw_commits_since(repo: impl AsRef<Path>, tag: Option<&str>) -> Result<Vec<RawCommit>, Error> {
    raw_commits_between(repo, tag, "HEAD")
}

pub fn raw_commits_between(
    repo: impl AsRef<Path>,
    from_tag: Option<&str>,
    to_ref: &str,
) -> Result<Vec<RawCommit>, Error> {
    let args = range_args(from_tag, to_ref);
    run_log(repo.as_ref(), &args, from_tag)
}

/// Shared `git log` range prefix: the range selector followed by the raw commit format.
pub(crate) fn range_args(from_tag: Option<&str>, to_ref: &str) -> Vec<String> {
    let mut args = vec!["log".to_string(), "--use-mailmap".to_string()];
    if let Some(t) = from_tag {
        args.push(format!("{t}..{to_ref}"));
    } else if to_ref != "HEAD" {
        args.push(to_ref.to_string());
    }
    args.push("--format=%H%x1f%h%x1f%aN%x1f%aE%x1f%B%x00".to_string());
    args
}

/// Runs one `git log` invocation and parses it, preserving the legacy policy that a failure with
/// no `from_tag` is an empty result rather than an error. Only a failing status is suppressed, and
/// only for that caller-supplied range spelling; [`commits_by_package`] uses the checked variant
/// instead, because a silently empty selection hides a broken query (S15).
fn run_log(repo: &Path, args: &[String], from_tag: Option<&str>) -> Result<Vec<RawCommit>, Error> {
    match run_log_checked(repo, args) {
        Err(Error::Status { .. }) if from_tag.is_none() => Ok(Vec::new()),
        other => other,
    }
}

/// Runs one `git log` invocation and parses it, propagating every spawn, status and decode failure
/// (S15, Contract Pillar III.4).
pub(crate) fn run_log_checked(repo: &Path, args: &[String]) -> Result<Vec<RawCommit>, Error> {
    let str_args: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = run_git(repo, &str_args)?;
    if !output.status.success() {
        return Err(Error::Status {
            command: str_args.join(" "),
            status: output.status,
        });
    }
    let text = String::from_utf8(output.stdout).map_err(|e| Error::Output {
        source: io::Error::new(io::ErrorKind::InvalidData, e),
    })?;
    Ok(parse_raw_commits(&text))
}

fn parse_raw_commits(text: &str) -> Vec<RawCommit> {
    let mut commits = Vec::new();
    for entry in text.split('\0') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let fields: Vec<&str> = entry.split('\x1f').collect();
        if fields.len() >= 5 {
            commits.push(RawCommit {
                hash: fields[0].trim().to_string(),
                short_hash: fields[1].trim().to_string(),
                author_name: fields[2].trim().to_string(),
                author_email: fields[3].trim().to_string(),
                message: fields[4].trim().to_string(),
            });
        }
    }
    commits
}

pub fn commits_since(repo: impl AsRef<Path>, tag: Option<&str>) -> Result<Vec<String>, Error> {
    commits_between(repo, tag, "HEAD")
}

pub fn commits_between(repo: impl AsRef<Path>, from_tag: Option<&str>, to_ref: &str) -> Result<Vec<String>, Error> {
    let raw = raw_commits_between(repo, from_tag, to_ref)?;
    Ok(raw.into_iter().map(|c| c.message).filter(|m| !m.is_empty()).collect())
}
