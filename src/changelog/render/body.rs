use super::template::render_template;
use crate::changelog::Error;
use crate::changelog::context::ReleaseContext;
use crate::config::Changelog;
use crate::conventional::ConventionalCommit;
use std::fs;
use std::path::Path;

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

/// Render the Keep-a-Changelog section body for a release based on `config`, parsed `commits`,
/// and an optional authoritative `ReleaseContext`.
pub fn render_body(
    config: &Changelog,
    commits: &[ConventionalCommit],
    context_override: Option<&ReleaseContext>,
) -> Result<String, Error> {
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
        Ok(render_conventional(config, commits))
    }
}

/// Render the Keep-a-Changelog section body for a release using an authoritative `ReleaseContext`.
pub fn render_body_with_context(
    config: &Changelog,
    commits: &[ConventionalCommit],
    context: &ReleaseContext,
) -> Result<String, Error> {
    render_body(config, commits, Some(context))
}

pub(crate) fn render_conventional(config: &Changelog, commits: &[ConventionalCommit]) -> String {
    let filtered_commits =
        crate::changelog::context::filter_commits(commits, config.ignore_release_commits, &config.ignore_scopes);
    let commits = &filtered_commits;

    struct Category {
        header: &'static str,
        items: Vec<String>,
    }

    let mut categories = [
        Category {
            header: "### ⚠️ Breaking Changes",
            items: Vec::new(),
        },
        Category {
            header: "### Features",
            items: Vec::new(),
        },
        Category {
            header: "### Bug Fixes",
            items: Vec::new(),
        },
        Category {
            header: "### Performance Improvements",
            items: Vec::new(),
        },
        Category {
            header: "### Refactoring",
            items: Vec::new(),
        },
        Category {
            header: "### Documentation",
            items: Vec::new(),
        },
        Category {
            header: "### Maintenance",
            items: Vec::new(),
        },
        Category {
            header: "### Other Changes",
            items: Vec::new(),
        },
    ];

    for c in commits {
        let item = match (&c.scope, config.include_scopes) {
            (Some(scope), true) => format!("- **{}**: {}", scope, c.description.trim()),
            _ => format!("- {}", c.description.trim()),
        };

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
        .map(|cat| format!("{}\n{}", cat.header, cat.items.join("\n")))
        .collect();

    if sections.is_empty() {
        format!("- {}", config.fallback_entry.trim())
    } else {
        sections.join("\n\n")
    }
}
