use std::path::Path;
use std::process::Command;
use thiserror::Error;

use crate::git;

#[derive(Debug, Error)]
pub enum OpenError {
    #[error("failed to determine repository remote URL: ensure a git origin remote is configured")]
    MissingRemoteUrl,
    #[error("failed to resolve tag for target '{target}': {reason}")]
    TagNotFound { target: String, reason: String },
    #[error("failed to launch browser command '{browser}': {source}")]
    BrowserLaunch {
        browser: String,
        #[source]
        source: std::io::Error,
    },
    #[error("browser command '{browser}' exited with failure code: {code:?}")]
    BrowserExit { browser: String, code: Option<i32> },
}

/// Target to open in the browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenTarget {
    /// Open the repository forge root (or releases page if available).
    Repo,
    /// Open a specific release tag/version page.
    Release(String),
    /// Open a comparison view between tags/revisions.
    Compare { base: Option<String>, head: String },
}

impl OpenTarget {
    /// Parse target argument string into `OpenTarget`.
    pub fn parse(raw: Option<&str>) -> Self {
        let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
            return Self::Repo;
        };

        if s.eq_ignore_ascii_case("compare") {
            return Self::Compare {
                base: None,
                head: "HEAD".to_string(),
            };
        }

        if let Some((base, head)) = s.split_once("...") {
            return Self::Compare {
                base: Some(base.trim().to_string()),
                head: head.trim().to_string(),
            };
        }

        Self::Release(s.to_string())
    }
}

/// Resolve the browser target URL purely from repo remote and target.
pub fn resolve_target_url(remote_url: &str, target: &OpenTarget, tag_prefix: &str) -> Result<String, OpenError> {
    let base = remote_url.trim().trim_end_matches('/');
    let is_gitlab = base.contains("gitlab.com") || base.contains("/-/");

    let prefix = crate::git::TagPrefix::new(tag_prefix);

    match target {
        OpenTarget::Repo => Ok(base.to_string()),
        OpenTarget::Release(ver) => {
            let tag = prefix.format_tag(ver);
            let url = if is_gitlab {
                format!("{base}/-/releases/{tag}")
            } else {
                format!("{base}/releases/tag/{tag}")
            };
            Ok(url)
        }
        OpenTarget::Compare { base: b_opt, head } => {
            let head_tag = prefix.format_tag(head);
            let url = match b_opt {
                Some(b) => {
                    let base_tag = prefix.format_tag(b);
                    if is_gitlab {
                        format!("{base}/-/compare/{base_tag}...{head_tag}")
                    } else {
                        format!("{base}/compare/{base_tag}...{head_tag}")
                    }
                }
                None => {
                    if is_gitlab {
                        format!("{base}/-/compare?to={head_tag}")
                    } else {
                        format!("{base}/compare/{head_tag}")
                    }
                }
            };
            Ok(url)
        }
    }
}

/// Returns the platform default browser launcher program and arguments.
pub fn default_browser_command() -> (&'static str, &'static [&'static str]) {
    #[cfg(target_os = "macos")]
    {
        ("open", &[])
    }
    #[cfg(target_os = "windows")]
    {
        ("cmd", &["/c", "start", ""])
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if is_wsl() && has_wslview() {
            ("wslview", &[])
        } else {
            ("xdg-open", &[])
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn is_wsl() -> bool {
    std::env::var_os("WSL_DISTRO_NAME").is_some()
        || std::fs::read_to_string("/proc/version")
            .map(|v| v.to_ascii_lowercase().contains("microsoft"))
            .unwrap_or(false)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn has_wslview() -> bool {
    Path::new("/usr/sbin/wslview").exists()
        || Path::new("/bin/wslview").exists()
        || Path::new("/usr/bin/wslview").exists()
}

/// Resolves the browser command and arguments honoring overrides and environment.
pub fn resolve_browser_command(browser_override: Option<&str>) -> (String, Vec<String>) {
    if let Some(custom) = browser_override {
        return (custom.to_string(), Vec::new());
    }

    if let Ok(env_browser) = std::env::var("BROWSER") {
        let trimmed = env_browser.trim();
        if !trimmed.is_empty() {
            return (trimmed.to_string(), Vec::new());
        }
    }

    let (prog, args) = default_browser_command();
    (prog.to_string(), args.iter().map(|&s| s.to_string()).collect())
}

/// Execute browser opening command purely separated from URL logic.
pub fn launch_browser(url: &str, browser_override: Option<&str>) -> Result<(), OpenError> {
    let (browser_name, args) = resolve_browser_command(browser_override);
    let mut cmd = Command::new(&browser_name);
    for arg in &args {
        cmd.arg(arg);
    }
    cmd.arg(url);

    let status = cmd.status().map_err(|e| OpenError::BrowserLaunch {
        browser: browser_name.clone(),
        source: e,
    })?;

    if !status.success() {
        return Err(OpenError::BrowserExit {
            browser: browser_name,
            code: status.code(),
        });
    }

    Ok(())
}

/// Resolves URL from repository directory and target.
pub fn resolve_url_for_repo(repo_dir: &Path, target: &OpenTarget, tag_prefix: &str) -> Result<String, OpenError> {
    let remote = git::remote_url(repo_dir).ok_or(OpenError::MissingRemoteUrl)?;
    resolve_target_url(&remote, target, tag_prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_open_targets() {
        assert_eq!(OpenTarget::parse(None), OpenTarget::Repo);
        assert_eq!(OpenTarget::parse(Some("")), OpenTarget::Repo);
        assert_eq!(OpenTarget::parse(Some("1.2.3")), OpenTarget::Release("1.2.3".into()));
        assert_eq!(OpenTarget::parse(Some("v1.2.3")), OpenTarget::Release("v1.2.3".into()));
        assert_eq!(
            OpenTarget::parse(Some("compare")),
            OpenTarget::Compare {
                base: None,
                head: "HEAD".into()
            }
        );
        assert_eq!(
            OpenTarget::parse(Some("v1.0.0...v1.1.0")),
            OpenTarget::Compare {
                base: Some("v1.0.0".into()),
                head: "v1.1.0".into()
            }
        );
    }

    #[test]
    fn test_resolve_github_urls() {
        let remote = "https://github.com/Row0902/cutver";
        assert_eq!(
            resolve_target_url(remote, &OpenTarget::Repo, "v").unwrap(),
            "https://github.com/Row0902/cutver"
        );
        assert_eq!(
            resolve_target_url(remote, &OpenTarget::Release("1.2.3".into()), "v").unwrap(),
            "https://github.com/Row0902/cutver/releases/tag/v1.2.3"
        );
        assert_eq!(
            resolve_target_url(remote, &OpenTarget::Release("v1.2.3".into()), "v").unwrap(),
            "https://github.com/Row0902/cutver/releases/tag/v1.2.3"
        );
        assert_eq!(
            resolve_target_url(
                remote,
                &OpenTarget::Compare {
                    base: Some("v1.0.0".into()),
                    head: "v1.1.0".into()
                },
                "v"
            )
            .unwrap(),
            "https://github.com/Row0902/cutver/compare/v1.0.0...v1.1.0"
        );
        assert_eq!(
            resolve_target_url(
                remote,
                &OpenTarget::Compare {
                    base: None,
                    head: "HEAD".into()
                },
                "v"
            )
            .unwrap(),
            "https://github.com/Row0902/cutver/compare/HEAD"
        );
    }

    #[test]
    fn test_resolve_gitlab_urls() {
        let remote = "https://gitlab.com/group/cutver";
        assert_eq!(
            resolve_target_url(remote, &OpenTarget::Repo, "v").unwrap(),
            "https://gitlab.com/group/cutver"
        );
        assert_eq!(
            resolve_target_url(remote, &OpenTarget::Release("1.2.3".into()), "v").unwrap(),
            "https://gitlab.com/group/cutver/-/releases/v1.2.3"
        );
        assert_eq!(
            resolve_target_url(
                remote,
                &OpenTarget::Compare {
                    base: Some("v1.0.0".into()),
                    head: "v1.1.0".into()
                },
                "v"
            )
            .unwrap(),
            "https://gitlab.com/group/cutver/-/compare/v1.0.0...v1.1.0"
        );
    }

    #[test]
    fn test_resolve_browser_command() {
        let (cmd, args) = resolve_browser_command(Some("my-browser"));
        assert_eq!(cmd, "my-browser");
        assert!(args.is_empty());

        unsafe {
            std::env::set_var("BROWSER", "custom-env-browser");
        }
        let (cmd, args) = resolve_browser_command(None);
        assert_eq!(cmd, "custom-env-browser");
        assert!(args.is_empty());
        unsafe {
            std::env::remove_var("BROWSER");
        }
    }
}
