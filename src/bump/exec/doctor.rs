use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rayon::prelude::*;

use crate::bump::exec::command::read;
use crate::bump::exec::manifest::{current_source, read_manifest};
use crate::bump::{ChangelogDrift, Drift, Error};
use crate::changelog;
use crate::config::Config;
use crate::git;
use crate::plugin::PluginManager;

pub fn doctor(config: &Config) -> Result<Vec<Drift>, Error> {
    let plugin_manager = Arc::new(PluginManager::from_config(&config.plugins)?);
    let (source_entry, _editor, expected) = current_source(config, Some(&plugin_manager))?;
    let template_context = once_cell_lazy_template_context(config, &expected);
    let drifts: Vec<Option<Drift>> = config
        .manifest
        .par_iter()
        .filter(|m| m.path != source_entry.path)
        .map(|m| {
            // A template sink has no version to read: it is verified by rendering the template and
            // comparing bytes, never by reverse-parsing the generated file (S3).
            if matches!(m.kind, crate::config::ManifestKind::Template { .. }) {
                return template_sink_drift(m, template_context.as_ref(), &expected, Some(&plugin_manager));
            }
            let (_editor, _content, actual) = read_manifest(m, Some(&plugin_manager))?;
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

/// The context a template sink is verified against.
///
/// Only what is recoverable after the fact is filled in: the expected version, its tag and the
/// remote. The commit set is empty and the date is unknown, so a template that interpolates
/// `date` or commit-derived fields can be reported as drifted after the fact. That limit is
/// documented in the manifest reference instead of hidden behind a green check.
fn once_cell_lazy_template_context(config: &Config, expected: &semver::Version) -> Option<changelog::ReleaseContext> {
    let tag = git::tag_name(&config.git.tag_prefix, &expected.to_string());
    let version = expected.to_string();
    let repository = git::remote_url(&config.root_dir);
    Some(changelog::assemble_release_context(changelog::AssembleContextParams {
        version: &version,
        prev_version: None,
        tag: &tag,
        prev_tag: None,
        date: "",
        repository,
        parsed_commits: &[],
        raw_commits: None,
        contributors: Vec::new(),
        first_time_contributors: Vec::new(),
        changelog_config: &config.changelog,
    }))
}

/// Drift for one sink: render the template for `expected` and compare with the file on disk.
fn template_sink_drift(
    m: &crate::config::Manifest,
    context: Option<&changelog::ReleaseContext>,
    expected: &semver::Version,
    plugin_manager: Option<&Arc<PluginManager>>,
) -> Result<Option<Drift>, Error> {
    let path = m.path.to_string();
    let editor = crate::manifest::editor_for_with_manager(m, plugin_manager).map_err(|e| Error::Manifest {
        path: path.clone(),
        source: e,
    })?;
    let Some(template_editor) = editor.as_template() else {
        return Ok(None);
    };
    let Some(context) = context else {
        return Ok(None);
    };
    let rendered = template_editor.render(context).map_err(|e| Error::Manifest {
        path: path.clone(),
        source: e,
    })?;
    let on_disk = match std::fs::read_to_string(m.path.as_str()) {
        Ok(content) => content,
        // A sink that does not exist yet is not drift: the next run generates it.
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => String::new(),
        // Never claim the rendered output differs: an unreadable sink is a different problem, and
        // reporting it as drift would send the operator to regenerate a file they must repair first.
        Err(source) => {
            return Ok(Some(Drift {
                path,
                expected: expected.to_string(),
                actual: format!("cannot be read: {source}"),
            }));
        }
    };
    if rendered == on_disk {
        return Ok(None);
    }
    Ok(Some(Drift {
        path,
        expected: expected.to_string(),
        actual: "template output differs".to_string(),
    }))
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
