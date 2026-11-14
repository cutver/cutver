use rayon::prelude::*;
use semver::Version;
use std::io;
use std::sync::Arc;

use crate::atomic;
use crate::bump::exec::command::read;
use crate::bump::exec::transaction::MutationTransaction;
use crate::bump::{Change, Error, Touched};
use crate::changelog;
use crate::config::Config;
use crate::manifest;
use crate::plugin::PluginManager;

pub struct SourceEntry<'a> {
    pub manifest: &'a crate::config::Manifest,
    pub path: String,
}

impl<'a> std::ops::Deref for SourceEntry<'a> {
    type Target = crate::config::Manifest;

    fn deref(&self) -> &Self::Target {
        self.manifest
    }
}

/// Single source of truth for mapping `config.version.current_source` to its
/// manifest entry, editor, and current version. Used by both `run` and `doctor`
/// to eliminate the previously duplicated lookup and error-mapping blocks.
pub(crate) fn current_source<'a>(
    config: &'a Config,
    plugin_manager: Option<&Arc<PluginManager>>,
) -> Result<(SourceEntry<'a>, Box<dyn manifest::ManifestEditor>, Version), Error> {
    let entry = config.primary_manifest().ok_or_else(|| Error::Read {
        path: config.version.current_source.clone(),
        source: io::Error::new(io::ErrorKind::NotFound, "current_source manifest entry not found"),
    })?;
    let content = read(&entry.path)?;
    let editor = manifest::editor_for_with_manager(entry, plugin_manager).map_err(|e| Error::CurrentSource {
        path: entry.path.to_string(),
        source: e,
    })?;
    let version = editor.read_version(&content).map_err(|e| Error::CurrentSource {
        path: entry.path.to_string(),
        source: e,
    })?;
    let source_entry = SourceEntry {
        manifest: entry,
        path: entry.path.to_string(),
    };
    Ok((source_entry, editor, version))
}

pub(crate) fn read_manifest(
    m: &crate::config::Manifest,
    plugin_manager: Option<&Arc<PluginManager>>,
) -> Result<(Box<dyn manifest::ManifestEditor>, String, Version), Error> {
    let content = read(&m.path)?;
    let editor = manifest::editor_for_with_manager(m, plugin_manager).map_err(|e| Error::Manifest {
        path: m.path.to_string(),
        source: e,
    })?;
    let version = editor.read_version(&content).map_err(|e| Error::Manifest {
        path: m.path.to_string(),
        source: e,
    })?;
    Ok((editor, content, version))
}

pub(crate) fn compute(
    config: &Config,
    next: &Version,
    context: Option<&changelog::ReleaseContext>,
    plugin_manager: Option<&Arc<PluginManager>>,
) -> Result<Vec<Change>, Error> {
    config
        .manifest
        .par_iter()
        .map(|m| {
            // A template sink generates its whole file from the release context: it has no version
            // to read and no in-place edit to make, so it never goes through `read_manifest`.
            if matches!(m.kind, crate::config::ManifestKind::Template { .. }) {
                return compute_template_sink(m, next, context, plugin_manager);
            }
            let (editor, content, old) = read_manifest(m, plugin_manager)?;
            let new = editor.write_version(&content, next).map_err(|e| Error::Manifest {
                path: m.path.to_string(),
                source: e,
            })?;
            let changed = content != new;
            Ok(Change {
                path: m.path.to_string(),
                old,
                original: Some(content),
                new,
                changed,
            })
        })
        .collect()
}

/// Reads a target that may legitimately not exist yet, and fails closed on anything else (#261).
///
/// An absent file is a normal state: a template sink is often generated for the first time. Any
/// other read failure is *not* normal, because the bytes returned here are the only copy rollback
/// will restore. Backing an unreadable file up as empty and writing that back would turn a real
/// manifest into a zero-byte file.
fn read_existing(path: &str) -> Result<Option<String>, Error> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::ReadUnreadable {
            path: path.to_string(),
            source,
        }),
    }
}

/// Renders one `kind = "template"` sink into a [`Change`] without reading a version from it.
///
/// `old` carries `next`: a sink has no version of its own, so the summary reports the version it was
/// generated for, and `changed` is what tells the pipeline whether a write is needed. A target that
/// does not exist yet is generated from scratch, with `original` empty so a rollback removes it
/// again: recreating a zero-byte file for something that was never there is a wrong restore, not a
/// safe one (#261).
fn compute_template_sink(
    m: &crate::config::Manifest,
    next: &Version,
    context: Option<&changelog::ReleaseContext>,
    plugin_manager: Option<&Arc<PluginManager>>,
) -> Result<Change, Error> {
    let path = m.path.to_string();
    let editor = manifest::editor_for_with_manager(m, plugin_manager).map_err(|e| Error::Manifest {
        path: path.clone(),
        source: e,
    })?;
    let Some(template_editor) = editor.as_template() else {
        return Err(Error::Manifest {
            path,
            source: manifest::Error::TemplateNeedsReleaseContext {
                path: m.path.to_string(),
            },
        });
    };
    let context = context.ok_or_else(|| Error::Manifest {
        path: path.clone(),
        source: manifest::Error::TemplateNeedsReleaseContext {
            path: m.path.to_string(),
        },
    })?;
    let new = template_editor.render(context).map_err(|e| Error::Manifest {
        path: path.clone(),
        source: e,
    })?;
    let original = read_existing(&path)?;
    let changed = original.as_deref().unwrap_or_default() != new;
    Ok(Change {
        path,
        old: next.clone(),
        original,
        new,
        changed,
    })
}

pub(crate) fn apply(
    computed: &[Change],
    next: &Version,
    dry_run: bool,
    transaction: &mut MutationTransaction<'_>,
) -> Result<(Vec<Touched>, Vec<String>), Error> {
    let mut touched = Vec::new();
    let mut paths = Vec::new();
    for c in computed {
        touched.push(Touched {
            path: c.path.clone(),
            old: c.old.to_string(),
            new: next.to_string(),
        });
        if !c.changed {
            continue;
        }
        if !dry_run {
            transaction.record_backup(c.path.clone(), c.original.clone());
            atomic::write_atomic(&c.path, &c.new).map_err(|e| Error::WriteRollback {
                path: c.path.clone(),
                source: e,
                rollback: "transaction rollback on drop.".into(),
            })?;
        }
        paths.push(c.path.clone());
    }
    Ok((touched, paths))
}
