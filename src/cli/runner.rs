use std::path::{Path, PathBuf};

use crate::bump::{self, Summary};
use crate::changelog::ReleaseContext;
use crate::cli::args::{BumpLevel, ChangelogCommands, Cli, Commands};
use crate::cli::changelog::{self, ChangelogFormat};
use crate::cli::style::Theme;
use crate::config;

pub fn print_error(msg: impl std::fmt::Display) {
    let theme = Theme::stderr();
    eprintln!("{} {}", theme.error("Error:"), msg);
}

pub fn run(args: Cli) -> i32 {
    match args.command {
        Commands::Init {
            update,
            force,
            path,
            no_template,
        } => match crate::init::run_init(path, update, force, no_template) {
            Ok(()) => 0,
            Err(e) => {
                print_error(e);
                1
            }
        },
        Commands::Changelog { command } => run_changelog(args.config.as_deref(), command),
        Commands::Doctor { check_changelog } => {
            let config = match load_config(args.config) {
                Ok(c) => c,
                Err(code) => return code,
            };
            run_doctor(&config, check_changelog)
        }
        Commands::Bump {
            level,
            dry_run,
            skip_preflight,
            first_release,
        } => {
            let config = match load_config(args.config) {
                Ok(c) => c,
                Err(code) => return code,
            };
            run_bump(&config, level, dry_run, &skip_preflight, first_release)
        }
        Commands::Open {
            target,
            print_url,
            browser,
        } => run_open(args.config.as_deref(), target.as_deref(), print_url, browser.as_deref()),
    }
}

pub fn run_open(
    config_override: Option<&Path>,
    target_arg: Option<&str>,
    print_url: bool,
    browser_override: Option<&str>,
) -> i32 {
    let (root_dir, tag_prefix) = match config_override {
        Some(path) => match config::load(path) {
            Ok(c) => (c.root_dir, c.git.tag_prefix),
            Err(e) => {
                print_error(e);
                return 1;
            }
        },
        None => match std::env::current_dir() {
            Ok(d) => match config::discover(&d) {
                Ok(c) => (c.root_dir, c.git.tag_prefix),
                Err(_) => (d, "v".to_string()),
            },
            Err(e) => {
                print_error(format!("failed to determine current directory: {e}"));
                return 1;
            }
        },
    };

    let target = crate::cli::open::OpenTarget::parse(target_arg);
    let url = match crate::cli::open::resolve_url_for_repo(&root_dir, &target, &tag_prefix) {
        Ok(u) => u,
        Err(e) => {
            print_error(e);
            return 1;
        }
    };

    if print_url {
        println!("{url}");
        return 0;
    }

    match crate::cli::open::launch_browser(&url, browser_override) {
        Ok(()) => 0,
        Err(e) => {
            print_error(e);
            1
        }
    }
}

pub fn load_config(config_path: Option<PathBuf>) -> Result<config::Config, i32> {
    let config = match config_path {
        Some(path) => config::load(path),
        None => {
            let start_dir = match std::env::current_dir() {
                Ok(d) => d,
                Err(e) => {
                    print_error(format!("unable to determine current directory: {e}"));
                    return Err(1);
                }
            };
            config::discover(start_dir)
        }
    };
    config.map_err(|e| {
        let theme = Theme::stderr();
        eprintln!("{} {e}", theme.error("Error loading config:"));
        1
    })
}

pub fn resolve_changelog_path(config_override: Option<&Path>, path: Option<PathBuf>) -> Result<PathBuf, i32> {
    if let Some(p) = path {
        return Ok(p);
    }

    let config_result = match config_override {
        Some(p) => config::load(p),
        None => match std::env::current_dir() {
            Ok(dir) => config::discover(dir),
            Err(e) => {
                print_error(format!("unable to determine current directory: {e}"));
                return Err(1);
            }
        },
    };

    match config_result {
        Ok(config) => {
            if let Some(ref cl_path) = config.changelog.path {
                let cl_pb = Path::new(cl_path);
                if cl_pb.is_absolute() {
                    Ok(cl_pb.to_path_buf())
                } else {
                    Ok(config.root_dir.join(cl_pb))
                }
            } else {
                let candidate = config.root_dir.join("CHANGELOG.md");
                if candidate.exists() {
                    Ok(candidate)
                } else {
                    let fallback = PathBuf::from("CHANGELOG.md");
                    if fallback.exists() {
                        Ok(fallback)
                    } else {
                        print_error("no changelog path configured and CHANGELOG.md not found");
                        Err(1)
                    }
                }
            }
        }
        Err(e) => {
            let fallback = PathBuf::from("CHANGELOG.md");
            if fallback.exists() {
                Ok(fallback)
            } else {
                print_error(e);
                Err(1)
            }
        }
    }
}

fn load_template_file(config_override: Option<&Path>, template_path: &Path) -> Result<String, String> {
    let resolved = if template_path.is_absolute() {
        template_path.to_path_buf()
    } else {
        let cfg = config_override
            .and_then(|p| config::load(p).ok())
            .or_else(|| std::env::current_dir().ok().and_then(|d| config::discover(d).ok()));
        if let Some(ref c) = cfg {
            let candidate = c.root_dir.join(template_path);
            if candidate.is_file() {
                candidate
            } else {
                template_path.to_path_buf()
            }
        } else {
            template_path.to_path_buf()
        }
    };

    std::fs::read_to_string(&resolved)
        .map_err(|e| format!("failed to read template file '{}': {e}", template_path.display()))
}

fn extract_heading_date(content: &str, target_ver: &str) -> Option<String> {
    let target_norm = target_ver.trim_start_matches(['v', 'V']);
    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let after_h2 = heading.trim_start();
            let lower = after_h2.to_ascii_lowercase();
            if lower.starts_with("[unreleased]") || lower.starts_with("unreleased") {
                continue;
            }
            if let Some((ver_part, date_part)) = after_h2.split_once(" - ") {
                let v = ver_part.trim().trim_matches(['[', ']']).trim_start_matches(['v', 'V']);
                if v == target_norm {
                    let d = date_part.trim();
                    if !d.is_empty() {
                        return Some(d.to_string());
                    }
                }
            }
        }
    }
    None
}

fn find_raw_heading_version<'a>(content: &'a str, target_ver: &str) -> Option<&'a str> {
    let target_norm = target_ver.trim_start_matches(['v', 'V']);
    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let after_h2 = heading.trim_start();
            let lower = after_h2.to_ascii_lowercase();
            if lower.starts_with("[unreleased]") || lower.starts_with("unreleased") {
                continue;
            }
            let raw_ver = if after_h2.starts_with('[') {
                after_h2.find(']').map(|end| after_h2[1..end].trim())
            } else {
                after_h2
                    .split_whitespace()
                    .next()
                    .map(|s| s.trim_end_matches(':').trim())
            };
            if let Some(raw) = raw_ver
                && raw.trim_start_matches(['v', 'V']) == target_norm
            {
                return Some(raw);
            }
        }
    }
    None
}

fn resolve_release_tag_and_prefix(
    cfg: Option<&config::Config>,
    root_dir: &Path,
    content: &str,
    version: &str,
) -> (crate::git::TagName, crate::git::TagPrefix) {
    let clean_ver = crate::git::TagName::parse(version)
        .map(|t| t.normalize_version(&crate::git::TagPrefix::default()).to_string())
        .unwrap_or_else(|_| version.trim_start_matches(['v', 'V']).to_string());

    if let Some(c) = cfg {
        let prefix = crate::git::TagPrefix::new(&c.git.tag_prefix);
        let tag = prefix.format_tag(&clean_ver);
        return (tag, prefix);
    }

    let v_prefix = crate::git::TagPrefix::new("v");
    let v_tag = v_prefix.format_tag(&clean_ver);
    if crate::git::tag_exists(root_dir, v_tag.as_str()).unwrap_or(false) {
        return (v_tag, v_prefix);
    }

    let empty_prefix = crate::git::TagPrefix::default();
    let bare_tag = empty_prefix.format_tag(&clean_ver);
    if crate::git::tag_exists(root_dir, bare_tag.as_str()).unwrap_or(false) {
        return (bare_tag, empty_prefix);
    }

    if let Some(raw) = find_raw_heading_version(content, &clean_ver)
        && raw.starts_with(['v', 'V'])
    {
        return (v_tag, v_prefix);
    }

    (bare_tag, empty_prefix)
}

struct ReleaseTargetInfo {
    version: String,
    prev_version: Option<String>,
    raw_commits: Vec<crate::git::RawCommit>,
}

fn resolve_latest_target_info(
    content: &str,
    target_path: &Path,
    root_dir: &Path,
    cfg: Option<&config::Config>,
) -> Result<ReleaseTargetInfo, i32> {
    let mut versions = crate::changelog::list_versions(content);
    if versions.is_empty() {
        print_error(format!(
            "no release section found in changelog '{}'",
            target_path.display()
        ));
        return Err(1);
    }

    let prev_ver = versions.get(1).cloned();
    let prev_tag = prev_ver
        .as_deref()
        .map(|pv| resolve_release_tag_and_prefix(cfg, root_dir, content, pv).0);
    let raw_commits = crate::git::raw_commits_since(root_dir, prev_tag.as_deref()).unwrap_or_default();
    let latest_ver = versions.remove(0);

    Ok(ReleaseTargetInfo {
        version: latest_ver,
        prev_version: prev_ver,
        raw_commits,
    })
}

fn resolve_show_target_info(
    content: &str,
    requested_version: &str,
    target_path: &Path,
    root_dir: &Path,
    cfg: Option<&config::Config>,
) -> Result<ReleaseTargetInfo, i32> {
    let target_norm = requested_version.trim_start_matches(['v', 'V']);
    let versions = crate::changelog::list_versions(content);
    let Some(idx) = versions
        .iter()
        .position(|v| v.trim_start_matches(['v', 'V']) == target_norm)
    else {
        print_error(format!(
            "version '{requested_version}' not found in changelog '{}'",
            target_path.display()
        ));
        return Err(1);
    };

    let matched_ver = versions[idx].clone();
    let prev_ver = versions.get(idx + 1).cloned();
    let (tag, _) = resolve_release_tag_and_prefix(cfg, root_dir, content, &matched_ver);
    let prev_tag = prev_ver
        .as_deref()
        .map(|pv| resolve_release_tag_and_prefix(cfg, root_dir, content, pv).0);
    let raw_commits = crate::git::raw_commits_between(root_dir, prev_tag.as_deref(), &tag).unwrap_or_default();

    Ok(ReleaseTargetInfo {
        version: matched_ver,
        prev_version: prev_ver,
        raw_commits,
    })
}

fn resolve_changelog_context(
    config_override: Option<&Path>,
    target_path: &Path,
    content: &str,
    target: ReleaseTargetInfo,
) -> ReleaseContext {
    let cfg = config_override
        .and_then(|p| config::load(p).ok())
        .or_else(|| std::env::current_dir().ok().and_then(|d| config::discover(d).ok()));

    let default_changelog = crate::config::Changelog::default();
    let changelog_config = cfg.as_ref().map(|c| &c.changelog).unwrap_or(&default_changelog);
    let dummy_root = PathBuf::from(".");
    let root_dir = cfg
        .as_ref()
        .map(|c| c.root_dir.as_path())
        .or_else(|| target_path.parent())
        .unwrap_or(&dummy_root);

    let (tag, _) = resolve_release_tag_and_prefix(cfg.as_ref(), root_dir, content, &target.version);
    let prev_tag = target
        .prev_version
        .as_deref()
        .map(|pv| resolve_release_tag_and_prefix(cfg.as_ref(), root_dir, content, pv).0);

    let commit_messages: Vec<String> = target.raw_commits.iter().map(|r| r.message.clone()).collect();
    let (_bump, parsed_commits) = crate::conventional::parse_and_deduce_bump(&commit_messages);
    let contributors = crate::git::list_authors_between(root_dir, prev_tag.as_deref(), &tag).unwrap_or_default();
    let repository = crate::git::remote_url(root_dir);
    let date_str = extract_heading_date(content, &target.version)
        .unwrap_or_else(|| crate::changelog::format_date(std::time::SystemTime::now()));

    let mut ctx = crate::changelog::assemble_release_context(crate::changelog::AssembleContextParams {
        version: &target.version,
        prev_version: target.prev_version.as_deref(),
        tag: &tag,
        prev_tag: prev_tag.as_deref(),
        date: &date_str,
        repository,
        parsed_commits: &parsed_commits,
        raw_commits: Some(&target.raw_commits),
        contributors,
        changelog_config,
    });

    if parsed_commits.is_empty()
        && let Some(body) = crate::changelog::extract_version(content, &target.version, false)
        && !body.is_empty()
    {
        ctx.all_changes = body;
    }

    ctx
}

fn execute_changelog_output(
    config_override: Option<&Path>,
    format: ChangelogFormat,
    context_supplier: impl FnOnce() -> Result<ReleaseContext, i32>,
    markdown_supplier: impl FnOnce(bool) -> Result<String, crate::changelog::Error>,
) -> i32 {
    match format {
        ChangelogFormat::Json => {
            let ctx = match context_supplier() {
                Ok(c) => c,
                Err(code) => return code,
            };
            if let Err(e) = changelog::print_release_context_json(&ctx) {
                print_error(e);
                return 1;
            }
            0
        }
        ChangelogFormat::Template(ref t_path) => {
            let template_str = match load_template_file(config_override, t_path) {
                Ok(s) => s,
                Err(e) => {
                    print_error(e);
                    return 1;
                }
            };
            let ctx = match context_supplier() {
                Ok(c) => c,
                Err(code) => return code,
            };
            match crate::changelog::render_template(&template_str, &ctx) {
                Ok(rendered) => {
                    changelog::print_changelog_output(&rendered, &Theme::stdout());
                    0
                }
                Err(e) => {
                    print_error(e);
                    1
                }
            }
        }
        ChangelogFormat::Markdown { include_header } => match markdown_supplier(include_header) {
            Ok(output) => {
                changelog::print_changelog_output(&output, &Theme::stdout());
                0
            }
            Err(e) => {
                print_error(e);
                1
            }
        },
    }
}

pub fn run_changelog(config_override: Option<&Path>, command: ChangelogCommands) -> i32 {
    match command {
        ChangelogCommands::Latest {
            include_header,
            path,
            template,
            json,
        } => {
            let target_path = match resolve_changelog_path(config_override, path) {
                Ok(p) => p,
                Err(code) => return code,
            };
            let format = ChangelogFormat::from_options(json, template, include_header);
            run_changelog_latest(config_override, &target_path, format)
        }
        ChangelogCommands::Show {
            version,
            include_header,
            path,
            template,
            json,
        } => {
            let target_path = match resolve_changelog_path(config_override, path) {
                Ok(p) => p,
                Err(code) => return code,
            };
            let format = ChangelogFormat::from_options(json, template, include_header);
            run_changelog_show(config_override, &target_path, &version, format)
        }
        ChangelogCommands::Open {
            target,
            print_url,
            browser,
        } => run_open(config_override, target.as_deref(), print_url, browser.as_deref()),
    }
}

fn read_file_content(path: &Path) -> Result<String, i32> {
    std::fs::read_to_string(path).map_err(|e| {
        print_error(format!("failed to read changelog '{}': {e}", path.display()));
        1
    })
}

fn run_changelog_latest(config_override: Option<&Path>, target_path: &Path, format: ChangelogFormat) -> i32 {
    let context_supplier = || {
        let content = read_file_content(target_path)?;
        let cfg = config_override
            .and_then(|p| config::load(p).ok())
            .or_else(|| std::env::current_dir().ok().and_then(|d| config::discover(d).ok()));
        let dummy_root = PathBuf::from(".");
        let root_dir = cfg
            .as_ref()
            .map(|c| c.root_dir.as_path())
            .or_else(|| target_path.parent())
            .unwrap_or(&dummy_root);
        let target = resolve_latest_target_info(&content, target_path, root_dir, cfg.as_ref())?;
        Ok(resolve_changelog_context(
            config_override,
            target_path,
            &content,
            target,
        ))
    };

    let markdown_supplier = |include_header| crate::changelog::read_latest(target_path, include_header);

    execute_changelog_output(config_override, format, context_supplier, markdown_supplier)
}

fn run_changelog_show(
    config_override: Option<&Path>,
    target_path: &Path,
    version: &str,
    format: ChangelogFormat,
) -> i32 {
    let context_supplier = || {
        let content = read_file_content(target_path)?;
        let cfg = config_override
            .and_then(|p| config::load(p).ok())
            .or_else(|| std::env::current_dir().ok().and_then(|d| config::discover(d).ok()));
        let dummy_root = PathBuf::from(".");
        let root_dir = cfg
            .as_ref()
            .map(|c| c.root_dir.as_path())
            .or_else(|| target_path.parent())
            .unwrap_or(&dummy_root);
        let target = resolve_show_target_info(&content, version, target_path, root_dir, cfg.as_ref())?;
        Ok(resolve_changelog_context(
            config_override,
            target_path,
            &content,
            target,
        ))
    };

    let markdown_supplier = |include_header| crate::changelog::read_version(target_path, version, include_header);

    execute_changelog_output(config_override, format, context_supplier, markdown_supplier)
}

pub fn run_bump(
    config: &config::Config,
    level: BumpLevel,
    dry_run: bool,
    skip_preflight: &[String],
    first_release: bool,
) -> i32 {
    match bump::run_with_first_release(config, level, dry_run, skip_preflight, first_release) {
        Ok(summary) => {
            print_bump_summary(&summary);
            0
        }
        Err(e) => {
            print_error(e);
            1
        }
    }
}

pub fn run_doctor(config: &config::Config, check_changelog: bool) -> i32 {
    let mut has_drift = false;
    let err_theme = Theme::stderr();
    let out_theme = Theme::stdout();

    match bump::doctor(config) {
        Ok(drifts) => {
            if !drifts.is_empty() {
                eprint!("{}", crate::cli::doctor::render_version_drift(&drifts, &err_theme));
                has_drift = true;
            }
        }
        Err(e) => {
            print_error(e);
            return 1;
        }
    }

    if check_changelog {
        match bump::doctor_changelog(config) {
            Ok(cl_drift) => {
                if !cl_drift.is_empty() {
                    eprint!("{}", crate::cli::doctor::render_changelog_drift(&cl_drift, &err_theme));
                    has_drift = true;
                }
            }
            Err(e) => {
                let theme = Theme::stderr();
                eprintln!("{} {e}", theme.error("Error checking changelog:"));
                return 1;
            }
        }
    }

    if has_drift {
        return 2;
    }

    print!(
        "{}",
        crate::cli::doctor::render_dashboard(config, check_changelog, &out_theme)
    );
    0
}

pub fn print_bump_summary(summary: &Summary) {
    crate::cli::tree::print_tree_summary(summary);
}

#[allow(dead_code)]
pub fn print_summary(summary: &Summary) {
    print_bump_summary(summary);
}

#[cfg(test)]
mod tests {
    static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn tmp_id(prefix: &str) -> String {
        let n = TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("{}-{}-{}", prefix, std::process::id(), n)
    }

    use super::*;
    use clap::Parser;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;

    fn temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(tmp_id(prefix));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        path
    }

    fn git_commit(dir: &Path, msg: &str) {
        let status = std::process::Command::new("git")
            .current_dir(dir)
            .args(["add", "."])
            .status()
            .unwrap();
        assert!(status.success(), "git add failed");
        let status = std::process::Command::new("git")
            .current_dir(dir)
            .args(["commit", "-m", msg, "-q"])
            .status()
            .unwrap();
        assert!(status.success(), "git commit failed");
    }

    fn git_tag(dir: &Path, tag: &str) {
        let status = std::process::Command::new("git")
            .current_dir(dir)
            .args(["tag", tag])
            .status()
            .unwrap();
        assert!(status.success(), "git tag failed: {tag}");
    }

    #[test]
    fn doctor_reports_valid_config() {
        let dir = temp_dir("cutver-doc-ok");
        write(&dir, "package.json", r#"{"version": "1.0.0"}"#);
        write(
            &dir,
            "release.toml",
            r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
        );
        let args =
            Cli::try_parse_from(["cutver", "-c", &dir.join("release.toml").to_string_lossy(), "doctor"]).unwrap();
        assert_eq!(run(args), 0);
    }

    #[test]
    fn doctor_fails_for_invalid_config() {
        let dir = temp_dir("cutver-doc-bad");
        write(&dir, "release.toml", "[version]\ncurrent_source = \"missing\"");
        let args =
            Cli::try_parse_from(["cutver", "-c", &dir.join("release.toml").to_string_lossy(), "doctor"]).unwrap();
        assert_eq!(run(args), 1);
    }

    #[test]
    fn doctor_reports_drift_exit_code() {
        let dir = temp_dir("cutver-doc-drift-exit");
        write(&dir, "package.json", r#"{"version": "1.2.3"}"#);
        write(&dir, "Cargo.toml", "[package]\nversion = \"1.0.0\"\n");
        write(
            &dir,
            "release.toml",
            r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
"#,
        );
        let args =
            Cli::try_parse_from(["cutver", "-c", &dir.join("release.toml").to_string_lossy(), "doctor"]).unwrap();
        assert_eq!(run(args), 2);
    }

    #[test]
    fn doctor_with_check_changelog_consistent() {
        let dir = temp_dir("cutver-doc-cl-main-ok");
        crate::git::init_test_repo(&dir);
        write(&dir, "package.json", r#"{"version": "1.0.0"}"#);
        write(
            &dir,
            "CHANGELOG.md",
            "# Changelog\n\n## [1.0.0] - 2026-01-01\n- initial\n",
        );
        write(
            &dir,
            "release.toml",
            r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
        );
        git_commit(&dir, "chore: initial");
        git_tag(&dir, "v1.0.0");

        let args = Cli::try_parse_from([
            "cutver",
            "-c",
            &dir.join("release.toml").to_string_lossy(),
            "doctor",
            "--check-changelog",
        ])
        .unwrap();
        assert_eq!(run(args), 0);
    }

    #[test]
    fn doctor_with_check_changelog_drift_missing_in_changelog() {
        let dir = temp_dir("cutver-doc-cl-main-missing");
        crate::git::init_test_repo(&dir);
        write(&dir, "package.json", r#"{"version": "1.1.0"}"#);
        write(
            &dir,
            "CHANGELOG.md",
            "# Changelog\n\n## [1.0.0] - 2026-01-01\n- initial\n",
        );
        write(
            &dir,
            "release.toml",
            r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
        );
        git_commit(&dir, "chore: initial");
        git_tag(&dir, "v1.0.0");
        git_tag(&dir, "v1.1.0");

        let args = Cli::try_parse_from([
            "cutver",
            "-c",
            &dir.join("release.toml").to_string_lossy(),
            "doctor",
            "--check-changelog",
        ])
        .unwrap();
        assert_eq!(run(args), 2);
    }

    #[test]
    fn doctor_with_check_changelog_orphan_section() {
        let dir = temp_dir("cutver-doc-cl-main-orphan");
        crate::git::init_test_repo(&dir);
        write(&dir, "package.json", r#"{"version": "1.0.0"}"#);
        write(
            &dir,
            "CHANGELOG.md",
            "# Changelog\n\n## [1.2.0] - 2026-01-02\n- unreleased\n\n## [1.0.0] - 2026-01-01\n- initial\n",
        );
        write(
            &dir,
            "release.toml",
            r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
        );
        git_commit(&dir, "chore: initial");
        git_tag(&dir, "v1.0.0");

        let args = Cli::try_parse_from([
            "cutver",
            "-c",
            &dir.join("release.toml").to_string_lossy(),
            "doctor",
            "--check-changelog",
        ])
        .unwrap();
        assert_eq!(run(args), 2);
    }

    #[test]
    fn doctor_with_check_changelog_error() {
        let dir = temp_dir("cutver-doc-cl-main-err");
        crate::git::init_test_repo(&dir);
        write(&dir, "package.json", r#"{"version": "1.0.0"}"#);
        write(
            &dir,
            "release.toml",
            r#"
[version]
current_source = "package.json"
[changelog]
path = "NONEXISTENT.md"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
        );
        let args = Cli::try_parse_from([
            "cutver",
            "-c",
            &dir.join("release.toml").to_string_lossy(),
            "doctor",
            "--check-changelog",
        ])
        .unwrap();
        assert_eq!(run(args), 1);
    }

    #[test]
    fn bump_dry_run_does_not_mutate() {
        let dir = temp_dir("cutver-bump-stub");
        crate::git::init_test_repo(&dir);
        write(&dir, "package.json", r#"{"version": "1.2.3"}"#);
        write(&dir, "Cargo.toml", "[package]\nversion = \"1.2.3\"\n");
        write(
            &dir,
            "release.toml",
            r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
[git]
require_clean_tree = false
[preflight]
tests = "cargo test"
"#,
        );
        let cargo_before = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        let args = Cli::try_parse_from([
            "cutver",
            "-c",
            &dir.join("release.toml").to_string_lossy(),
            "bump",
            "minor",
            "--dry-run",
            "--skip-preflight",
            "tests",
        ])
        .unwrap();
        assert_eq!(run(args), 0);
        assert_eq!(fs::read_to_string(dir.join("Cargo.toml")).unwrap(), cargo_before);
    }

    #[test]
    fn bump_auto_dry_run_with_feat() {
        let dir = temp_dir("cutver-bump-auto-main");
        crate::git::init_test_repo(&dir);
        write(&dir, "package.json", r#"{"version": "1.2.3"}"#);
        write(&dir, "Cargo.toml", "[package]\nversion = \"1.2.3\"\n");
        write(
            &dir,
            "release.toml",
            r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
[git]
require_clean_tree = false
"#,
        );
        std::process::Command::new("git")
            .current_dir(&dir)
            .args(["add", "."])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .current_dir(&dir)
            .args(["commit", "-m", "chore: initial commit", "-q"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .current_dir(&dir)
            .args(["commit", "--allow-empty", "-m", "feat: exciting new feature", "-q"])
            .status()
            .unwrap();

        let args = Cli::try_parse_from([
            "cutver",
            "-c",
            &dir.join("release.toml").to_string_lossy(),
            "bump",
            "auto",
            "--dry-run",
        ])
        .unwrap();
        assert_eq!(run(args), 0);
    }

    #[test]
    fn config_defaults_and_preflight_order() {
        let dir = temp_dir("cutver-config-defaults");
        write(
            &dir,
            "release.toml",
            "[version]\ncurrent_source = \"a\"\n[[manifest]]\npath = \"a\"\nkind = \"cargo-package\"\n[preflight]\ntests = \"cargo test\"\nz = \"z\"\na = \"a\"\nm = \"m\"\n[changelog]\npath = \"CHANGELOG.md\"\n",
        );
        write(&dir, "a", "");
        let cfg = config::load(dir.join("release.toml")).unwrap();
        assert_eq!(Path::new(&cfg.version.current_source).file_name().unwrap(), "a");
        assert_eq!(
            (cfg.manifest.len(), cfg.preflight.len(), cfg.git.tag_prefix.as_str()),
            (1, 4, "v")
        );
        assert_eq!(
            (
                cfg.changelog.format.as_str(),
                cfg.changelog.entry_template.as_str(),
                cfg.git.commit_message.as_str()
            ),
            ("keep-a-changelog", "", "chore(release): v{version}")
        );
        assert!(cfg.git.require_clean_tree && cfg.git.require_branch.is_none());
        assert_eq!(
            cfg.preflight.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            vec!["tests", "z", "a", "m"]
        );
    }

    #[test]
    fn changelog_latest_explicit_path() {
        let dir = temp_dir("cutver-cl-explicit");
        let cl = write(
            &dir,
            "MY_CHANGELOG.md",
            "# Changelog\n\n## [1.2.0] - 2026-03-01\n\n- Added feature X\n\n## [1.1.0] - 2026-02-01\n\n- Old feature\n",
        );
        let args = Cli::try_parse_from(["cutver", "changelog", "latest", "-p", &cl.to_string_lossy()]).unwrap();
        assert_eq!(run(args), 0);
    }

    #[test]
    fn changelog_latest_explicit_path_with_header() {
        let dir = temp_dir("cutver-cl-header");
        let cl = write(
            &dir,
            "MY_CHANGELOG.md",
            "# Changelog\n\n## [1.2.0] - 2026-03-01\n\n- Added feature X\n",
        );
        let args = Cli::try_parse_from(["cutver", "changelog", "latest", "-H", "-p", &cl.to_string_lossy()]).unwrap();
        assert_eq!(run(args), 0);
    }

    #[test]
    fn changelog_latest_from_config() {
        let dir = temp_dir("cutver-cl-cfg");
        write(
            &dir,
            "CUSTOM_CHANGELOG.md",
            "# Changelog\n\n## [2.0.0] - 2026-04-01\n\n- Breaking change\n",
        );
        write(&dir, "package.json", r#"{"version": "2.0.0"}"#);
        write(
            &dir,
            "cutver.toml",
            r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
[changelog]
path = "CUSTOM_CHANGELOG.md"
"#,
        );
        let args = Cli::try_parse_from([
            "cutver",
            "-c",
            &dir.join("cutver.toml").to_string_lossy(),
            "changelog",
            "latest",
        ])
        .unwrap();
        assert_eq!(run(args), 0);
    }

    #[test]
    fn changelog_latest_missing_explicit_path_fails() {
        let args = Cli::try_parse_from([
            "cutver",
            "changelog",
            "latest",
            "-p",
            "this-file-does-not-exist-12345.md",
        ])
        .unwrap();
        assert_eq!(run(args), 1);
    }

    #[test]
    fn changelog_latest_no_release_section_fails() {
        let dir = temp_dir("cutver-cl-no-rel");
        let cl = write(
            &dir,
            "CHANGELOG.md",
            "# Changelog\n\n## [Unreleased]\n\n- Work in progress\n",
        );
        let args = Cli::try_parse_from(["cutver", "changelog", "latest", "-p", &cl.to_string_lossy()]).unwrap();
        assert_eq!(run(args), 1);
    }

    #[test]
    fn changelog_latest_config_missing_file_fails() {
        let dir = temp_dir("cutver-cl-cfg-missing");
        write(&dir, "package.json", r#"{"version": "1.0.0"}"#);
        write(
            &dir,
            "cutver.toml",
            r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
[changelog]
path = "NONEXISTENT.md"
"#,
        );
        let args = Cli::try_parse_from([
            "cutver",
            "-c",
            &dir.join("cutver.toml").to_string_lossy(),
            "changelog",
            "latest",
        ])
        .unwrap();
        assert_eq!(run(args), 1);
    }

    #[test]
    fn changelog_latest_config_without_changelog_path_fallback() {
        let dir = temp_dir("cutver-cl-no-path");
        write(
            &dir,
            "CHANGELOG.md",
            "# Changelog\n\n## [1.0.0] - 2026-01-01\n\n- Initial release\n",
        );
        write(&dir, "package.json", r#"{"version": "1.0.0"}"#);
        write(
            &dir,
            "cutver.toml",
            r#"
[version]
current_source = "package.json"
[[manifest]]
path = "package.json"
kind = "json"
field = "version"
"#,
        );
        let args = Cli::try_parse_from([
            "cutver",
            "-c",
            &dir.join("cutver.toml").to_string_lossy(),
            "changelog",
            "latest",
        ])
        .unwrap();
        assert_eq!(run(args), 0);
    }

    #[test]
    fn run_changelog_direct() {
        let dir = temp_dir("cutver-run-cl");
        let cl = write(
            &dir,
            "CHANGELOG.md",
            "# Changelog\n\n## [1.0.0] - 2026-01-01\n\n- Released\n",
        );
        let cmd = ChangelogCommands::Latest {
            include_header: false,
            path: Some(cl),
            template: None,
            json: false,
        };
        assert_eq!(run_changelog(None, cmd), 0);
    }

    #[test]
    fn changelog_show_explicit_path() {
        let dir = temp_dir("cutver-cl-show-explicit");
        let cl = write(
            &dir,
            "MY_CHANGELOG.md",
            "# Changelog\n\n## [1.2.0] - 2026-03-01\n\n- Added feature X\n\n## [1.1.0] - 2026-02-01\n\n- Old feature\n",
        );
        let args = Cli::try_parse_from(["cutver", "changelog", "show", "1.1.0", "-p", &cl.to_string_lossy()]).unwrap();
        assert_eq!(run(args), 0);
    }

    #[test]
    fn changelog_show_explicit_path_with_header() {
        let dir = temp_dir("cutver-cl-show-header");
        let cl = write(
            &dir,
            "MY_CHANGELOG.md",
            "# Changelog\n\n## [1.2.0] - 2026-03-01\n\n- Added feature X\n\n## [1.1.0] - 2026-02-01\n\n- Old feature\n",
        );
        let args = Cli::try_parse_from([
            "cutver",
            "changelog",
            "show",
            "1.1.0",
            "-H",
            "-p",
            &cl.to_string_lossy(),
        ])
        .unwrap();
        assert_eq!(run(args), 0);
    }

    #[test]
    fn changelog_show_missing_version_fails() {
        let dir = temp_dir("cutver-cl-show-missing");
        let cl = write(
            &dir,
            "MY_CHANGELOG.md",
            "# Changelog\n\n## [1.2.0] - 2026-03-01\n\n- Added feature X\n",
        );
        let args = Cli::try_parse_from(["cutver", "changelog", "show", "0.9.0", "-p", &cl.to_string_lossy()]).unwrap();
        assert_eq!(run(args), 1);
    }

    #[test]
    fn run_changelog_show_direct() {
        let dir = temp_dir("cutver-run-cl-show");
        let cl = write(
            &dir,
            "CHANGELOG.md",
            "# Changelog\n\n## [1.0.0] - 2026-01-01\n\n- Released\n",
        );
        let cmd = ChangelogCommands::Show {
            version: "1.0.0".to_string(),
            include_header: false,
            path: Some(cl),
            template: None,
            json: false,
        };
        assert_eq!(run_changelog(None, cmd), 0);
    }

    #[test]
    fn run_init_success_and_fail_on_collision() {
        let dir = temp_dir("cutver-runner-init");
        write(&dir, "Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n");

        let args = Cli::try_parse_from(["cutver", "init", "-p", &dir.to_string_lossy()]).unwrap();
        assert_eq!(run(args), 0);
        assert!(dir.join("cutver.toml").is_file());

        // Second run without force should fail
        let args_fail = Cli::try_parse_from(["cutver", "init", "-p", &dir.to_string_lossy()]).unwrap();
        assert_eq!(run(args_fail), 1);

        // Third run with force should succeed
        let args_force = Cli::try_parse_from(["cutver", "init", "-p", &dir.to_string_lossy(), "--force"]).unwrap();
        assert_eq!(run(args_force), 0);
    }
}
