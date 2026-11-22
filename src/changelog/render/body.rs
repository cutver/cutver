use super::template::render_template;
use crate::changelog::Error;
use crate::changelog::context::{ReleaseContext, filter_commits};
use crate::config::Changelog;
use crate::conventional::ConventionalCommit;
use crate::plugin::PluginManager;
use crate::plugin::types::PluginName;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn resolve_template_path(file_path: &str) -> std::path::PathBuf {
    let p = Path::new(file_path);
    if p.is_relative() {
        if p.exists() {
            p.to_path_buf()
        } else {
            match std::env::current_dir() {
                Ok(cd) => cd.join(p),
                Err(_) => p.to_path_buf(),
            }
        }
    } else {
        p.to_path_buf()
    }
}

fn load_template_string(config: &Changelog) -> Result<String, Error> {
    if let Some(ref t) = config.template {
        Ok(t.clone())
    } else if let Some(ref file_path) = config.template_file {
        let full_path = resolve_template_path(file_path);
        fs::read_to_string(&full_path).map_err(|e| Error::TemplateFileRead {
            path: full_path.display().to_string(),
            source: e,
        })
    } else {
        Ok(config.entry_template.clone())
    }
}

/// The workspace root a changelog plugin request advertises, resolved through `cwd`.
///
/// `std::env::current_dir` failing means the process has no directory to name, so the sandbox root the
/// request would advertise does not exist. That used to be flattened into an empty string: the guest
/// received a fabricated empty workspace and the cause was gone (#266 S9). A render that cannot name
/// its root fails with [`Error::CurrentDir`] instead. `cwd` is a parameter so both answers are
/// testable without changing the process directory — the seam `crate::plugin::cache` and
/// `crate::cli::ci` use for the same reason.
fn plugin_root_dir(cwd: impl FnOnce() -> io::Result<PathBuf>) -> Result<String, Error> {
    let dir = cwd().map_err(|source| Error::CurrentDir { source })?;
    Ok(dir.to_string_lossy().to_string())
}

/// Render the Keep-a-Changelog section body for a release based on `config`, parsed `commits`,
/// an optional authoritative `ReleaseContext`, and an optional `PluginManager`.
pub fn render_body_with_plugin(
    config: &Changelog,
    commits: &[ConventionalCommit],
    context_override: Option<&ReleaseContext>,
    plugin_manager: Option<&PluginManager>,
) -> Result<String, Error> {
    if config.format == "plugin" {
        let manager = plugin_manager.ok_or(Error::PluginManagerRequired)?;
        let plugin_name = match &config.plugin {
            Some(name) => {
                let p_name = PluginName::new(name).map_err(|e| Error::InvalidPluginName(name.clone(), e))?;
                if manager.config(&p_name).is_none() {
                    return Err(crate::plugin::PluginError::PluginNotFound { name: p_name }.into());
                }
                p_name
            }
            None => manager.resolve_changelog_plugin()?.clone(),
        };

        let fallback_ctx;
        let context = match context_override {
            Some(ctx) => ctx,
            None => {
                let today = crate::changelog::format_date(std::time::SystemTime::now());
                fallback_ctx = crate::changelog::context::build_context_with_filter(
                    "",
                    None,
                    "",
                    None,
                    &today,
                    None,
                    commits,
                    Vec::new(),
                    config.include_scopes,
                    &config.fallback_entry,
                    config.ignore_release_commits,
                    &config.ignore_scopes,
                );
                &fallback_ctx
            }
        };

        let root_dir = plugin_root_dir(std::env::current_dir)?;
        let req = context.to_changelog_render_request(root_dir);
        let res = manager.dispatch_changelog(&plugin_name, &req)?;
        return Ok(res.body);
    }

    if config.template.is_some() || config.template_file.is_some() || config.mode == "template" {
        let template_str = load_template_string(config)?;
        let fallback_ctx;
        let context = match context_override {
            Some(ctx) => ctx,
            None => {
                let today = crate::changelog::format_date(std::time::SystemTime::now());
                fallback_ctx = crate::changelog::context::build_context_with_filter(
                    "",
                    None,
                    "",
                    None,
                    &today,
                    None,
                    commits,
                    Vec::new(),
                    config.include_scopes,
                    &config.fallback_entry,
                    config.ignore_release_commits,
                    &config.ignore_scopes,
                );
                &fallback_ctx
            }
        };

        render_template(&template_str, context)
    } else {
        Ok(render_conventional(config, commits, context_override))
    }
}

/// Render the Keep-a-Changelog section body for a release based on `config`, parsed `commits`,
/// and an optional authoritative `ReleaseContext`.
pub fn render_body(
    config: &Changelog,
    commits: &[ConventionalCommit],
    context_override: Option<&ReleaseContext>,
) -> Result<String, Error> {
    render_body_with_plugin(config, commits, context_override, None)
}

/// Render the Keep-a-Changelog section body for a release using an authoritative `ReleaseContext`.
pub fn render_body_with_context(
    config: &Changelog,
    commits: &[ConventionalCommit],
    context: &ReleaseContext,
) -> Result<String, Error> {
    render_body(config, commits, Some(context))
}

/// Heading level of a root changelog section: `### Features` in the flat layout, and
/// `### <package>`, `### Other changes`, `### New Contributors` in the grouped one.
pub(crate) const SECTION_HEADING_LEVEL: usize = 3;

/// Heading level of the conventional category buckets in the flat, no-package layout.
pub(crate) const FLAT_CATEGORY_HEADING_LEVEL: usize = SECTION_HEADING_LEVEL;

/// Heading level of the conventional category buckets nested under a package section.
pub(crate) const PACKAGE_CATEGORY_HEADING_LEVEL: usize = SECTION_HEADING_LEVEL + 1;

pub(crate) fn render_conventional(
    config: &Changelog,
    commits: &[ConventionalCommit],
    context: Option<&ReleaseContext>,
) -> String {
    let filtered = filter_commits(commits, config.ignore_release_commits, &config.ignore_scopes);
    let mut body = render_categorised(config, &filtered, FLAT_CATEGORY_HEADING_LEVEL);
    if config.new_contributors
        && let Some(new_contrib_section) = context.and_then(|ctx| render_new_contributors(&ctx.first_time_contributors))
    {
        body.push_str("\n\n");
        body.push_str(&new_contrib_section);
    }
    body
}

/// Formats a single contributor name as a markdown list item bullet.
pub(crate) fn format_contributor_bullet(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.starts_with('@') || trimmed.contains(' ') {
        format!("- {trimmed}")
    } else {
        format!("- @{trimmed}")
    }
}

/// Renders the new contributors section if non-empty, shared across flat and grouped modes.
pub(crate) fn render_new_contributors(first_time_contributors: &[String]) -> Option<String> {
    if first_time_contributors.is_empty() {
        return None;
    }
    let items: Vec<String> = first_time_contributors
        .iter()
        .map(|name| format_contributor_bullet(name))
        .collect();
    Some(format!(
        "{} New Contributors\n{}",
        "#".repeat(SECTION_HEADING_LEVEL),
        items.join("\n")
    ))
}

/// Renders one changelog item line, shared by the flat and the per-package renderers.
pub(crate) fn render_item(commit: &ConventionalCommit, include_scopes: bool) -> String {
    match (&commit.scope, include_scopes) {
        (Some(scope), true) => format!("- **{}**: {}", scope, commit.description.trim()),
        _ => format!("- {}", commit.description.trim()),
    }
}

/// Render the conventional category buckets at the given markdown heading level.
///
/// `commits` must already be filtered: the caller filters once, at the point where it decides
/// whether a section exists at all, so grouping only changes the grouping (S14).
pub(crate) fn render_categorised(config: &Changelog, commits: &[ConventionalCommit], level: usize) -> String {
    let heading = "#".repeat(level);

    struct Category {
        title: &'static str,
        items: Vec<String>,
    }

    let mut categories = [
        Category {
            title: "⚠️ Breaking Changes",
            items: Vec::new(),
        },
        Category {
            title: "Features",
            items: Vec::new(),
        },
        Category {
            title: "Bug Fixes",
            items: Vec::new(),
        },
        Category {
            title: "Performance Improvements",
            items: Vec::new(),
        },
        Category {
            title: "Refactoring",
            items: Vec::new(),
        },
        Category {
            title: "Documentation",
            items: Vec::new(),
        },
        Category {
            title: "Maintenance",
            items: Vec::new(),
        },
        Category {
            title: "Other Changes",
            items: Vec::new(),
        },
    ];

    for c in commits {
        let item = render_item(c, config.include_scopes);

        if c.is_breaking {
            categories[0].items.push(item);
        } else {
            match c.commit_type.as_str() {
                "feat" => categories[1].items.push(item),
                "fix" => categories[2].items.push(item),
                "perf" => categories[3].items.push(item),
                "refactor" => categories[4].items.push(item),
                "docs" => categories[5].items.push(item),
                "chore" | "build" | "ci" | "test" => categories[6].items.push(item),
                _ => categories[7].items.push(item),
            }
        }
    }

    let sections: Vec<String> = categories
        .into_iter()
        .filter(|cat| !cat.items.is_empty())
        .map(|cat| format!("{heading} {}\n{}", cat.title, cat.items.join("\n")))
        .collect();

    if sections.is_empty() {
        format!("- {}", config.fallback_entry.trim())
    } else {
        sections.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// S9 (#266): `env::current_dir` failing used to be flattened into `root_dir: ""`, so a changelog
    /// plugin received a fabricated empty workspace and the cause was gone. The failure is typed now
    /// and carries its `io::Error`, which also means there is no value left to put in a request.
    #[test]
    fn a_missing_current_directory_is_a_typed_error_instead_of_an_empty_root_dir() {
        let error = plugin_root_dir(|| {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "the current directory no longer exists",
            ))
        })
        .expect_err("a directory that cannot be determined is not an empty one");

        assert!(
            matches!(error, Error::CurrentDir { .. }),
            "the failure keeps its own variant instead of borrowing a file-shaped one: {error:?}"
        );
        assert_eq!(
            error.to_string(),
            "failed to determine the current directory: the current directory no longer exists"
        );
        assert!(
            std::error::Error::source(&error).is_some(),
            "the cause travels with the error instead of being replaced by `\"\"`: {error:?}"
        );
    }

    /// The success path is unchanged: the request carries the directory the boundary reports, and its
    /// spelling stays the lossy string the plugin surface has always advertised.
    #[test]
    fn the_root_dir_is_the_directory_the_seam_reports() {
        let root = plugin_root_dir(|| Ok(std::path::PathBuf::from("/work/repo"))).expect("a reported directory");

        assert_eq!(root, "/work/repo");
        assert!(!root.is_empty(), "a resolved workspace root is never the empty string");
    }
}
