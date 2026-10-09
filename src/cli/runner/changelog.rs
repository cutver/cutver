use std::path::{Path, PathBuf};

use crate::changelog::ReleaseContext;
use crate::cli::args::ChangelogCommands;
use crate::cli::changelog::{self, ChangelogFormat};
use crate::cli::path::relativize_path;
use crate::cli::style::Theme;
use crate::config;

pub use super::changelog_context::{
    extract_heading_date, find_raw_heading_version, load_template_file, resolve_release_tag_and_prefix,
};
use super::dispatch::{print_error, resolve_changelog_path};
use super::open::run_open;

pub struct ReleaseTargetInfo {
    pub version: String,
    pub prev_version: Option<String>,
    pub raw_commits: Vec<crate::git::RawCommit>,
}

pub fn resolve_latest_target_info(
    content: &str,
    target_path: &Path,
    root_dir: &Path,
    cfg: Option<&config::Config>,
) -> Result<ReleaseTargetInfo, i32> {
    let mut versions = crate::changelog::list_versions(content);
    if versions.is_empty() {
        let display_path = relativize_path(&target_path.to_string_lossy(), Some(root_dir));
        print_error(format!("no release section found in changelog '{display_path}'"));
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

pub fn resolve_show_target_info(
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
        let display_path = relativize_path(&target_path.to_string_lossy(), Some(root_dir));
        print_error(format!(
            "version '{requested_version}' not found in changelog '{display_path}'"
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

pub fn resolve_changelog_context(
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
    // Fail-safe policy lives in `crate::git::first_time_contributors`: a first release marks
    // everyone new, but unreadable prior history announces nobody.
    let first_time_contributors = crate::git::first_time_contributors(root_dir, prev_tag.as_deref(), &contributors);
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
        first_time_contributors,
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

pub fn execute_changelog_output(
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

pub fn read_file_content(path: &Path) -> Result<String, i32> {
    std::fs::read_to_string(path).map_err(|e| {
        print_error(format!("failed to read changelog '{}': {e}", path.display()));
        1
    })
}

pub fn run_changelog_latest(config_override: Option<&Path>, target_path: &Path, format: ChangelogFormat) -> i32 {
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

pub fn run_changelog_show(
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
