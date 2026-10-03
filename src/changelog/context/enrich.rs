use super::types::CommitContext;
use crate::conventional::ConventionalCommit;

/// Enriches a `CommitContext` with metadata from `RawCommit`, `ConventionalCommit`, and forge URLs.
pub fn enrich_commit_context(
    mut ctx: CommitContext,
    raw_commit: Option<&crate::git::RawCommit>,
    conventional: Option<&ConventionalCommit>,
    repo_url: Option<&str>,
) -> CommitContext {
    if let Some(raw) = raw_commit {
        ctx.hash = Some(raw.hash.clone());
        ctx.short_hash = Some(raw.short_hash.clone());
        ctx.author = Some(crate::git::resolve_author(&raw.author_name, &raw.author_email));
        ctx.author_email = Some(raw.author_email.clone());
    }

    let (body, footers) = match conventional {
        Some(c) => (c.body.as_deref(), c.footers.as_slice()),
        None => (None, &[][..]),
    };

    let raw_body = raw_commit.and_then(|r| r.message.split_once("\n\n").map(|(_, b)| b));
    let effective_body = body.or(raw_body);

    let (clean, prs) = super::parse::strip_trailing_pr_numbers(&ctx.description);
    ctx.clean_description = clean;

    if !prs.is_empty() {
        // Last/outermost PR tag is the PR number
        ctx.pr_number = Some(prs[0]);
        // Preceding PR tags become issue references if not already present
        for &issue_num in &prs[1..] {
            if !ctx.issue_numbers.contains(&issue_num) {
                ctx.issue_numbers.push(issue_num);
            }
        }
    } else {
        ctx.pr_number = super::parse::extract_pr_number(&ctx.description, effective_body);
    }

    for issue in super::parse::extract_issue_numbers(&ctx.description, effective_body, footers) {
        if !ctx.issue_numbers.contains(&issue) {
            ctx.issue_numbers.push(issue);
        }
    }

    let (pr_url, commit_url) = super::parse::build_urls(repo_url, ctx.pr_number, ctx.hash.as_deref());
    ctx.pr_url = pr_url;
    ctx.commit_url = commit_url;
    ctx.line = format_commit_line(&ctx, true);
    ctx.bullet = format!("- {}", ctx.line);

    ctx
}

/// Formats a single commit into a canonical, rich Markdown line.
/// Output format: `**scope**: clean description in [#123](url) ([abc1234](url)) by @author`
pub fn format_commit_line(ctx: &CommitContext, include_scope: bool) -> String {
    let mut line = String::new();
    if include_scope && let Some(scope) = &ctx.scope {
        line.push_str(&format!("**{scope}**: "));
    }

    let desc = if ctx.clean_description.is_empty() {
        &ctx.description
    } else {
        &ctx.clean_description
    };
    line.push_str(desc);

    if let Some(pr) = ctx.pr_number {
        match &ctx.pr_url {
            Some(url) => line.push_str(&format!(" in [#{pr}]({url})")),
            None => line.push_str(&format!(" in #{pr}")),
        }
    }

    if let Some(hash) = &ctx.short_hash {
        match &ctx.commit_url {
            Some(url) => line.push_str(&format!(" ([{hash}]({url}))")),
            None => line.push_str(&format!(" ({hash})")),
        }
    }

    if let Some(author) = &ctx.author
        && !author.is_empty()
    {
        line.push_str(&format!(" by @{author}"));
    }

    line
}

/// Filter commits based on release commit patterns and ignored scopes.
pub fn filter_commits(
    commits: &[ConventionalCommit],
    ignore_release_commits: bool,
    ignore_scopes: &[String],
) -> Vec<ConventionalCommit> {
    commits
        .iter()
        .filter(|c| {
            if ignore_release_commits {
                if c.commit_type.eq_ignore_ascii_case("chore") && c.scope.as_deref() == Some("release") {
                    return false;
                }

                if let Some(scope) = &c.scope
                    && scope.eq_ignore_ascii_case("release")
                {
                    return false;
                }

                let desc = c.description.trim();
                let lower_desc = desc.to_ascii_lowercase();
                if lower_desc.starts_with("release:") || lower_desc.starts_with("release ") {
                    return false;
                }

                if let Some(rest) = desc.strip_prefix(['v', 'V']) {
                    let rest = rest.trim_start();
                    if rest.starts_with(|ch: char| ch.is_ascii_digit()) {
                        return false;
                    }
                }
            }

            if !ignore_scopes.is_empty()
                && let Some(scope) = &c.scope
                && ignore_scopes.iter().any(|s| s.eq_ignore_ascii_case(scope))
            {
                return false;
            }

            true
        })
        .cloned()
        .collect()
}
