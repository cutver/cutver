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

pub fn push(
    repo: impl AsRef<Path>,
    branch: Option<&str>,
    include_tags: bool,
    dry_run: bool,
) -> Result<Option<String>, Error> {
    let current = if branch.is_none() {
        current_branch(&repo).unwrap_or_else(|_| "HEAD".to_string())
    } else {
        String::new()
    };
    let target = branch.unwrap_or(&current);
    if dry_run {
        return Ok(Some(
            format!(
                "git push origin {} {}",
                target,
                if include_tags { "--tags" } else { "" }
            )
            .trim()
            .to_string(),
        ));
    }
    let mut args = vec!["push", "origin", target];
    if include_tags {
        args.push("--tags");
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

pub fn push_tag_force(repo: impl AsRef<Path>, tag_name: &str, dry_run: bool) -> Result<Option<String>, Error> {
    let refspec = format!("+refs/tags/{tag_name}:refs/tags/{tag_name}");
    if dry_run {
        return Ok(Some(format!("git push origin {refspec}")));
    }
    let args = ["push", "origin", &refspec];
    let status = Command::new("git")
        .current_dir(&repo)
        .args(args)
        .status()
        .map_err(|e| Error::Command {
            command: format!("git push origin {refspec}"),
            source: e,
        })?;
    if status.success() {
        Ok(None)
    } else {
        Err(Error::Status {
            command: format!("git push origin {refspec}"),
            status,
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
