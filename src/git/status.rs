use super::command::{run_git, stdout_text};
use super::types::Error;
use std::path::Path;

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

pub fn require_clean_tree(repo: impl AsRef<Path>, require: bool) -> Result<(), Error> {
    if !require {
        return Ok(());
    }
    let text = stdout_text(run_git(&repo, &["status", "--porcelain"])?, "status --porcelain")?;
    if is_clean(&text) {
        Ok(())
    } else {
        Err(Error::Guard {
            detail: format!("working tree has uncommitted changes:\n{text}"),
        })
    }
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
        if line.len() >= 4 {
            let status_code = &line[..2];
            if status_code.starts_with('?') || status_code.starts_with('!') {
                continue;
            }
            let mut file_path = line[3..].trim();
            if file_path.starts_with('"') && file_path.ends_with('"') && file_path.len() >= 2 {
                file_path = &file_path[1..file_path.len() - 1];
            }
            if let Some((_, new_path)) = file_path.split_once(" -> ") {
                file_path = new_path.trim();
            }
            if !file_path.is_empty() {
                files.push(file_path.to_string());
            }
        }
    }
    Ok(files)
}
