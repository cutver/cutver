use std::path::Path;

use crate::config;
use crate::git::types::{strip_one_version_marker, strip_version_prefix};
use crate::git::{TagName, TagPrefix};

/// The prefix this read surface strips from headings and builds tags with.
///
/// The configured `[git] tag_prefix` is the authority. With no configuration there is nothing
/// declared, so the empty prefix is used: a heading may still carry the historical `v`/`V` marker,
/// and `git::types` stays the one module that decides what to strip.
pub fn resolve_prefix(cfg: Option<&config::Config>) -> TagPrefix {
    cfg.map(|c| TagPrefix::new(&c.git.tag_prefix)).unwrap_or_default()
}

pub fn load_template_file(config_override: Option<&Path>, template_path: &Path) -> Result<String, String> {
    let resolved = if template_path.is_absolute() {
        template_path.to_path_buf()
    } else {
        let cfg = config_override
            .and_then(|p| config::load(p).ok())
            .or_else(|| std::env::current_dir().ok().and_then(|d| config::discover(d).ok()));
        if let Some(ref c) = cfg {
            let candidate = c.root_dir.join(template_path);
            if candidate.is_file() {
                candidate
            } else {
                template_path.to_path_buf()
            }
        } else {
            template_path.to_path_buf()
        }
    };

    std::fs::read_to_string(&resolved)
        .map_err(|e| format!("failed to read template file '{}': {e}", template_path.display()))
}

pub fn extract_heading_date(content: &str, target_ver: &str, prefix: &TagPrefix) -> Option<String> {
    let target_norm = strip_one_version_marker(target_ver, prefix);
    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let after_h2 = heading.trim_start();
            let lower = after_h2.to_ascii_lowercase();
            if lower.starts_with("[unreleased]") || lower.starts_with("unreleased") {
                continue;
            }
            if let Some((ver_part, date_part)) = after_h2.split_once(" - ") {
                let v = strip_one_version_marker(ver_part.trim().trim_matches(['[', ']']), prefix);
                if v == target_norm {
                    let d = date_part.trim();
                    if !d.is_empty() {
                        return Some(d.to_string());
                    }
                }
            }
        }
    }
    None
}

pub fn find_raw_heading_version<'a>(content: &'a str, target_ver: &str, prefix: &TagPrefix) -> Option<&'a str> {
    let target_norm = strip_one_version_marker(target_ver, prefix);
    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let after_h2 = heading.trim_start();
            let lower = after_h2.to_ascii_lowercase();
            if lower.starts_with("[unreleased]") || lower.starts_with("unreleased") {
                continue;
            }
            let raw_ver = if after_h2.starts_with('[') {
                after_h2.find(']').map(|end| after_h2[1..end].trim())
            } else {
                after_h2
                    .split_whitespace()
                    .next()
                    .map(|s| s.trim_end_matches(':').trim())
            };
            if let Some(raw) = raw_ver
                && strip_one_version_marker(raw, prefix) == target_norm
            {
                return Some(raw);
            }
        }
    }
    None
}

pub fn resolve_release_tag_and_prefix(
    cfg: Option<&config::Config>,
    root_dir: &Path,
    content: &str,
    version: &str,
) -> (crate::git::TagName, crate::git::TagPrefix) {
    let prefix = resolve_prefix(cfg);
    let clean_ver = TagName::parse(version)
        .map(|t| t.normalize_version(&prefix).to_string())
        .unwrap_or_else(|_| strip_version_prefix(version, &prefix).to_string());

    if cfg.is_some() {
        return (prefix.format_tag(&clean_ver), prefix);
    }

    // Nothing is declared, so the default prefix a project without a config gets is the candidate
    // to probe first, followed by a bare tag, and finally the marker the heading itself carried.
    let default_prefix = TagPrefix::new(crate::config::Git::default().tag_prefix);
    let default_tag = default_prefix.format_tag(&clean_ver);
    if crate::git::tag_exists(root_dir, default_tag.as_str()).unwrap_or(false) {
        return (default_tag, default_prefix);
    }

    let empty_prefix = TagPrefix::default();
    let bare_tag = empty_prefix.format_tag(&clean_ver);
    if crate::git::tag_exists(root_dir, bare_tag.as_str()).unwrap_or(false) {
        return (bare_tag, empty_prefix);
    }

    if let Some(raw) = find_raw_heading_version(content, &clean_ver, &empty_prefix)
        && strip_one_version_marker(raw, &empty_prefix) != raw
    {
        return (default_tag, default_prefix);
    }

    (bare_tag, empty_prefix)
}
