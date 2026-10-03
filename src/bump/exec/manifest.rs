use rayon::prelude::*;
use semver::Version;
use std::io;

use crate::atomic;
use crate::bump::exec::command::read;
use crate::bump::exec::transaction::MutationTransaction;
use crate::bump::{Change, Error, Touched};
use crate::config::Config;
use crate::manifest;

/// Single source of truth for mapping `config.version.current_source` to its
/// manifest entry, editor, and current version. Used by both `run` and `doctor`
/// to eliminate the previously duplicated lookup and error-mapping blocks.
pub(crate) fn current_source(
    config: &Config,
) -> Result<(&crate::config::Manifest, Box<dyn manifest::ManifestEditor>, Version), Error> {
    let entry = config.primary_manifest().ok_or_else(|| Error::Read {
        path: config.version.current_source.clone(),
        source: io::Error::new(io::ErrorKind::NotFound, "current_source manifest entry not found"),
    })?;
    let content = read(&entry.path)?;
    let editor = manifest::editor_for(entry).map_err(|e| Error::CurrentSource {
        path: entry.path.clone(),
        source: e,
    })?;
    let version = editor.read_version(&content).map_err(|e| Error::CurrentSource {
        path: entry.path.clone(),
        source: e,
    })?;
    Ok((entry, editor, version))
}

pub(crate) fn read_manifest(
    m: &crate::config::Manifest,
) -> Result<(Box<dyn manifest::ManifestEditor>, String, Version), Error> {
    let content = read(&m.path)?;
    let editor = manifest::editor_for(m).map_err(|e| Error::Manifest {
        path: m.path.clone(),
        source: e,
    })?;
    let version = editor.read_version(&content).map_err(|e| Error::Manifest {
        path: m.path.clone(),
        source: e,
    })?;
    Ok((editor, content, version))
}

pub(crate) fn compute(config: &Config, next: &Version) -> Result<Vec<Change>, Error> {
    config
        .manifest
        .par_iter()
        .map(|m| {
            let (editor, content, old) = read_manifest(m)?;
            let new = editor.write_version(&content, next).map_err(|e| Error::Manifest {
                path: m.path.clone(),
                source: e,
            })?;
            let changed = content != new;
            Ok(Change {
                path: m.path.clone(),
                old,
                original: content,
                new,
                changed,
            })
        })
        .collect()
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
