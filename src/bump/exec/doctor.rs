use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::bump::exec::command::read;
use crate::bump::exec::manifest::{current_source, read_manifest};
use crate::bump::{ChangelogDrift, Drift, Error};
use crate::changelog;
use crate::config::Config;
use crate::git;

pub fn doctor(config: &Config) -> Result<Vec<Drift>, Error> {
    let (source_entry, _editor, expected) = current_source(config)?;
    let drifts: Vec<Option<Drift>> = config
        .manifest
        .par_iter()
        .filter(|m| m.path != source_entry.path)
        .map(|m| {
            let (_editor, _content, actual) = read_manifest(m)?;
            if actual != expected {
                Ok(Some(Drift {
                    path: m.path.to_string(),
                    expected: expected.to_string(),
                    actual: actual.to_string(),
                }))
            } else {
                Ok(None)
            }
        })
        .collect::<Result<Vec<Option<Drift>>, Error>>()?;

    Ok(drifts.into_iter().flatten().collect())
}

pub fn doctor_changelog(config: &Config) -> Result<ChangelogDrift, Error> {
    let changelog_path = match &config.changelog.path {
        Some(p) => {
            if Path::new(p).is_absolute() {
                PathBuf::from(p)
            } else {
                config.root_dir.join(p)
            }
        }
        None => config.root_dir.join("CHANGELOG.md"),
    };

    let content = read(&changelog_path)?;
    let changelog_versions = changelog::list_versions(&content);
    let git_tags = git::list_tags(&config.root_dir, Some(&config.git.tag_prefix))?;

    let changelog_set: HashSet<&str> = changelog_versions.iter().map(String::as_str).collect();
    let mut missing_in_changelog = Vec::new();
    let mut normalized_tags = HashSet::new();

    let prefix = git::TagPrefix::new(&config.git.tag_prefix);
    for tag in &git_tags {
        let Ok(tag_name) = git::TagName::parse(tag) else {
            continue;
        };
        if tag_name.is_floating_major(&prefix) {
            continue;
        }
        let normalized = tag_name.normalize_version(&prefix);
        if !changelog_set.contains(normalized) {
            missing_in_changelog.push(tag.clone());
        }
        normalized_tags.insert(normalized.to_string());
    }

    let mut orphan_sections = Vec::new();
    for version in &changelog_versions {
        if !normalized_tags.contains(version.as_str()) {
            orphan_sections.push(version.clone());
        }
    }

    Ok(ChangelogDrift {
        missing_in_changelog,
        orphan_sections,
    })
}
