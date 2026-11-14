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

/// Fails closed on any change except an untracked file this invocation declared as its own input.
///
/// `cutver publish --notes-file RELEASE_NOTES.md` is handed a file the release workflow rendered
/// inside the checkout, so refusing it made the command reject its own argument (#302). The
/// exemption is exactly that narrow: only an untracked entry is eligible, and only when it names one
/// of `declared_inputs` after both are canonicalized, so `RELEASE_NOTES.md`, `./RELEASE_NOTES.md`, an
/// absolute spelling and a path reached through a symlink are one file. A tracked file — modified,
/// staged or deleted — always aborts, as does any other untracked entry.
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
    let mut remaining = String::new();
    for line in porcelain.lines() {
        if is_untracked(line) && is_declared(repo, entry_path(line), &declared) {
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

fn is_declared(repo: &Path, entry: &str, declared: &[PathBuf]) -> bool {
    canonical_in(repo, Path::new(entry)).is_some_and(|path| declared.contains(&path))
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
