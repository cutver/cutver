use super::types::ForgeMetadata;

/// Iteratively strip trailing `(#...)` PR/issue numbers from the end of a description.
/// Returns the cleaned description and the parsed PR numbers in innermost-first or outermost-first order?
/// Following Issue #101 specification:
/// Iteratively strips from the end (`rfind("(#")` and `ends_with(')')`), pushing each parsed number.
/// The first element pushed is the outermost PR number.
pub fn strip_trailing_pr_numbers(description: &str) -> (String, Vec<u64>) {
    let mut trimmed = description.trim();
    let mut prs = Vec::new();
    while let Some(open_paren) = trimmed.rfind("(#")
        && trimmed.ends_with(')')
    {
        let num_str = &trimmed[open_paren + 2..trimmed.len() - 1];
        if num_str.chars().all(|c| c.is_ascii_digit()) && !num_str.is_empty() {
            if let Ok(num) = num_str.parse::<u64>() {
                prs.push(num);
            }
            trimmed = trimmed[..open_paren].trim_end();
            continue;
        }
        break;
    }
    (trimmed.to_string(), prs)
}

/// Strip trailing `(#...)` PR number from a commit description.
/// Delegates to `strip_trailing_pr_numbers` for backward compatibility.
#[allow(dead_code)]
pub fn strip_trailing_pr_number(description: &str) -> String {
    strip_trailing_pr_numbers(description).0
}

/// Extract PR number from commit description or body.
pub fn extract_pr_number(description: &str, body: Option<&str>) -> Option<u64> {
    // 1. Check end of description for (#123)
    let desc_trimmed = description.trim();
    if let Some(open_paren) = desc_trimmed.rfind("(#")
        && desc_trimmed.ends_with(')')
    {
        let num_str = &desc_trimmed[open_paren + 2..desc_trimmed.len() - 1];
        if let Ok(num) = num_str.parse::<u64>() {
            return Some(num);
        }
    }

    // 2. Check for "Merge pull request #123"
    let check_pr = |s: &str| -> Option<u64> {
        let lower = s.to_ascii_lowercase();
        let target = "merge pull request #";
        if let Some(idx) = lower.find(target) {
            let rest = &s[idx + target.len()..];
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(num) = num_str.parse::<u64>() {
                return Some(num);
            }
        }
        let target2 = "pull request #";
        if let Some(idx) = lower.find(target2) {
            let rest = &s[idx + target2.len()..];
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(num) = num_str.parse::<u64>() {
                return Some(num);
            }
        }
        None
    };

    if let Some(pr) = check_pr(description) {
        return Some(pr);
    }
    if let Some(b) = body
        && let Some(pr) = check_pr(b)
    {
        return Some(pr);
    }

    None
}

/// Extract referenced issue numbers from description, body, and footers.
pub fn extract_issue_numbers(description: &str, body: Option<&str>, footers: &[(String, String)]) -> Vec<u64> {
    let mut issues = Vec::new();
    let mut keywords = [
        "fixes #",
        "fix #",
        "fixed #",
        "closes #",
        "close #",
        "closed #",
        "resolves #",
        "resolve #",
        "resolved #",
        "refs #",
        "ref #",
    ];
    keywords.sort_by_key(|b| std::cmp::Reverse(b.len()));

    let mut scan_text = |text: &str| {
        let lower = text.to_ascii_lowercase();
        for kw in &keywords {
            let mut search_idx = 0;
            while let Some(pos) = lower[search_idx..].find(kw) {
                let abs_pos = search_idx + pos;
                let rest = &text[abs_pos + kw.len()..];
                let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(num) = num_str.parse::<u64>()
                    && !issues.contains(&num)
                {
                    issues.push(num);
                }
                search_idx = abs_pos + kw.len();
            }
        }
    };

    scan_text(description);
    if let Some(b) = body {
        scan_text(b);
    }
    for (k, v) in footers {
        let combined = format!("{k}: {v}");
        scan_text(&combined);
        let space_combined = format!("{k} {v}");
        scan_text(&space_combined);
    }

    issues
}

/// Build forge web URLs for a pull request and commit hash.
pub fn build_urls(
    repo_url: Option<&str>,
    pr_number: Option<u64>,
    hash: Option<&str>,
) -> (Option<String>, Option<String>) {
    let repo = match repo_url {
        Some(r) => r.trim_end_matches('/'),
        None => return (None, None),
    };

    let is_gitlab = repo.contains("gitlab.com") || repo.contains("/-/");

    let pr_url = pr_number.map(|num| {
        if is_gitlab {
            format!("{repo}/-/merge_requests/{num}")
        } else {
            format!("{repo}/pull/{num}")
        }
    });

    let commit_url = hash.map(|h| {
        if is_gitlab {
            format!("{repo}/-/commit/{h}")
        } else {
            format!("{repo}/commit/{h}")
        }
    });

    (pr_url, commit_url)
}

/// Parse owner, repo, and forge type from a Git repository URL (HTTPS or SSH).
pub fn parse_repo_forge(url: &str) -> ForgeMetadata {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return ForgeMetadata {
            owner: None,
            repo: None,
            forge: None,
        };
    }

    let (host, path) = if let Some(stripped) = trimmed.strip_prefix("git@") {
        if let Some((h, p)) = stripped.split_once(':') {
            (h, p)
        } else {
            ("", stripped)
        }
    } else if let Some(stripped) = trimmed.strip_prefix("ssh://git@") {
        if let Some((h, p)) = stripped.split_once('/') {
            (h, p)
        } else {
            ("", stripped)
        }
    } else if let Some(stripped) = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
    {
        if let Some((h, p)) = stripped.split_once('/') {
            (h, p)
        } else {
            (stripped, "")
        }
    } else {
        ("", trimmed)
    };

    let forge = if host.contains("github.com") {
        Some("github".to_string())
    } else if host.contains("gitlab.com") {
        Some("gitlab".to_string())
    } else if host.contains("bitbucket.org") {
        Some("bitbucket".to_string())
    } else if host.contains("codeberg.org") {
        Some("codeberg".to_string())
    } else if host.contains("sourcehut.org") || host.contains("sr.ht") {
        Some("sourcehut".to_string())
    } else if !host.is_empty() {
        let h_lower = host.to_ascii_lowercase();
        if let Some(pos) = h_lower.find('.') {
            Some(h_lower[..pos].to_string())
        } else {
            Some(h_lower)
        }
    } else {
        None
    };

    let clean_path = path
        .trim_matches('/')
        .strip_suffix(".git")
        .unwrap_or(path.trim_matches('/'));
    let segments: Vec<&str> = clean_path.split('/').filter(|s| !s.is_empty()).collect();

    let (owner, repo) = match segments.len() {
        0 => (None, None),
        1 => (None, Some(segments[0].to_string())),
        _ => {
            let repo_name = segments[segments.len() - 1].to_string();
            let owner_name = segments[..segments.len() - 1].join("/");
            (Some(owner_name), Some(repo_name))
        }
    };

    ForgeMetadata { owner, repo, forge }
}
