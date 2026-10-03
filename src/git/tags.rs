use super::command::{run_git, stdout_text};
use super::ops::is_floating_major_tag;
use super::types::Error;
use std::io;
use std::path::Path;

pub fn tag_exists(repo: impl AsRef<Path>, tag_name: &str) -> Result<bool, Error> {
    let output = run_git(&repo, &["tag", "-l", tag_name])?;
    if !output.status.success() {
        return Err(Error::Status {
            command: format!("tag -l {tag_name}"),
            status: output.status,
        });
    }
    Ok(output.stdout.iter().any(|b| !b.is_ascii_whitespace()))
}

pub fn has_remote(repo: impl AsRef<Path>, remote: &str) -> bool {
    let Ok(output) = run_git(&repo, &["remote"]) else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let text = String::from_utf8_lossy_owned(output.stdout);
    text.lines().any(|l| l.trim() == remote)
}

pub fn remote_tag_exists(repo: impl AsRef<Path>, remote: &str, tag_name: &str) -> Result<Option<String>, Error> {
    if !has_remote(&repo, remote) {
        return Ok(None);
    }
    let tag_ref = format!("refs/tags/{tag_name}");
    let peeled_ref = format!("{tag_ref}^{{}}");
    let output = run_git(&repo, &["ls-remote", "--tags", remote, &tag_ref, &peeled_ref])?;
    if !output.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy_owned(output.stdout);
    let mut direct_hash = None;
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let (Some(hash), Some(r)) = (parts.next(), parts.next()) else {
            continue;
        };
        if r == peeled_ref {
            return Ok(Some(hash.to_string()));
        }
        if r == tag_ref {
            direct_hash = Some(hash.to_string());
        }
    }
    Ok(direct_hash)
}

pub fn tag_name(prefix: &str, version: &str) -> String {
    format!("{prefix}{version}")
}

pub fn list_tags(repo: impl AsRef<Path>, tag_prefix: Option<&str>) -> Result<Vec<String>, Error> {
    let output = run_git(&repo, &["tag", "-l"])?;
    let text = stdout_text(output, "tag -l")?;
    let prefix = tag_prefix.filter(|p| !p.is_empty());
    let tags = text
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter(|tag| match prefix {
            Some(p) => tag.starts_with(p),
            None => true,
        })
        .map(String::from)
        .collect();
    Ok(tags)
}

pub fn latest_tag(repo: impl AsRef<Path>, tag_prefix: Option<&str>) -> Result<Option<String>, Error> {
    let mut excluded: Vec<String> = Vec::new();
    loop {
        let mut args = vec!["describe", "--tags", "--abbrev=0"];
        let match_arg;
        if let Some(prefix) = tag_prefix
            && !prefix.is_empty()
        {
            match_arg = format!("{prefix}*");
            args.push("--match");
            args.push(&match_arg);
        }
        for ex in &excluded {
            args.push("--exclude");
            args.push(ex);
        }
        let output = run_git(&repo, &args)?;
        if !output.status.success() {
            return Ok(None);
        }
        let tag = String::from_utf8(output.stdout)
            .map_err(|e| Error::Output {
                source: io::Error::new(io::ErrorKind::InvalidData, e),
            })?
            .trim()
            .to_string();
        if tag.is_empty() {
            return Ok(None);
        }

        if !is_floating_major_tag(&tag, tag_prefix) {
            return Ok(Some(tag));
        }

        // It is a floating major tag. Check if there are other tags pointing to the same commit.
        let target_commit = format!("{tag}^{{commit}}");
        let points_at_output = run_git(&repo, &["tag", "--points-at", &target_commit])?;
        if points_at_output.status.success() {
            let points_at_text = String::from_utf8_lossy_owned(points_at_output.stdout);
            let prefix_opt = tag_prefix.filter(|p| !p.is_empty());
            for candidate in points_at_text.lines().map(str::trim).filter(|s| !s.is_empty()) {
                if !is_floating_major_tag(candidate, tag_prefix) {
                    if let Some(p) = prefix_opt {
                        if candidate.starts_with(p) {
                            return Ok(Some(candidate.to_string()));
                        }
                    } else {
                        return Ok(Some(candidate.to_string()));
                    }
                }
            }
        }

        // Exclude this floating tag and query again
        excluded.push(tag);
    }
}
