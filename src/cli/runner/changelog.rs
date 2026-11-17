use std::path::{Path, PathBuf};

use crate::changelog::ReleaseContext;
use crate::cli::args::ChangelogCommands;
use crate::cli::changelog::{self, ChangelogFormat};
use crate::cli::path::relativize_path;
use crate::cli::style::Theme;
use crate::config;
use crate::git::types::strip_one_version_marker;

pub use super::changelog_context::{
    extract_heading_date, find_raw_heading_version, load_template_file, resolve_prefix, resolve_release_tag_and_prefix,
};
use super::dispatch::{print_error, resolve_changelog_path};
use super::open::run_open;

pub struct ReleaseTargetInfo {
    pub version: String,
    pub prev_version: Option<String>,
    pub raw_commits: Vec<crate::git::RawCommit>,
}

/// What the read surface does when the commit range behind the requested version cannot be read.
///
/// `--json` is a machine contract: `"commits": []` must mean "the range is empty", so an
/// unreadable range fails closed with exit `1` and the git error on stderr. Template rendering
/// keeps its long-standing degradation — the rendered entry carries the changelog body, and the
/// commit list it iterates over is simply empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitHistoryFailure {
    /// Print the git error and exit `1` instead of reporting an empty range.
    FailClosed,
    /// Keep the pre-existing degradation: no commits, exit `0`.
    Degrade,
}

impl GitHistoryFailure {
    fn for_format(format: &ChangelogFormat) -> Self {
        match format {
            ChangelogFormat::Json => Self::FailClosed,
            ChangelogFormat::Template(_) | ChangelogFormat::Markdown { .. } => Self::Degrade,
        }
    }
}

/// Reads the commits in `from_tag..to_ref` through the checked `git log` runner, which propagates
/// every failure instead of reading a failing status as an empty history.
fn read_raw_commits(
    root_dir: &Path,
    from_tag: Option<&str>,
    to_ref: &str,
    on_failure: GitHistoryFailure,
) -> Result<Vec<crate::git::RawCommit>, i32> {
    let args = crate::git::log::range_args(from_tag, to_ref);
    match crate::git::log::run_log_checked(root_dir, &args) {
        Ok(commits) => Ok(commits),
        Err(_) if on_failure == GitHistoryFailure::Degrade => Ok(Vec::new()),
        Err(err) => {
            print_error(git_history_error(&err));
            Err(1)
        }
    }
}

fn git_history_error(err: &crate::git::Error) -> String {
    format!("failed to read the release commit range from git: {err}")
}

pub fn resolve_latest_target_info(
    content: &str,
    target_path: &Path,
    root_dir: &Path,
    cfg: Option<&config::Config>,
    on_git_failure: GitHistoryFailure,
) -> Result<ReleaseTargetInfo, i32> {
    let mut versions = crate::changelog::list_versions_with_prefix(content, &resolve_prefix(cfg));
    if versions.is_empty() {
        let display_path = relativize_path(&target_path.to_string_lossy(), Some(root_dir));
        print_error(format!("no release section found in changelog '{display_path}'"));
        return Err(1);
    }

    let prev_ver = versions.get(1).cloned();
    let prev_tag = prev_ver
        .as_deref()
        .map(|pv| resolve_release_tag_and_prefix(cfg, root_dir, content, pv).0);
    let raw_commits = read_raw_commits(root_dir, prev_tag.as_deref(), "HEAD", on_git_failure)?;
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
    on_git_failure: GitHistoryFailure,
) -> Result<ReleaseTargetInfo, i32> {
    let prefix = resolve_prefix(cfg);
    let target_norm = strip_one_version_marker(requested_version, &prefix);
    let versions = crate::changelog::list_versions_with_prefix(content, &prefix);
    let Some(idx) = versions
        .iter()
        .position(|v| strip_one_version_marker(v, &prefix) == target_norm)
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
    let raw_commits = read_raw_commits(root_dir, prev_tag.as_deref(), tag.as_str(), on_git_failure)?;

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
    let prefix = resolve_prefix(cfg.as_ref());

    let commit_messages: Vec<String> = target.raw_commits.iter().map(|r| r.message.clone()).collect();
    let (_bump, parsed_commits) = crate::conventional::parse_and_deduce_bump(&commit_messages);
    let contributors = match crate::git::list_authors_between(root_dir, prev_tag.as_deref(), &tag) {
        Ok(authors) => authors,
        Err(err) => {
            crate::cli::style::warn_degradation(format!(
                "contributor list omitted, could not read commit authors: {err}"
            ));
            Vec::new()
        }
    };
    // Fail-safe policy lives in `crate::git::first_time_contributors`: a first release marks
    // everyone new, but unreadable prior history announces nobody and is reported here.
    let first_time_contributors =
        match crate::git::first_time_contributors(root_dir, prev_tag.as_deref(), &contributors) {
            Ok(names) => names,
            Err(err) => {
                crate::cli::style::warn_degradation(format!(
                    "\"New Contributors\" section omitted, prior history is unreadable: {err}"
                ));
                Vec::new()
            }
        };
    let repository = crate::git::remote_url(root_dir);
    let date_str = extract_heading_date(content, &target.version, &prefix)
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
        && let Some(body) = crate::changelog::extract_version_with_prefix(content, &target.version, false, &prefix)
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
    let cfg = load_optional_config(config_override);
    let prefix = resolve_prefix(cfg.as_ref());
    let on_git_failure = GitHistoryFailure::for_format(&format);

    let context_supplier = || {
        let content = read_file_content(target_path)?;
        let root_dir = root_dir_for(cfg.as_ref(), target_path);
        let target = resolve_latest_target_info(&content, target_path, &root_dir, cfg.as_ref(), on_git_failure)?;
        Ok(resolve_changelog_context(
            config_override,
            target_path,
            &content,
            target,
        ))
    };

    let markdown_supplier =
        |include_header| crate::changelog::read_latest_with_prefix(target_path, include_header, &prefix);

    execute_changelog_output(config_override, format, context_supplier, markdown_supplier)
}

pub fn run_changelog_show(
    config_override: Option<&Path>,
    target_path: &Path,
    version: &str,
    format: ChangelogFormat,
) -> i32 {
    let cfg = load_optional_config(config_override);
    let prefix = resolve_prefix(cfg.as_ref());
    let on_git_failure = GitHistoryFailure::for_format(&format);

    let context_supplier = || {
        let content = read_file_content(target_path)?;
        let root_dir = root_dir_for(cfg.as_ref(), target_path);
        let target = resolve_show_target_info(&content, version, target_path, &root_dir, cfg.as_ref(), on_git_failure)?;
        Ok(resolve_changelog_context(
            config_override,
            target_path,
            &content,
            target,
        ))
    };

    let markdown_supplier =
        |include_header| crate::changelog::read_version_with_prefix(target_path, version, include_header, &prefix);

    execute_changelog_output(config_override, format, context_supplier, markdown_supplier)
}

/// The configuration this read surface resolves for the invocation, or `None` when there is none.
fn load_optional_config(config_override: Option<&Path>) -> Option<config::Config> {
    config_override
        .and_then(|p| config::load(p).ok())
        .or_else(|| std::env::current_dir().ok().and_then(|d| config::discover(d).ok()))
}

fn root_dir_for(cfg: Option<&config::Config>, target_path: &Path) -> PathBuf {
    cfg.map(|c| c.root_dir.clone())
        .or_else(|| target_path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// A changelog workspace: an isolated temp dir holding a config, a manifest and the changelog.
    struct Fixture {
        dir: PathBuf,
        _tmp: tempfile::TempDir,
    }

    impl Fixture {
        fn new(name: &str, changelog: &str, tag_prefix: Option<&str>) -> Self {
            let tmp = tempfile::Builder::new()
                .prefix(&format!("cutver-cli-{name}-"))
                .tempdir()
                .expect("temp dir");
            let dir = tmp.path().to_path_buf();
            std::fs::write(dir.join("package.json"), r#"{"version": "1.2.0"}"#).unwrap();
            std::fs::write(dir.join("CHANGELOG.md"), changelog).unwrap();
            let mut config = String::from(
                "[version]\ncurrent_source = \"package.json\"\n\n[[manifest]]\npath = \"package.json\"\nkind = \"json\"\nfield = \"version\"\n\n[changelog]\npath = \"CHANGELOG.md\"\n",
            );
            if let Some(prefix) = tag_prefix {
                config.push_str(&format!("\n[git]\ntag_prefix = \"{prefix}\"\n"));
            }
            std::fs::write(dir.join("cutver.toml"), config).unwrap();
            Self { dir, _tmp: tmp }
        }

        fn config(&self) -> PathBuf {
            self.dir.join("cutver.toml")
        }

        fn changelog(&self) -> PathBuf {
            self.dir.join("CHANGELOG.md")
        }
    }

    fn git(dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn commit(dir: &Path, message: &str) {
        git(dir, &["commit", "--allow-empty", "-q", "-m", message]);
    }

    const RELEASE_PREFIXED: &str = "# Changelog\n\n## release-1.2.0\n- new work\n\n## release-1.1.0\n- older work\n";

    /// `[git] tag_prefix` names the tags, so it also says what the heading carries: with
    /// `release-`, the entry for `1.2.0` is `## release-1.2.0`.
    #[test]
    fn show_matches_the_heading_of_a_configured_prefix() {
        let fixture = Fixture::new("show-prefix", RELEASE_PREFIXED, Some("release-"));
        let code = run_changelog_show(
            Some(&fixture.config()),
            &fixture.changelog(),
            "1.2.0",
            ChangelogFormat::Markdown { include_header: false },
        );
        assert_eq!(code, 0);
    }

    #[test]
    fn latest_matches_the_heading_of_a_configured_prefix() {
        let fixture = Fixture::new("latest-prefix", RELEASE_PREFIXED, Some("release-"));
        let code = run_changelog_latest(
            Some(&fixture.config()),
            &fixture.changelog(),
            ChangelogFormat::Markdown { include_header: true },
        );
        assert_eq!(code, 0);
    }

    /// A machine reading `"commits": []` cannot tell "no commits" from "git was unavailable",
    /// so `--json` must fail closed instead of reporting an empty range as success.
    #[test]
    fn latest_json_fails_closed_when_git_history_is_unreadable() {
        let fixture = Fixture::new(
            "latest-json-no-git",
            "## [1.2.0]\n- new work\n\n## [1.1.0]\n- older work\n",
            None,
        );
        // A repository with no commit to answer `v1.1.0..HEAD`.
        crate::git::init_test_repo(&fixture.dir);
        assert_eq!(
            run_changelog_latest(Some(&fixture.config()), &fixture.changelog(), ChangelogFormat::Json),
            1
        );
    }

    #[test]
    fn show_json_fails_closed_when_the_git_range_is_unreadable() {
        let fixture = Fixture::new(
            "show-json-no-git",
            "## [1.2.0]\n- new work\n\n## [1.1.0]\n- older work\n\n## [1.0.0]\n- first work\n",
            None,
        );
        crate::git::init_test_repo(&fixture.dir);
        git(&fixture.dir, &["add", "."]);
        commit(&fixture.dir, "chore: initial");
        // The tags the range names do not exist, so git cannot answer `v1.0.0..v1.1.0`.
        assert_eq!(
            run_changelog_show(
                Some(&fixture.config()),
                &fixture.changelog(),
                "1.1.0",
                ChangelogFormat::Json
            ),
            1
        );
    }

    /// Failing closed must not cost the working read: a readable range still exits `0`.
    #[test]
    fn latest_json_succeeds_when_the_git_range_reads() {
        let fixture = Fixture::new(
            "latest-json-git",
            "## [1.2.0]\n- new work\n\n## [1.1.0]\n- older work\n",
            None,
        );
        crate::git::init_test_repo(&fixture.dir);
        git(&fixture.dir, &["add", "."]);
        commit(&fixture.dir, "chore: initial");
        git(&fixture.dir, &["tag", "v1.1.0"]);
        commit(&fixture.dir, "feat: new work");
        assert_eq!(
            run_changelog_latest(Some(&fixture.config()), &fixture.changelog(), ChangelogFormat::Json),
            0
        );
    }

    /// Template rendering keeps its long-standing degradation: it reports the changelog body and
    /// simply has no commit data to iterate over.
    #[test]
    fn latest_template_keeps_its_degradation_when_git_is_unreadable() {
        let fixture = Fixture::new(
            "latest-template-no-git",
            "## [1.2.0]\n- new work\n\n## [1.1.0]\n- older work\n",
            None,
        );
        let template = fixture.dir.join("notes.j2");
        std::fs::write(&template, "Release {{ version }}").unwrap();
        assert_eq!(
            run_changelog_latest(
                Some(&fixture.config()),
                &fixture.changelog(),
                ChangelogFormat::Template(template)
            ),
            0
        );
    }

    /// `open` does not read the changelog: it builds the release URL from `[git] tag_prefix`, which it
    /// already did before this slice. This pins that the tag it names is the one the heading carries.
    #[test]
    fn open_targets_the_tag_of_a_configured_prefix() {
        let fixture = Fixture::new("open-prefix", RELEASE_PREFIXED, Some("release-"));
        crate::git::init_test_repo(&fixture.dir);
        git(
            &fixture.dir,
            &["remote", "add", "origin", "https://github.com/acme/widget"],
        );
        let captured = fixture.dir.join("captured-url.txt");
        let browser = write_capture_script(&fixture.dir);
        let code = run_open(
            Some(&fixture.config()),
            Some("1.2.0"),
            false,
            Some(&browser.to_string_lossy()),
        );
        assert_eq!(code, 0);
        let url = std::fs::read_to_string(&captured).expect("the launcher received the URL");
        assert_eq!(url.trim(), "https://github.com/acme/widget/releases/tag/release-1.2.0");
    }

    /// Plain markdown output is a file read, so it never touches git: no repository is required.
    #[test]
    fn latest_markdown_does_not_need_a_repository() {
        let fixture = Fixture::new("latest-markdown-no-repo", "## [1.2.0] - 2026-03-01\n- new work\n", None);
        // No `git init` here on purpose: anything that reads history would fail.
        assert_eq!(
            run_changelog_latest(
                Some(&fixture.config()),
                &fixture.changelog(),
                ChangelogFormat::Markdown { include_header: true }
            ),
            0
        );
    }

    /// A version the changelog does not carry is a user error with its own exit code, not an
    /// unreadable range.
    #[test]
    fn show_json_reports_a_missing_version_as_exit_1() {
        let fixture = Fixture::new("show-json-missing", "## [1.2.0] - 2026-03-01\n- new work\n", None);
        crate::git::init_test_repo(&fixture.dir);
        git(&fixture.dir, &["add", "."]);
        commit(&fixture.dir, "chore: initial");
        assert_eq!(
            run_changelog_show(
                Some(&fixture.config()),
                &fixture.changelog(),
                "9.9.9",
                ChangelogFormat::Json
            ),
            1
        );
    }

    /// A heading that carries the default `v` marker is still read when the project's own prefix is
    /// something else: the prefix decides first, the legacy marker is the fallback.
    #[test]
    fn a_legacy_v_heading_is_found_under_another_prefix() {
        let fixture = Fixture::new(
            "legacy-v-heading",
            "## [v1.2.0] - 2026-03-01\n- new work\n\n## [v1.1.0] - 2026-02-01\n- older work\n",
            Some("release-"),
        );
        assert_eq!(
            run_changelog_show(
                Some(&fixture.config()),
                &fixture.changelog(),
                "1.2.0",
                ChangelogFormat::Markdown { include_header: false }
            ),
            0
        );
        assert_eq!(
            run_changelog_latest(
                Some(&fixture.config()),
                &fixture.changelog(),
                ChangelogFormat::Markdown { include_header: false }
            ),
            0
        );
    }

    /// S4: the requested version is compared exactly; surrounding whitespace is not silently
    /// accepted (baseline behaviour on the context path). The exact spelling is the control that
    /// proves exit `0` is reachable here.
    #[test]
    fn show_does_not_accept_a_padded_version() {
        let fixture = Fixture::new("show-padded", "## [1.2.0] - 2026-03-01\n- new work\n", None);
        crate::git::init_test_repo(&fixture.dir);
        git(&fixture.dir, &["add", "."]);
        commit(&fixture.dir, "chore: initial");
        // The single release entry reads `git log v1.2.0`, so the tag has to exist for exit `0` to
        // be reachable.
        git(&fixture.dir, &["tag", "v1.2.0"]);

        assert_eq!(
            run_changelog_show(
                Some(&fixture.config()),
                &fixture.changelog(),
                " 1.2.0 ",
                ChangelogFormat::Json
            ),
            1
        );
        assert_eq!(
            run_changelog_show(
                Some(&fixture.config()),
                &fixture.changelog(),
                "1.2.0",
                ChangelogFormat::Json
            ),
            0
        );
    }

    /// The fail-closed path names the read that failed, so the operator sees which range was
    /// unreadable rather than a bare exit code.
    #[test]
    fn the_git_failure_message_names_the_failed_read() {
        let err = crate::git::Error::Command {
            command: "log".to_string(),
            source: std::io::Error::other("git is not installed"),
        };
        assert_eq!(
            git_history_error(&err),
            "failed to read the release commit range from git: git command 'log' failed to run: git is not installed"
        );
    }

    /// The browser override receives the URL as its only argument; the script writes it to
    /// `captured-url.txt` next to itself.
    #[cfg(windows)]
    fn write_capture_script(dir: &Path) -> PathBuf {
        let path = dir.join("capture-url.cmd");
        std::fs::write(&path, "@echo %1>\"%~dp0captured-url.txt\"\r\n").unwrap();
        path
    }

    #[cfg(unix)]
    fn write_capture_script(dir: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("capture-url.sh");
        std::fs::write(
            &path,
            "#!/bin/sh\nprintf '%s' \"$1\" > \"$(dirname \"$0\")/captured-url.txt\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
}
