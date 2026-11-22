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
        let cfg = optional_config(config_override).map_err(|error| {
            format!(
                "failed to read the configuration that resolves template file '{}': {error}",
                template_path.display()
            )
        })?;
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

/// The configuration this read surface resolves for one invocation, reported as a failure the command
/// can return.
///
/// The typed failure is printed here because the read surfaces answer in exit codes, the same way
/// `dispatch::load_config` does; what changed with #266 S9 is that a configuration which *exists* and
/// does not load is no longer read as "this workspace declares none".
pub fn load_optional_config(config_override: Option<&Path>) -> Result<Option<config::Config>, i32> {
    optional_config(config_override).map_err(|error| {
        super::dispatch::print_error(
            crate::cli::machine::FailureCode::Changelog,
            format!("failed to read the configuration this read resolves against: {error}"),
        );
        1
    })
}

/// The same resolution, keeping the typed error so each caller can name what it was resolving.
///
/// `None` is the domain answer "this workspace declares no configuration": `changelog latest` and
/// `show` read a changelog that may live outside any cutver workspace, so discovery finding nothing
/// is not a failure. Anything else is the failure `.ok()` used to convert into that answer, after
/// which the read carried on from defaults (#266 S7, S9; the `.ok()` half of #267 absorbed by S11).
fn optional_config(config_override: Option<&Path>) -> Result<Option<config::Config>, config::ConfigError> {
    let loaded = match config_override {
        Some(path) => config::load(path),
        None => match std::env::current_dir() {
            Ok(dir) => config::discover(dir),
            // No directory to discover from is the same "no configuration" answer.
            Err(_) => return Ok(None),
        },
    };
    match loaded {
        Ok(config) => Ok(Some(config)),
        Err(config::ConfigError::NotFound(_)) => Ok(None),
        Err(error) => Err(error),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// S9, S11: the template read resolved the workspace root through the same `.ok()`-discarded
    /// configuration, so a document that did not load silently fell back to resolving the template
    /// against whatever directory the process was launched from. It fails closed now.
    #[test]
    fn the_template_read_fails_closed_when_the_configuration_does_not_load() {
        let dir = tempfile::Builder::new()
            .prefix("cutver-template-config-")
            .tempdir()
            .expect("a temp dir");
        let config = dir.path().join("cutver.toml");
        std::fs::write(&config, "[version\ncurrent_source = \"package.json\"\n").expect("the document is written");
        std::fs::write(dir.path().join("notes.j2"), "Release {{ version }}").expect("the template is written");

        let error = match load_template_file(Some(&config), Path::new("notes.j2")) {
            Ok(_) => panic!("a configuration that does not load must not be read as no configuration"),
            Err(error) => error,
        };

        assert!(
            error.contains("failed to parse config file"),
            "the error carries the configuration failure instead of falling back: {error}"
        );
    }
}
