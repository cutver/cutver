use crate::git::TagPrefix;

/// Extract the latest release notes from changelog `content` under the empty prefix.
///
/// With no configured `[git] tag_prefix` there is nothing to strip but the historical `v`/`V`
/// marker; [`extract_latest_with_prefix`] is the prefix-aware form.
pub fn extract_latest(content: &str, include_header: bool) -> Option<String> {
    extract_latest_with_prefix(content, include_header, &TagPrefix::default())
}

/// Extract the latest release notes from changelog `content`.
///
/// Scans `content` line-by-line looking for markdown level-2 headings starting with `## `.
/// Skips any unreleased heading (e.g. `## [Unreleased]` or `## Unreleased`, case-insensitive).
/// A heading names a release when its version token, stripped of `prefix`, starts with a digit.
/// Identifies the first release heading and collects content until the next line starting with `## ` or EOF.
///
/// If `include_header` is false, excludes the header line and trims leading and trailing whitespace from the body.
/// If `include_header` is true, includes the header line and trims trailing whitespace.
/// Returns `None` if no release section exists.
pub fn extract_latest_with_prefix(content: &str, include_header: bool, prefix: &TagPrefix) -> Option<String> {
    let mut in_release = false;
    let mut collected = Vec::new();

    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ")
            && is_release_heading(heading, prefix)
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

/// Extract release notes for a specific `target_version` from changelog `content` under the empty prefix.
///
/// With no configured `[git] tag_prefix` there is nothing to strip but the historical `v`/`V`
/// marker; [`extract_version_with_prefix`] is the prefix-aware form.
pub fn extract_version(content: &str, target_version: &str, include_header: bool) -> Option<String> {
    extract_version_with_prefix(content, target_version, include_header, &TagPrefix::default())
}

/// Extract release notes for a specific `target_version` from changelog `content`.
///
/// Normalizes `target_version` and heading versions with [`crate::git::types::strip_one_version_marker`],
/// so the configured `prefix` decides what a heading carries (`## release-1.2.0` names `1.2.0` under
/// `release-`), a `v`/`V` heading marker is still accepted, and no version is stripped twice.
/// Scans `content` line-by-line looking for markdown headings starting with `## `.
/// Collects subsequent lines until the next release heading or EOF.
///
/// If `include_header` is false, excludes the header line and trims leading and trailing whitespace from the body.
/// If `include_header` is true, includes the header line and trims trailing whitespace.
/// Returns `None` if no matching release heading is found.
pub fn extract_version_with_prefix(
    content: &str,
    target_version: &str,
    include_header: bool,
    prefix: &TagPrefix,
) -> Option<String> {
    let target_norm = normalize_version(target_version, prefix);
    if target_norm.is_empty() {
        return None;
    }

    let mut in_target = false;
    let mut collected = Vec::new();

    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if in_target {
                if is_release_heading(heading, prefix) {
                    break;
                }
            } else if extract_heading_version(heading)
                .is_some_and(|heading_ver| normalize_version(heading_ver, prefix) == target_norm)
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

/// List all release versions present in changelog `content` under the empty prefix.
///
/// With no configured `[git] tag_prefix` there is nothing to strip but the historical `v`/`V`
/// marker; [`list_versions_with_prefix`] is the prefix-aware form.
pub fn list_versions(content: &str) -> Vec<String> {
    list_versions_with_prefix(content, &TagPrefix::default())
}

/// List all release versions present in changelog `content`.
///
/// Scans `content` line-by-line looking for markdown level-2 headings starting with `## `.
/// Skips unreleased headings (e.g. `## [Unreleased]` or `## Unreleased`, case-insensitive).
/// Extracts each version token with `extract_heading_version` and strips `prefix` from it, so a
/// `## release-1.2.0` heading lists `1.2.0` under `[git] tag_prefix = "release-"`.
/// Returns versions preserving the order they appear in the changelog.
pub fn list_versions_with_prefix(content: &str, prefix: &TagPrefix) -> Vec<String> {
    let mut versions = Vec::new();
    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if !is_release_heading(heading, prefix) {
                continue;
            }

            if let Some(ver_token) = extract_heading_version(heading) {
                let normalized = normalize_version(ver_token, prefix);
                if !normalized.is_empty() {
                    versions.push(normalized.to_string());
                }
            }
        }
    }
    versions
}

pub(crate) fn is_release_heading(heading: &str, prefix: &TagPrefix) -> bool {
    let after_h2 = heading.trim_start();
    let lower = after_h2.to_ascii_lowercase();
    if lower.starts_with("[unreleased]") || lower.starts_with("unreleased") {
        return false;
    }
    if let Some(token) = extract_heading_version(heading) {
        let norm = normalize_version(token, prefix);
        norm.chars().next().is_some_and(|c| c.is_ascii_digit())
    } else {
        false
    }
}

pub(crate) fn normalize_version<'a>(v: &'a str, prefix: &TagPrefix) -> &'a str {
    crate::git::types::strip_one_version_marker(v.trim(), prefix)
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
