use super::command::run_git;
use super::types::{Error, RawCommit};
use std::io;
use std::path::Path;
use std::process::Command;

pub fn raw_commits_since(repo: impl AsRef<Path>, tag: Option<&str>) -> Result<Vec<RawCommit>, Error> {
    raw_commits_between(repo, tag, "HEAD")
}

pub fn raw_commits_between(
    repo: impl AsRef<Path>,
    from_tag: Option<&str>,
    to_ref: &str,
) -> Result<Vec<RawCommit>, Error> {
    let mut args = vec!["log".to_string(), "--use-mailmap".to_string()];
    if let Some(t) = from_tag {
        args.push(format!("{t}..{to_ref}"));
    } else if to_ref != "HEAD" {
        args.push(to_ref.to_string());
    }
    args.push("--format=%H%x1f%h%x1f%aN%x1f%aE%x1f%B%x00".to_string());
    let str_args: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = run_git(&repo, &str_args)?;
    if !output.status.success() {
        if from_tag.is_none() {
            return Ok(Vec::new());
        }
        return Err(Error::Status {
            command: str_args.join(" "),
            status: output.status,
        });
    }
    let text = String::from_utf8(output.stdout).map_err(|e| Error::Output {
        source: io::Error::new(io::ErrorKind::InvalidData, e),
    })?;

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
    Ok(commits)
}

pub fn commits_since(repo: impl AsRef<Path>, tag: Option<&str>) -> Result<Vec<String>, Error> {
    commits_between(repo, tag, "HEAD")
}

pub fn commits_between(repo: impl AsRef<Path>, from_tag: Option<&str>, to_ref: &str) -> Result<Vec<String>, Error> {
    let raw = raw_commits_between(repo, from_tag, to_ref)?;
    Ok(raw.into_iter().map(|c| c.message).filter(|m| !m.is_empty()).collect())
}

pub fn resolve_author(name: &str, email: &str) -> String {
    let email = email.trim().trim_matches(|c| c == '<' || c == '>');
    let domain = "@users.noreply.github.com";
    if email.len() >= domain.len() && email[email.len() - domain.len()..].eq_ignore_ascii_case(domain) {
        let user_part = &email[..email.len() - domain.len()];
        let handle = match user_part.split_once('+') {
            Some((_, after_plus)) => after_plus,
            None => user_part,
        };
        if !handle.is_empty() {
            return handle.to_string();
        }
    }
    name.trim().to_string()
}

pub fn list_authors_since(dir: &Path, tag: Option<&str>) -> io::Result<Vec<String>> {
    list_authors_between(dir, tag, "HEAD")
}

pub fn list_authors_between(dir: &Path, from_tag: Option<&str>, to_ref: &str) -> io::Result<Vec<String>> {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir).arg("log").arg("--use-mailmap");
    let range;
    if let Some(t) = from_tag {
        range = format!("{t}..{to_ref}");
        cmd.arg(&range);
    } else if to_ref != "HEAD" {
        cmd.arg(to_ref);
    }
    cmd.arg("--format=%aN\t%aE");
    let output = cmd.output()?;
    if !output.status.success() {
        if from_tag.is_none() {
            return Ok(Vec::new());
        }
        return Err(io::Error::other(format!(
            "git log failed with status {}",
            output.status
        )));
    }
    let text = String::from_utf8_lossy_owned(output.stdout);
    let mut seen = std::collections::HashSet::new();
    let mut authors = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (name, email) = match line.split_once('\t') {
            Some((n, e)) => (n, e),
            None => (line, ""),
        };
        let author = resolve_author(name, email);
        if !author.is_empty() && seen.insert(author.clone()) {
            authors.push(author);
        }
    }
    Ok(authors)
}
