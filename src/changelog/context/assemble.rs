use super::types::{AssembleContextParams, CommitContext, InterpolationContext, ReleaseContext};
use crate::conventional::ConventionalCommit;

impl InterpolationContext {
    /// Build an `InterpolationContext` from a `ReleaseContext`.
    pub fn from_release_context(ctx: &ReleaseContext, bump_level: &str, branch: Option<&str>) -> Self {
        InterpolationContext {
            version: ctx.version.clone(),
            previous_version: ctx.previous_version.clone(),
            tag: ctx.tag.clone(),
            previous_tag: ctx.previous_tag.clone(),
            bump_level: bump_level.to_string(),
            is_prerelease: ctx.is_prerelease,
            prerelease: ctx.prerelease.clone(),
            build: ctx.build.clone(),
            major: ctx.major,
            minor: ctx.minor,
            patch: ctx.patch,
            date: ctx.date.clone(),
            branch: branch.map(String::from),
            repo_owner: ctx.repo_owner.clone(),
            repo_name: ctx.repo_name.clone(),
            forge: ctx.forge.clone(),
            project_name: ctx.project_name.clone(),
        }
    }
}

/// Helper to parse SemVer breakdown components.
fn parse_semver_components(version: &str) -> (u64, u64, u64, bool, Option<String>, Option<String>) {
    super::types::parse_semver_parts(version, None, None, None, None, None, None)
}

struct Category {
    header: &'static str,
    items: Vec<String>,
}

fn init_categories() -> [Category; 8] {
    [
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
    ]
}

#[allow(clippy::too_many_arguments)]
pub fn build_context_with_raw_and_filter(
    version: &str,
    previous_version: Option<&str>,
    tag: &str,
    previous_tag: Option<&str>,
    date: &str,
    repository: Option<String>,
    commits: &[ConventionalCommit],
    raw_commits: Option<&[crate::git::RawCommit]>,
    contributors: Vec<String>,
    include_scopes: bool,
    fallback_entry: &str,
    ignore_release_commits: bool,
    ignore_scopes: &[String],
) -> ReleaseContext {
    let filtered_commits = super::enrich::filter_commits(commits, ignore_release_commits, ignore_scopes);
    let commits = &filtered_commits;
    let mut categories = init_categories();

    let mut commit_contexts = Vec::with_capacity(commits.len());
    let mut raw_iter = raw_commits.map(|raws| raws.iter());

    for c in commits {
        let matching_raw = if let Some(ref mut iter) = raw_iter {
            iter.find(|r| {
                if let Some(parsed) = ConventionalCommit::parse(&r.message) {
                    parsed == *c
                } else {
                    r.message.starts_with(&c.description)
                        || r.message.lines().next().is_some_and(|l| l.contains(&c.description))
                }
            })
        } else {
            None
        };

        let enriched =
            super::enrich::enrich_commit_context(CommitContext::from(c), matching_raw, Some(c), repository.as_deref());
        let item = format!("- {}", super::enrich::format_commit_line(&enriched, include_scopes));
        commit_contexts.push(enriched);

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

    let [c0, c1, c2, c3, c4, c5, c6, c7] = categories;
    let breaking = c0.items.join("\n");
    let features = c1.items.join("\n");
    let fixes = c2.items.join("\n");
    let perf = c3.items.join("\n");
    let refactor = c4.items.join("\n");
    let docs = c5.items.join("\n");
    let maintenance = c6.items.join("\n");
    let other = c7.items.join("\n");

    let categories = [c0, c1, c2, c3, c4, c5, c6, c7];
    let sections: Vec<String> = categories
        .iter()
        .filter(|cat| !cat.items.is_empty())
        .map(|cat| format!("{}\n{}", cat.header, cat.items.join("\n")))
        .collect();

    let all_changes = if sections.is_empty() {
        format!("- {}", fallback_entry.trim())
    } else {
        sections.join("\n\n")
    };

    let compare_url = match (&repository, previous_tag) {
        (Some(repo), Some(prev)) => Some(format!("{repo}/compare/{prev}...{tag}")),
        _ => None,
    };

    let (major, minor, patch, is_prerelease, prerelease, build) = parse_semver_components(version);

    let forge_meta = repository.as_deref().map(super::parse::parse_repo_forge);
    let repo_owner = forge_meta.as_ref().and_then(|f| f.owner.clone());
    let repo_name = forge_meta.as_ref().and_then(|f| f.repo.clone());
    let forge = forge_meta.as_ref().and_then(|f| f.forge.clone());
    let project_name = repo_name.clone();

    ReleaseContext {
        version: version.to_string(),
        previous_version: previous_version.map(String::from),
        tag: tag.to_string(),
        previous_tag: previous_tag.map(String::from),
        date: date.to_string(),
        compare_url,
        repository,
        features,
        fixes,
        breaking,
        perf,
        refactor,
        docs,
        maintenance,
        other,
        all_changes,
        commits: commit_contexts,
        contributors,
        major,
        minor,
        patch,
        is_prerelease,
        prerelease,
        build,
        repo_owner,
        repo_name,
        forge,
        project_name,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_context_with_filter(
    version: &str,
    previous_version: Option<&str>,
    tag: &str,
    previous_tag: Option<&str>,
    date: &str,
    repository: Option<String>,
    commits: &[ConventionalCommit],
    contributors: Vec<String>,
    include_scopes: bool,
    fallback_entry: &str,
    ignore_release_commits: bool,
    ignore_scopes: &[String],
) -> ReleaseContext {
    build_context_with_raw_and_filter(
        version,
        previous_version,
        tag,
        previous_tag,
        date,
        repository,
        commits,
        None,
        contributors,
        include_scopes,
        fallback_entry,
        ignore_release_commits,
        ignore_scopes,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_context_with_raw(
    version: &str,
    previous_version: Option<&str>,
    tag: &str,
    previous_tag: Option<&str>,
    date: &str,
    repository: Option<String>,
    commits: &[ConventionalCommit],
    raw_commits: Option<&[crate::git::RawCommit]>,
    contributors: Vec<String>,
    include_scopes: bool,
    fallback_entry: &str,
) -> ReleaseContext {
    build_context_with_raw_and_filter(
        version,
        previous_version,
        tag,
        previous_tag,
        date,
        repository,
        commits,
        raw_commits,
        contributors,
        include_scopes,
        fallback_entry,
        true,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_context(
    version: &str,
    previous_version: Option<&str>,
    tag: &str,
    previous_tag: Option<&str>,
    date: &str,
    repository: Option<String>,
    commits: &[ConventionalCommit],
    contributors: Vec<String>,
    include_scopes: bool,
    fallback_entry: &str,
) -> ReleaseContext {
    build_context_with_filter(
        version,
        previous_version,
        tag,
        previous_tag,
        date,
        repository,
        commits,
        contributors,
        include_scopes,
        fallback_entry,
        true,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_context_auto(
    version: &str,
    tag_prefix: &str,
    previous_tag: Option<&str>,
    date: Option<&str>,
    repository: Option<String>,
    commits: &[ConventionalCommit],
    contributors: Vec<String>,
    include_scopes: bool,
    fallback_entry: &str,
    ignore_release_commits: bool,
    ignore_scopes: &[String],
) -> ReleaseContext {
    let clean_version = version.strip_prefix(tag_prefix).unwrap_or(version);
    let tag = if version.starts_with(tag_prefix) {
        version.to_string()
    } else {
        format!("{tag_prefix}{version}")
    };
    let previous_version = previous_tag.map(|pt| pt.strip_prefix(tag_prefix).unwrap_or(pt));
    let today = crate::changelog::format_date(std::time::SystemTime::now());
    let date_str = date.unwrap_or(&today);

    build_context_with_filter(
        clean_version,
        previous_version,
        &tag,
        previous_tag,
        date_str,
        repository,
        commits,
        contributors,
        include_scopes,
        fallback_entry,
        ignore_release_commits,
        ignore_scopes,
    )
}

pub fn assemble_release_context(params: AssembleContextParams<'_>) -> ReleaseContext {
    build_context_with_raw_and_filter(
        params.version,
        params.prev_version,
        params.tag,
        params.prev_tag,
        params.date,
        params.repository,
        params.parsed_commits,
        params.raw_commits,
        params.contributors,
        params.changelog_config.include_scopes,
        &params.changelog_config.fallback_entry,
        params.changelog_config.ignore_release_commits,
        &params.changelog_config.ignore_scopes,
    )
}
