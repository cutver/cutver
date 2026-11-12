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

/// True when `description` is a bare version token such as `v1.2.0`, `2026.11.12`,
/// `2026.11`, `1.2.3-rc.1`, `2.0.0+build.5` or `v1.0.0_rc1`, optionally followed by marker
/// groups such as `[skip ci]`, `[SKIP CI]`, `[bump: minor]` or `(hotfix)`.
///
/// Cutver's protocol carries the version as a plain string, so a versioning plugin may
/// produce any scheme; this therefore tests the *shape* of a version, never SemVer. A single
/// token without a version separator (`step1`, `http2`) is prose, not a version.
pub(super) fn is_bare_version(description: &str) -> bool {
    let trimmed = description.trim();
    let (token, tail) = match trimmed.split_once(char::is_whitespace) {
        Some((token, tail)) => (token, tail),
        None => (trimmed, ""),
    };

    is_marker_tail(tail) && is_version_shaped(token.strip_prefix(['v', 'V']).unwrap_or(token))
}

/// True when `tail` is empty or only `[...]`/`(...)` marker groups separated by whitespace.
fn is_marker_tail(tail: &str) -> bool {
    let mut rest = tail.trim_start();
    while !rest.is_empty() {
        let closer = match rest.as_bytes()[0] {
            b'[' => ']',
            b'(' => ')',
            _ => return false,
        };
        let Some(offset) = rest[1..].find(closer) else {
            return false;
        };
        rest = rest[1 + offset + 1..].trim_start();
    }

    true
}

/// True when `token` (already stripped of its optional `v`) looks like a version under any scheme.
fn is_version_shaped(token: &str) -> bool {
    const SEPARATORS: [char; 5] = ['.', '+', '-', '_', '~'];

    if token.is_empty() || !token.chars().any(|ch| ch.is_ascii_digit()) {
        return false;
    }

    if !token.chars().all(|ch| ch.is_alphanumeric() || SEPARATORS.contains(&ch)) {
        return false;
    }

    token.chars().any(|ch| SEPARATORS.contains(&ch)) || token.chars().all(|ch| ch.is_ascii_digit())
}

/// Filter commits based on release commit patterns and ignored scopes.
///
/// A commit is self-referential only when it is a `chore` whose scope is `release` **and**
/// whose description is nothing but a version token: the release commit Cutver itself writes.
/// `fix(release)`, `feat(release)` and `chore(release)` commits carrying content stay.
pub fn filter_commits(
    commits: &[ConventionalCommit],
    ignore_release_commits: bool,
    ignore_scopes: &[String],
) -> Vec<ConventionalCommit> {
    commits
        .iter()
        .filter(|c| {
            if ignore_release_commits
                && c.commit_type.eq_ignore_ascii_case("chore")
                && c.scope
                    .as_deref()
                    .is_some_and(|scope| scope.eq_ignore_ascii_case("release"))
                && is_bare_version(&c.description)
            {
                return false;
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
