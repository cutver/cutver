use std::path::Path;

use crate::config;

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

pub fn extract_heading_date(content: &str, target_ver: &str) -> Option<String> {
    let target_norm = target_ver.trim_start_matches(['v', 'V']);
    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let after_h2 = heading.trim_start();
            let lower = after_h2.to_ascii_lowercase();
            if lower.starts_with("[unreleased]") || lower.starts_with("unreleased") {
                continue;
            }
            if let Some((ver_part, date_part)) = after_h2.split_once(" - ") {
                let v = ver_part.trim().trim_matches(['[', ']']).trim_start_matches(['v', 'V']);
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

pub fn find_raw_heading_version<'a>(content: &'a str, target_ver: &str) -> Option<&'a str> {
    let target_norm = target_ver.trim_start_matches(['v', 'V']);
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
                && raw.trim_start_matches(['v', 'V']) == target_norm
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
    let clean_ver = crate::git::TagName::parse(version)
        .map(|t| t.normalize_version(&crate::git::TagPrefix::default()).to_string())
        .unwrap_or_else(|_| version.trim_start_matches(['v', 'V']).to_string());

    if let Some(c) = cfg {
        let prefix = crate::git::TagPrefix::new(&c.git.tag_prefix);
        let tag = prefix.format_tag(&clean_ver);
        return (tag, prefix);
    }

    let v_prefix = crate::git::TagPrefix::new("v");
    let v_tag = v_prefix.format_tag(&clean_ver);
    if crate::git::tag_exists(root_dir, v_tag.as_str()).unwrap_or(false) {
        return (v_tag, v_prefix);
    }

    let empty_prefix = crate::git::TagPrefix::default();
    let bare_tag = empty_prefix.format_tag(&clean_ver);
    if crate::git::tag_exists(root_dir, bare_tag.as_str()).unwrap_or(false) {
        return (bare_tag, empty_prefix);
    }

    if let Some(raw) = find_raw_heading_version(content, &clean_ver)
        && raw.starts_with(['v', 'V'])
    {
        return (v_tag, v_prefix);
    }

    (bare_tag, empty_prefix)
}
