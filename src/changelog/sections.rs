use crate::changelog::context::filter_commits;
use crate::changelog::render::{render_categorised, render_item};
use crate::config::Changelog;
use crate::conventional::ConventionalCommit;

/// One declared package's slice of the release range, ready to render (S3, S5).
#[derive(Debug, Clone, Copy)]
pub struct PackageSection<'a> {
    /// The declared `package` identity, rendered verbatim as the section heading.
    pub name: &'a str,
    /// The commits selected for this package, still unparsed-order-preserving.
    pub commits: &'a [ConventionalCommit],
}

/// True when the configured changelog mode can express package sections.
///
/// The plugin format and the custom-template modes receive flat commits and keep their current
/// output untouched; only the default conventional renderer learns the grouped layout (S3).
pub fn supports_package_sections(config: &Changelog) -> bool {
    config.format != "plugin"
        && config.template.is_none()
        && config.template_file.is_none()
        && config.mode != "template"
}

/// Render the default changelog body with one section per declared package (S3, S5, S6, S7).
///
/// Layout: `### <package>` followed by the conventional category buckets one heading level deeper,
/// then `### Other changes` carrying every commit that touches no declared manifest path, without
/// category subheadings. Packages appear in the order given, which the caller supplies in
/// declaration order. A package whose selected commits all filter out renders no heading at all,
/// and an empty range keeps the configured fallback entry.
pub fn render_body_with_packages(
    config: &Changelog,
    packages: &[PackageSection<'_>],
    unowned: &[ConventionalCommit],
) -> String {
    let mut rendered: Vec<String> = Vec::new();

    for package in packages {
        let commits = filter_commits(package.commits, config.ignore_release_commits, &config.ignore_scopes);
        if commits.is_empty() {
            continue;
        }
        let body = render_categorised(config, &commits, 4);
        rendered.push(format!("### {}\n{body}", package.name));
    }

    let unowned = filter_commits(unowned, config.ignore_release_commits, &config.ignore_scopes);
    if !unowned.is_empty() {
        let items: Vec<String> = unowned
            .iter()
            .map(|commit| render_item(commit, config.include_scopes))
            .collect();
        rendered.push(format!("### Other changes\n{}", items.join("\n")));
    }

    if rendered.is_empty() {
        format!("- {}", config.fallback_entry.trim())
    } else {
        rendered.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(message: &str) -> ConventionalCommit {
        ConventionalCommit::parse(message).expect("conventional commit")
    }

    #[test]
    fn packages_render_in_given_order_and_skip_empty_ones() {
        let alpha = [commit("feat(alpha): alpha feature")];
        let beta: [ConventionalCommit; 0] = [];
        let packages = [
            PackageSection {
                name: "alpha",
                commits: &alpha,
            },
            PackageSection {
                name: "beta",
                commits: &beta,
            },
        ];

        let body = render_body_with_packages(&Changelog::default(), &packages, &[]);
        assert_eq!(body, "### alpha\n#### Features\n- **alpha**: alpha feature");
        assert!(!body.contains("### beta"));
    }

    #[test]
    fn unowned_commits_render_under_other_changes_without_category_headings() {
        let unowned = [commit("ci: tune pipeline"), commit("chore(deps): bump serde")];
        let body = render_body_with_packages(&Changelog::default(), &[], &unowned);
        assert_eq!(body, "### Other changes\n- tune pipeline\n- **deps**: bump serde");
        assert!(!body.contains("####"));
    }

    #[test]
    fn filtered_out_commits_do_not_emit_a_heading() {
        let alpha = [commit("chore(release): v1.1.0")];
        let packages = [PackageSection {
            name: "alpha",
            commits: &alpha,
        }];

        let body = render_body_with_packages(&Changelog::default(), &packages, &[]);
        assert_eq!(body, "- Maintenance and updates.");
    }
}
