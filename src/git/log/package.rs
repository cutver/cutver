//! Per-package commit selection by declared manifest path (S4, S5, S6, S7, S8, S12, S13, S15).
//!
//! Everything that turns declared manifest paths into path-limited `git log` queries lives here,
//! together with the [`AnchoredPathspec`] that keeps a declared directory name literal and the
//! unowned-remainder derivation the renderer consumes.

use super::{range_args, run_log_checked};
use crate::git::types::{Error, RawCommit};
use std::collections::HashSet;
use std::path::Path;

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
/// **declaration order** (S5) and must be able to tell that a declared package selected zero
/// commits, rather than being unable to distinguish "no commits" from "missing key" (S6). An
/// entry with an empty `commits` slice is exactly that: the package exists and owns nothing in
/// this range, so the renderer omits its heading instead of emitting an empty section. The name
/// is carried in [`PackageCommits`] so ordering and identity never depend on a positional side
/// channel.
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
///
/// The range itself is selected once, by the caller. [`raw_commits_between`] walks the plain range
/// with no pathspec, so git applies no path-based history simplification; each package query is
/// path-limited and therefore pushes `--full-history` (see [`package_query_args`]) to keep a merge
/// that brought a package change into the mainline. That difference is why the two queries exist
/// at all, and why the plain range is the caller's to reuse rather than something this function
/// re-derives when it already has it.
pub fn commits_by_package(
    repo: impl AsRef<Path>,
    from_tag: Option<&str>,
    to_ref: &str,
    packages: &[(&str, &Path)],
) -> Result<PackageCommitSelection, Error> {
    commits_by_package_with_range(repo, from_tag, to_ref, packages, None)
}

/// [`commits_by_package`] when the plain range is already in hand.
///
/// Passing `range` consumes the caller's selection instead of running a second `git log` over the
/// same range, so the changelog pipeline has one source of truth for the release range and a
/// caller that narrows or reorders it is not silently overridden.
pub fn commits_by_package_with_range(
    repo: impl AsRef<Path>,
    from_tag: Option<&str>,
    to_ref: &str,
    packages: &[(&str, &Path)],
    range: Option<&[RawCommit]>,
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
    let owned_range = match range {
        Some(commits) => commits.to_vec(),
        None => run_log_checked(repo, &range_args(from_tag, to_ref))?,
    };
    let unowned = owned_range
        .into_iter()
        .filter(|c| !owned.contains(c.hash.as_str()))
        .collect();

    Ok(PackageCommitSelection {
        packages: selected,
        unowned,
    })
}

/// One package's `git log` arguments: the range, then the package's literal pathspec and one
/// exclusion per declared directory nested under it (feature doc L7).
pub(super) fn package_query_args(
    from_tag: Option<&str>,
    to_ref: &str,
    dirs: &[AnchoredPathspec],
    idx: usize,
) -> Vec<String> {
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
    crate::config::ManifestPath::rebase_onto_repo(repo, declared).ok_or_else(|| {
        let canonical = |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        Error::Guard {
            detail: format!(
                "declared manifest path '{}' is outside repository '{}'",
                canonical(declared).display(),
                canonical(repo).display()
            ),
        }
    })
}
