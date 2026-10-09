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

/// Lists the deduplicated authors of the commits reachable from `from_tag` up to `to_ref`.
///
/// Legacy contract: when `from_tag` is `None` and the `git log` invocation fails, this returns
/// `Ok(vec![])` instead of propagating the error. Callers depend on that suppression, so it is
/// preserved here while the shared extraction helper below reports every failure honestly.
pub fn list_authors_between(dir: &Path, from_tag: Option<&str>, to_ref: &str) -> io::Result<Vec<String>> {
    match list_authors_from_ref(dir, from_tag, to_ref) {
        Ok(authors) => Ok(authors),
        Err(_) if from_tag.is_none() => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// Lists the deduplicated authors of every commit reachable from `before_ref`.
pub fn list_authors_before(dir: &Path, before_ref: &str) -> io::Result<Vec<String>> {
    list_authors_from_ref(dir, None, before_ref)
}

/// Computes which of `contributors` are making their first contribution to the repository.
///
/// Fail-safe direction: a first release (`prev_tag` is `None`) genuinely has no prior history, so
/// every contributor is new. If the prior history is unreadable this propagates the error so
/// callers can announce **nobody** as new while telling the user why -- falsely branding a
/// long-time contributor as a newcomer is a worse failure than omitting the section.
pub fn first_time_contributors(dir: &Path, prev_tag: Option<&str>, contributors: &[String]) -> io::Result<Vec<String>> {
    let Some(before) = prev_tag else {
        return Ok(contributors.to_vec());
    };
    let prior = list_authors_before(dir, before)?;
    Ok(contributors.iter().filter(|c| !prior.contains(c)).cloned().collect())
}

/// Shared `git log` author extraction. Propagates every failure -- spawn, status and decode -- as
/// an error so callers can choose their own suppression policy.
fn list_authors_from_ref(dir: &Path, from_tag: Option<&str>, to_ref: &str) -> io::Result<Vec<String>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    fn run(dir: &Path, args: &[&str]) {
        assert!(
            Command::new("git")
                .current_dir(dir)
                .args(args)
                .status()
                .expect("git should run")
                .success(),
            "git command failed: {args:?}"
        );
    }

    fn repo_with_authors() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        crate::git::init_test_repo(dir.path());

        fs::write(dir.path().join("f"), "1").unwrap();
        run(dir.path(), &["add", "f"]);
        run(
            dir.path(),
            &["commit", "-m", "init", "--author=Alice <alice@example.com>", "-q"],
        );
        run(dir.path(), &["tag", "v1"]);

        fs::write(dir.path().join("f"), "2").unwrap();
        run(
            dir.path(),
            &["commit", "-am", "bob work", "--author=Bob <bob@example.com>", "-q"],
        );

        fs::write(dir.path().join("f"), "3").unwrap();
        run(
            dir.path(),
            &[
                "commit",
                "-am",
                "alice again",
                "--author=Alice <alice@example.com>",
                "-q",
            ],
        );

        dir
    }

    #[test]
    fn list_authors_before_dedups_and_errors_on_bad_ref() {
        let dir = repo_with_authors();
        let path = dir.path();

        // Only the commit reachable from `v1` counts, and it is Alice's.
        assert_eq!(list_authors_before(path, "v1").unwrap(), vec!["Alice".to_string()]);

        // HEAD reachability contains Alice twice; the result is deduplicated.
        assert_eq!(
            list_authors_before(path, "HEAD").unwrap(),
            vec!["Alice".to_string(), "Bob".to_string()]
        );

        // An unreadable ref must surface as an error, never as a silent empty list.
        assert!(list_authors_before(path, "no-such-ref").is_err());
    }

    #[test]
    fn list_authors_between_keeps_legacy_none_suppression() {
        let dir = repo_with_authors();
        let path = dir.path();

        // The legacy contract: `None` + a failing ref yields an empty list, not an error.
        assert_eq!(
            list_authors_between(path, None, "no-such-ref").unwrap(),
            Vec::<String>::new()
        );

        // A concrete `from_tag` still propagates the failure.
        assert!(list_authors_between(path, Some("v1"), "no-such-ref").is_err());
    }

    #[test]
    fn first_time_contributors_filters_and_propagates_unreadable_history() {
        let dir = repo_with_authors();
        let path = dir.path();
        let contributors = vec!["Alice".to_string(), "Bob".to_string(), "Carol".to_string()];

        // No prior tag: a genuine first release, so everyone is new.
        assert_eq!(
            first_time_contributors(path, None, &contributors).unwrap(),
            contributors.clone()
        );

        // `v1` is reachable history: Alice predates it, Bob and Carol do not.
        assert_eq!(
            first_time_contributors(path, Some("v1"), &contributors).unwrap(),
            vec!["Bob".to_string(), "Carol".to_string()]
        );

        // Unreadable history surfaces as an error, never as a silent empty list.
        assert!(first_time_contributors(path, Some("no-such-ref"), &contributors).is_err());
    }
}
