use super::command::{run_git, stdout_text};
use super::types::Error;
use std::path::{Path, PathBuf};

pub fn rev_parse(repo: impl AsRef<Path>, rev: &str) -> Result<String, Error> {
    let text = stdout_text(run_git(&repo, &["rev-parse", rev])?, &format!("rev-parse {rev}"))?;
    Ok(text.trim().to_string())
}

pub fn is_clean(porcelain: &str) -> bool {
    porcelain.trim().is_empty()
}

pub fn parse_branch(output: &str) -> &str {
    output.trim()
}

/// Fails closed on any change in the working tree.
///
/// The inline phase of a release declares no input of its own, so it exempts nothing (#302).
pub fn require_clean_tree(repo: impl AsRef<Path>, require: bool) -> Result<(), Error> {
    require_clean_tree_except(repo, require, &[])
}

/// Fails closed on any change except an untracked entry this invocation declared as its own input.
///
/// `cutver publish --notes-file RELEASE_NOTES.md` is handed a file the release workflow rendered
/// inside the checkout, so refusing it made the command reject its own argument (#302). The
/// exemption is exactly that narrow: only an untracked entry is eligible, and only when it names one
/// of `declared_inputs` after both are canonicalized, so `RELEASE_NOTES.md`, `./RELEASE_NOTES.md`, an
/// absolute spelling and a path reached through a symlink are one file. A tracked file — modified,
/// staged or deleted — always aborts, as does any other untracked entry.
///
/// A declared input that is a **directory** covers the untracked entries it holds, and nothing
/// else. `git status` reports an untracked tree as its topmost entry (`?? .cutver/`), which no file
/// path can match, so a directory this invocation produced would otherwise refuse the command it is
/// an input to (#326 S10). Such an entry is exempt only when everything under it is a declared
/// input, and only when it is a directory strictly inside the repository, so a declared `.` cannot
/// whitelist the whole worktree and a scratch file beside the input is not exempt either. When the
/// directory also holds a tracked file git lists the untracked files one by one instead, and each
/// of those is exempt for living inside the declared directory.
pub fn require_clean_tree_except(
    repo: impl AsRef<Path>,
    require: bool,
    declared_inputs: &[PathBuf],
) -> Result<(), Error> {
    if !require {
        return Ok(());
    }
    let repo = repo.as_ref();
    let text = stdout_text(run_git(repo, &["status", "--porcelain"])?, "status --porcelain")?;
    let remaining = undeclared_changes(repo, &text, declared_inputs);
    if is_clean(&remaining) {
        Ok(())
    } else {
        Err(Error::Guard {
            detail: format!("working tree has uncommitted changes:\n{remaining}"),
        })
    }
}

/// The porcelain entries left once the inputs this run declared are removed, in git's order.
fn undeclared_changes(repo: &Path, porcelain: &str, declared_inputs: &[PathBuf]) -> String {
    if declared_inputs.is_empty() {
        return porcelain.to_string();
    }
    let declared: Vec<PathBuf> = declared_inputs
        .iter()
        .filter_map(|path| canonical_in(repo, path))
        .collect();
    let root = canonical_in(repo, Path::new("."));
    let mut remaining = String::new();
    for line in porcelain.lines() {
        if is_untracked(line) && is_declared(repo, entry_path(line), &declared, root.as_deref()) {
            continue;
        }
        remaining.push_str(line);
        remaining.push('\n');
    }
    remaining
}

fn is_untracked(line: &str) -> bool {
    line.starts_with("??")
}

fn is_declared(repo: &Path, entry: &str, declared: &[PathBuf], root: Option<&Path>) -> bool {
    let Some(path) = canonical_in(repo, Path::new(entry)) else {
        return false;
    };
    // The declared file itself, spelled any way (#302): a symlinked notes file stays exempt, exactly
    // as it was before the directory exemption existed.
    if declared.iter().any(|input| input == &path) {
        return true;
    }
    // Anything a declared directory holds. Both spellings are the same input — git lists the files
    // one by one when the directory also carries a tracked file, and collapses the whole tree when
    // it does not — so both shapes refuse a symlink the same way: a link is a path the declaration
    // never named, and the collapsed shape already refuses one.
    let inside = declared
        .iter()
        .any(|input| is_subtree(input, root) && path.starts_with(input));
    (inside && !is_symlink(repo, entry)) || covers(&path, declared, root)
}

/// Whether the entry git reported is itself a symlink. `symlink_metadata` does not follow the link,
/// so a link is refused instead of being resolved into whatever it points at.
fn is_symlink(repo: &Path, entry: &str) -> bool {
    std::fs::symlink_metadata(repo.join(entry)).is_ok_and(|metadata| metadata.file_type().is_symlink())
}

/// Whether `declared` may exempt a subtree: a directory strictly inside the repository. A declared
/// `.` covers nothing, and neither does a declaration that resolves outside the root.
fn is_subtree(declared: &Path, root: Option<&Path>) -> bool {
    let Some(root) = root else {
        return false;
    };
    declared != root && declared.starts_with(root) && declared.is_dir()
}

/// Whether an untracked `entry` is the tree git collapsed a declared directory into.
///
/// `git status --porcelain` reports an untracked tree as its **topmost** entry: with nothing tracked
/// inside it, the attestation directory arrives as `?? .cutver/` and none of its files is listed.
/// Such an entry is exempt only when everything under it is a declared input, so one scratch file
/// beside the declared directory keeps the whole entry undeclared and the exemption can never widen
/// into a general "untracked directories are fine". A file *inside* the declared directory is
/// exempt instead, through the directory itself: the whole directory is the input.
fn covers(entry: &Path, declared: &[PathBuf], root: Option<&Path>) -> bool {
    if !is_subtree(entry, root) {
        return false;
    }
    if !declared.iter().any(|input| input.starts_with(entry)) {
        return false;
    }
    holds_only_declared(entry, declared)
}

/// Whether every file beneath `dir` is a declared input. Symlinks are not followed and not accepted:
/// a link names a path outside the tree the declaration covers, so it fails closed.
fn holds_only_declared(dir: &Path, declared: &[PathBuf]) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries {
        let Ok(entry) = entry else { return false };
        let Ok(file_type) = entry.file_type() else { return false };
        let path = entry.path();
        if file_type.is_symlink() {
            return false;
        }
        if file_type.is_dir() {
            if !holds_only_declared(&path, declared) {
                return false;
            }
        } else if !declared.iter().any(|input| input == &path || path.starts_with(input)) {
            return false;
        }
    }
    true
}

/// One file, spelled any way: the path is resolved against the repository and canonicalized, so a
/// `./` prefix, an absolute path and a path reached through a symlink all land on the file git
/// reported. A path that cannot be resolved names no file, and so exempts nothing.
fn canonical_in(repo: &Path, path: &Path) -> Option<PathBuf> {
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo.join(path)
    };
    std::fs::canonicalize(full).ok()
}

pub fn current_branch(repo: impl AsRef<Path>) -> Result<String, Error> {
    let output = run_git(&repo, &["symbolic-ref", "--short", "HEAD"])?;
    let text = stdout_text(output, "symbolic-ref --short HEAD")?;
    Ok(parse_branch(&text).to_string())
}

pub fn require_branch(repo: impl AsRef<Path>, expected: Option<&str>) -> Result<(), Error> {
    if let Some(branch) = expected {
        let current = current_branch(&repo)?;
        if current != branch {
            return Err(Error::Guard {
                detail: format!("on branch '{current}', expected '{branch}'"),
            });
        }
    }
    Ok(())
}

pub fn status_files(repo: impl AsRef<Path>) -> Result<Vec<String>, Error> {
    let text = stdout_text(
        run_git(&repo, &["status", "--porcelain", "-uno"])?,
        "status --porcelain -uno",
    )?;
    let mut files = Vec::new();
    for line in text.lines() {
        let status_code = line.get(..2).unwrap_or("");
        if status_code.starts_with('?') || status_code.starts_with('!') {
            continue;
        }
        let file_path = entry_path(line);
        if !file_path.is_empty() {
            files.push(file_path.to_string());
        }
    }
    Ok(files)
}

/// The path one porcelain entry names: the two status columns removed, git's C-style quoting undone
/// and the destination of a rename taken. One parser for every reader in this module.
fn entry_path(line: &str) -> &str {
    if line.len() < 4 {
        return "";
    }
    let mut path = line[3..].trim();
    if path.starts_with('"') && path.ends_with('"') && path.len() >= 2 {
        path = &path[1..path.len() - 1];
    }
    if let Some((_, new_path)) = path.split_once(" -> ") {
        path = new_path.trim();
    }
    path
}
