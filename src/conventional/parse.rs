use super::types::ConventionalCommit;

impl ConventionalCommit {
    /// Parse a commit message according to the Conventional Commits v1.0.0 specification.
    ///
    /// The header must match `<type>[(<scope>)][!]: <description>`.
    /// `!` in the header prefix or `BREAKING CHANGE:` / `BREAKING-CHANGE:` anywhere in
    /// the footers or body sets `is_breaking = true`.
    pub fn parse(message: &str) -> Option<Self> {
        let trimmed = message.trim();
        if trimmed.is_empty() {
            return None;
        }

        let (header, rest) = match trimmed.split_once('\n') {
            Some((h, r)) => (h.trim_end(), r),
            None => (trimmed, ""),
        };

        let (commit_type, scope, header_breaking, description) = parse_header(header)?;

        let paragraphs = split_paragraphs(rest);

        let mut footer_start_idx = paragraphs.len();
        while footer_start_idx > 0 {
            let p = &paragraphs[footer_start_idx - 1];
            let first_line = p.lines().next().unwrap_or("");
            if is_footer_start(first_line) {
                footer_start_idx -= 1;
            } else {
                break;
            }
        }

        let body = if footer_start_idx > 0 {
            Some(paragraphs[..footer_start_idx].join("\n\n"))
        } else {
            None
        };

        let mut footers = Vec::new();
        for p in &paragraphs[footer_start_idx..] {
            for line in p.lines() {
                if let Some((token, val)) = parse_footer_line(line) {
                    footers.push((token, val));
                } else if let Some(last) = footers.last_mut() {
                    last.1.push('\n');
                    last.1.push_str(line.trim());
                }
            }
        }

        let is_breaking = header_breaking
            || body.as_ref().is_some_and(|b| {
                b.contains("BREAKING CHANGE:")
                    || b.contains("BREAKING-CHANGE:")
                    || b.contains("BREAKING CHANGE :")
                    || b.contains("BREAKING-CHANGE :")
            })
            || footers.iter().any(|(k, v)| {
                k == "BREAKING CHANGE"
                    || k == "BREAKING-CHANGE"
                    || format!("{k}: {v}").contains("BREAKING CHANGE:")
                    || format!("{k}: {v}").contains("BREAKING-CHANGE:")
                    || format!("{k}: {v}").contains("BREAKING CHANGE :")
                    || format!("{k}: {v}").contains("BREAKING-CHANGE :")
            });

        Some(ConventionalCommit {
            commit_type,
            scope,
            is_breaking,
            description,
            body,
            footers,
        })
    }
}

pub(crate) fn parse_header(header: &str) -> Option<(String, Option<String>, bool, String)> {
    let (prefix, description) = header.split_once(": ")?;
    let description = description.trim();
    if description.is_empty() {
        return None;
    }

    let (prefix, header_breaking) = if let Some(stripped) = prefix.strip_suffix('!') {
        (stripped, true)
    } else {
        (prefix, false)
    };

    let (commit_type, scope) = if let Some(open_idx) = prefix.find('(') {
        if !prefix.ends_with(')') || open_idx == 0 || open_idx >= prefix.len() - 1 {
            return None;
        }
        let ctype = &prefix[..open_idx];
        let scope_content = &prefix[open_idx + 1..prefix.len() - 1];
        let trimmed_scope = scope_content.trim();
        if trimmed_scope.is_empty()
            || scope_content.contains('(')
            || scope_content.contains(')')
            || scope_content.contains('\n')
        {
            return None;
        }
        (ctype, Some(trimmed_scope.to_string()))
    } else {
        (prefix, None)
    };

    if commit_type.is_empty() || !commit_type.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return None;
    }

    Some((commit_type.to_string(), scope, header_breaking, description.to_string()))
}

pub(crate) fn split_paragraphs(text: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join("\n"));
                current.clear();
            }
        } else {
            current.push(line.trim_end().to_string());
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join("\n"));
    }
    paragraphs
}

pub(crate) fn is_footer_start(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.starts_with("BREAKING CHANGE:")
        || trimmed.starts_with("BREAKING-CHANGE:")
        || trimmed.starts_with("BREAKING CHANGE :")
        || trimmed.starts_with("BREAKING-CHANGE :")
    {
        return true;
    }
    if let Some((token, _)) = trimmed.split_once(": ") {
        return !token.is_empty()
            && !token.contains(char::is_whitespace)
            && token.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_');
    }
    if let Some((token, _)) = trimmed.split_once(" #") {
        return !token.is_empty()
            && !token.contains(char::is_whitespace)
            && token.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_');
    }
    false
}

pub(crate) fn parse_footer_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if let Some(rest) = trimmed
        .strip_prefix("BREAKING CHANGE:")
        .or_else(|| trimmed.strip_prefix("BREAKING CHANGE :"))
    {
        return Some(("BREAKING CHANGE".to_string(), rest.trim().to_string()));
    }
    if let Some(rest) = trimmed
        .strip_prefix("BREAKING-CHANGE:")
        .or_else(|| trimmed.strip_prefix("BREAKING-CHANGE :"))
    {
        return Some(("BREAKING-CHANGE".to_string(), rest.trim().to_string()));
    }
    if let Some((token, rest)) = trimmed.split_once(": ")
        && !token.is_empty()
        && !token.contains(char::is_whitespace)
        && token.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Some((token.to_string(), rest.trim().to_string()));
    }
    if let Some((token, rest)) = trimmed.split_once(" #")
        && !token.is_empty()
        && !token.contains(char::is_whitespace)
        && token.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Some((token.to_string(), format!("#{rest}").trim().to_string()));
    }
    None
}
