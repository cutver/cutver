/// Extract the latest release notes from changelog `content`.
///
/// Scans `content` line-by-line looking for markdown level-2 headings starting with `## `.
/// Skips any unreleased heading (e.g. `## [Unreleased]` or `## Unreleased`, case-insensitive).
/// Identifies the first release heading and collects content until the next line starting with `## ` or EOF.
///
/// If `include_header` is false, excludes the header line and trims leading and trailing whitespace from the body.
/// If `include_header` is true, includes the header line and trims trailing whitespace.
/// Returns `None` if no release section exists.
pub fn extract_latest(content: &str, include_header: bool) -> Option<String> {
    let mut in_release = false;
    let mut collected = Vec::new();

    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ")
            && is_release_heading(heading)
        {
            if in_release {
                break;
            }
            in_release = true;
            if include_header {
                collected.push(line);
            }
            continue;
        }
        if in_release {
            collected.push(line);
        }
    }

    if !in_release {
        return None;
    }

    let joined = collected.join("\n");
    if include_header {
        Some(joined.trim_end().to_string())
    } else {
        Some(joined.trim().to_string())
    }
}

/// Extract release notes for a specific `target_version` from changelog `content`.
///
/// Normalizes `target_version` and heading versions by stripping optional leading `'v'` or `'V'`.
/// Scans `content` line-by-line looking for markdown headings starting with `## `.
/// Collects subsequent lines until the next line starting with `## ` or EOF.
///
/// If `include_header` is false, excludes the header line and trims leading and trailing whitespace from the body.
/// If `include_header` is true, includes the header line and trims trailing whitespace.
/// Returns `None` if no matching release heading is found.
pub fn extract_version(content: &str, target_version: &str, include_header: bool) -> Option<String> {
    let target_norm = normalize_version(target_version);
    if target_norm.is_empty() {
        return None;
    }

    let mut in_target = false;
    let mut collected = Vec::new();

    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if in_target {
                if is_release_heading(heading) {
                    break;
                }
            } else if extract_heading_version(heading)
                .is_some_and(|heading_ver| normalize_version(heading_ver) == target_norm)
            {
                in_target = true;
                if include_header {
                    collected.push(line);
                }
                continue;
            }
        }
        if in_target {
            collected.push(line);
        }
    }

    if !in_target {
        return None;
    }

    let joined = collected.join("\n");
    if include_header {
        Some(joined.trim_end().to_string())
    } else {
        Some(joined.trim().to_string())
    }
}

/// List all release versions present in changelog `content`.
///
/// Scans `content` line-by-line looking for markdown level-2 headings starting with `## `.
/// Skips unreleased headings (e.g. `## [Unreleased]` or `## Unreleased`, case-insensitive).
/// Extracts each version token with `extract_heading_version` and normalizes it with `normalize_version`.
/// Returns versions preserving the order they appear in the changelog.
pub fn list_versions(content: &str) -> Vec<String> {
    let mut versions = Vec::new();
    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if !is_release_heading(heading) {
                continue;
            }

            if let Some(ver_token) = extract_heading_version(heading) {
                let normalized = normalize_version(ver_token);
                if !normalized.is_empty() {
                    versions.push(normalized.to_string());
                }
            }
        }
    }
    versions
}

pub(crate) fn is_release_heading(heading: &str) -> bool {
    let after_h2 = heading.trim_start();
    let lower = after_h2.to_ascii_lowercase();
    if lower.starts_with("[unreleased]") || lower.starts_with("unreleased") {
        return false;
    }
    if let Some(token) = extract_heading_version(heading) {
        let norm = normalize_version(token);
        norm.chars().next().is_some_and(|c| c.is_ascii_digit())
    } else {
        false
    }
}

pub(crate) fn normalize_version(v: &str) -> &str {
    let trimmed = v.trim();
    if let Some(rest) = trimmed.strip_prefix(['v', 'V']) {
        rest
    } else {
        trimmed
    }
}

pub(crate) fn extract_heading_version(heading: &str) -> Option<&str> {
    let heading = heading.trim_start();
    if heading.starts_with('[') {
        let end = heading.find(']')?;
        Some(heading[1..end].trim())
    } else {
        let first = heading.split_whitespace().next()?;
        Some(first.trim_end_matches(':').trim())
    }
}
