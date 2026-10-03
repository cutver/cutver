use std::path::Path;
use std::process::Command;

pub fn remote_url(repo: impl AsRef<Path>) -> Option<String> {
    let output = Command::new("git")
        .current_dir(repo)
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(normalize_repo_url(trimmed))
}

pub fn normalize_repo_url(url: &str) -> String {
    let trimmed = url.trim();
    let mut normalized = if let Some(stripped) = trimmed.strip_prefix("git@") {
        if let Some((host, path)) = stripped.split_once(':') {
            format!("https://{host}/{path}")
        } else {
            trimmed.to_string()
        }
    } else if let Some(stripped) = trimmed.strip_prefix("ssh://git@") {
        format!("https://{stripped}")
    } else {
        trimmed.to_string()
    };
    if let Some(stripped) = normalized.strip_suffix(".git") {
        normalized = stripped.to_string();
    }
    normalized
}
