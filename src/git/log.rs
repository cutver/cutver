use super::command::run_git;
use super::types::{Error, RawCommit};
use std::collections::HashSet;
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
    let args = range_args(from_tag, to_ref);
    run_log(repo.as_ref(), &args, from_tag)
}

/// One declared package's slice of a commit range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageCommits {
    /// The declared package identity, echoed back so callers never re-associate by index.
    pub package: String,
    /// Commits owned by this package, in `git log` order (newest first).
    pub commits: Vec<RawCommit>,
}

/// Path-filtered commit selection for a whole workspace.
///
/// Chosen over a `HashMap<String, Vec<RawCommit>>` because the renderer must preserve
/// **declaration order** (S5) and must see a declared package with zero commits as an empty
/// section rather than a missing key (S6): an ordered `Vec` with one entry per declared package
/// expresses both, while the name is carried in [`PackageCommits`] so ordering and identity never
/// depend on a positional side channel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageCommitSelection {
    /// One entry per declared package, in declaration order.
    pub packages: Vec<PackageCommits>,
    /// Commits that touch no declared manifest path; never dropped (S7).
    pub unowned: Vec<RawCommit>,
}

/// A repository-anchored directory that reaches `git` as a **literal** pathspec (S12).
///
/// A bare argument is parsed by git as a pathspec *pattern*: a leading `:` is read as pathspec
/// magic, and `*`, `?`, `[` and `\` are read as wildcards. A package declared inside `:pkg`
/// therefore selected a sibling package's commits instead of its own before this type existed.
/// The `:(top,literal)` prefix keeps every metacharacter of the declared name literal and anchors
/// the match to the repository root, so the result never depends on the process working
/// directory. Parsing happens once, at construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchoredPathspec {
    components: Vec<String>,
}

impl AnchoredPathspec {
    /// Parses a repository-relative directory. An empty or `.`-only spelling is the repository
    /// root, whose pathspec is the empty pattern after the magic prefix (the whole tree).
    pub fn parse(dir: &str) -> Result<Self, Error> {
        let mut components = Vec::new();
        for component in dir.split('/') {
            match component {
                "" | "." => {}
                ".." => {
                    return Err(Error::Guard {
                        detail: format!("package directory '{dir}' escapes the repository"),
                    });
                }
                name => components.push(name.to_string()),
            }
        }
        Ok(Self { components })
    }

    /// Pathspec argument selecting this directory and everything below it.
    pub fn selector(&self) -> String {
        format!(":(top,literal){}", self.components.join("/"))
    }

    /// Pathspec argument removing this directory from a selection.
    pub fn exclusion(&self) -> String {
        format!(":(exclude,top,literal){}", self.components.join("/"))
    }

    /// True when `candidate` is a strict descendant of this directory, comparing whole path
    /// components so `crates/ab` is never treated as nested under `crates/a`.
    pub fn is_strictly_nested(&self, candidate: &Self) -> bool {
        candidate.components.len() > self.components.len()
            && candidate.components[..self.components.len()] == self.components[..]
    }
}

/// Selects commits by declared package path (S4, S8).
///
/// Ownership is resolved per changed file: a file belongs to the package whose declared manifest
/// path is the longest prefix of it. Each package is queried with a single `git log <range> --`
/// invocation whose pathspecs are the package directory, anchored as a literal pathspec by
/// [`AnchoredPathspec`] (S12), plus `:(exclude)` entries for every declared path nested under it,
/// so a commit under `crates/a/sub` never leaks into the section of a package declared at
/// `crates/a` (feature doc L7). Commits touching files of two sibling packages appear once in
/// each.
///
/// The unowned remainder is derived **always**, in-process, as the plain range minus every commit
/// hash any declared package claimed, so it means exactly "returned by no declared package" (S7).
/// A commit touching a declared path and an unowned path is therefore rendered once, under its
/// package, never again under the unowned section. Deriving it unconditionally is what keeps a
/// commit that changes no path at all -- a release-trigger commit on an empty tree -- in the
/// selection when a root-level package is declared, since a path-limited query cannot return it
/// and no other package claims it (S13).
///
/// Declared paths are turned into repo-relative pathspecs from their own value; absolute declared
/// paths are rebased onto the repository root, never onto the current working directory.
///
/// A failing `git log` surfaces as an error instead of an empty section or a fall-through into the
/// unowned remainder (S15, Contract Pillar III.4). This differs from [`raw_commits_between`],
/// whose documented `from_tag: None` suppression is preserved for its own callers.
pub fn commits_by_package(
    repo: impl AsRef<Path>,
    from_tag: Option<&str>,
    to_ref: &str,
    packages: &[(&str, &Path)],
) -> Result<PackageCommitSelection, Error> {
    let repo = repo.as_ref();
    let dirs: Vec<AnchoredPathspec> = packages
        .iter()
        .map(|(_, declared)| package_dir(repo, declared))
        .collect::<Result<_, _>>()?;

    let mut selected = Vec::with_capacity(packages.len());
    for (idx, (name, _)) in packages.iter().enumerate() {
        let args = package_query_args(from_tag, to_ref, &dirs, idx);
        selected.push(PackageCommits {
            package: (*name).to_string(),
            commits: run_log_checked(repo, &args)?,
        });
    }

    let owned: HashSet<&str> = selected
        .iter()
        .flat_map(|p| p.commits.iter().map(|c| c.hash.as_str()))
        .collect();
    let range = run_log_checked(repo, &range_args(from_tag, to_ref))?;
    let unowned = range.into_iter().filter(|c| !owned.contains(c.hash.as_str())).collect();

    Ok(PackageCommitSelection {
        packages: selected,
        unowned,
    })
}

/// One package's `git log` arguments: the range, then the package's literal pathspec and one
/// exclusion per declared directory nested under it (feature doc L7).
fn package_query_args(from_tag: Option<&str>, to_ref: &str, dirs: &[AnchoredPathspec], idx: usize) -> Vec<String> {
    let mut args = range_args(from_tag, to_ref);
    // A path-limited `git log` simplifies history by default and drops merges that are
    // TREESAME to one parent, so `--full-history` is required to keep a merge that brought a
    // package change into the mainline.
    args.push("--full-history".to_string());
    args.push("--".to_string());
    args.push(dirs[idx].selector());
    args.extend(
        dirs.iter()
            .enumerate()
            .filter(|(other, dir)| *other != idx && dirs[idx].is_strictly_nested(dir))
            .map(|(_, dir)| dir.exclusion()),
    );
    args
}

/// Shared `git log` range prefix: the range selector followed by the raw commit format.
fn range_args(from_tag: Option<&str>, to_ref: &str) -> Vec<String> {
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
fn run_log_checked(repo: &Path, args: &[String]) -> Result<Vec<RawCommit>, Error> {
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

/// The anchored pathspec of the directory that owns a declared manifest path.
fn package_dir(repo: &Path, declared: &Path) -> Result<AnchoredPathspec, Error> {
    let relative = repo_relative(repo, declared)?;
    let dir = relative.rsplit_once('/').map_or("", |(dir, _)| dir);
    AnchoredPathspec::parse(dir)
}

/// Rebases a declared manifest path onto the repository root. Relative declared paths are already
/// repo-relative; absolute ones are normalized against the canonicalized repository prefix (see
/// [`ManifestPath::rebase_onto_repo`](crate::config::ManifestPath)), so the resulting pathspec never
/// depends on the process working directory, on a symlinked spelling of the repository, or on the
/// Windows verbatim prefix. A path that genuinely escapes the repository is rejected.
fn repo_relative(repo: &Path, declared: &Path) -> Result<String, Error> {
    crate::config::ManifestPath::rebase_onto_repo(repo, declared).ok_or_else(|| Error::Guard {
        detail: format!(
            "declared manifest path '{}' is outside repository '{}'",
            declared.display(),
            repo.display()
        ),
    })
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
