use std::sync::Arc;

use semver::Version;
use thiserror::Error;

use crate::plugin::manager::PluginManager;

pub mod cargo_toml;
pub mod gradle;
pub mod json;
pub mod json_scan;
pub mod plugin;
pub mod pyproject;
pub mod regex;

pub use plugin::PluginManifestEditor;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to parse {kind}: {detail}")]
    Parse { kind: &'static str, detail: String },
    #[error("field '{0}' not found")]
    FieldNotFound(String),
    #[error("field '{0}' is not a string")]
    NotAString(String),
    #[error("invalid version string '{0}': {1}")]
    InvalidVersion(String, semver::Error),
    #[error("target '{0}' not found")]
    TargetNotFound(String),
    #[error("no match for pattern '{0}'")]
    NoMatch(String),
    #[error("regex error: {0}")]
    Regex(#[from] ::regex::Error),
    #[error("plugin error: {0}")]
    Plugin(#[from] crate::plugin::PluginError),
    #[error("plugin manager is required to resolve plugin manifest '{0}'")]
    PluginManagerRequired(String),
    #[error("invalid plugin name '{0}': {1}")]
    InvalidPluginName(String, crate::plugin::types::PluginNameError),
}

pub trait ManifestEditor: std::fmt::Debug + Send + Sync {
    fn read_version(&self, content: &str) -> Result<Version, Error>;
    fn write_version(&self, content: &str, version: &Version) -> Result<String, Error>;
}

use crate::config;

pub fn editor_for(entry: &config::Manifest) -> Result<Box<dyn ManifestEditor>, Error> {
    editor_for_with_manager(entry, None)
}

pub fn editor_for_with_manager(
    entry: &config::Manifest,
    plugin_manager: Option<&Arc<PluginManager>>,
) -> Result<Box<dyn ManifestEditor>, Error> {
    match &entry.kind {
        config::ManifestKind::Json { field } => Ok(Box::new(json::JsonEditor::new(field.clone()))),
        config::ManifestKind::CargoPackage => Ok(Box::new(cargo_toml::CargoEditor)),
        config::ManifestKind::Gradle {
            version_name_field,
            version_code_field,
        } => Ok(Box::new(gradle::GradleEditor::try_new(
            version_name_field.clone(),
            version_code_field.clone(),
        )?)),
        config::ManifestKind::Regex { pattern, replacement } => {
            Ok(Box::new(regex::RegexEditor::new(pattern.clone(), replacement.clone())))
        }
        config::ManifestKind::Pyproject { table } => Ok(Box::new(pyproject::PyprojectEditor::new(table.clone()))),
        config::ManifestKind::Plugin { plugin } => {
            let manager = plugin_manager.ok_or_else(|| Error::PluginManagerRequired(entry.path.to_string()))?;
            let plugin_name = match plugin {
                Some(name) => {
                    let p_name = crate::plugin::types::PluginName::new(name)
                        .map_err(|e| Error::InvalidPluginName(name.clone(), e))?;
                    if manager.config(&p_name).is_none() {
                        return Err(crate::plugin::PluginError::PluginNotFound { name: p_name }.into());
                    }
                    p_name
                }
                None => {
                    let path_str = entry.path.as_str();
                    let filename = std::path::Path::new(path_str)
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or(path_str);
                    match manager.resolve_plugin_for_manifest(path_str) {
                        Ok(p) => p.clone(),
                        Err(e) => {
                            if filename != path_str {
                                if let Ok(p) = manager.resolve_plugin_for_manifest(filename) {
                                    p.clone()
                                } else {
                                    return Err(e.into());
                                }
                            } else {
                                return Err(e.into());
                            }
                        }
                    }
                }
            };
            Ok(Box::new(PluginManifestEditor::new(
                plugin_name,
                entry.path.to_string(),
                Arc::clone(manager),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Manifest, ManifestKind, ManifestPath};

    #[test]
    fn editor_for_pyproject() {
        let entry = Manifest {
            path: ManifestPath::parse("pyproject.toml").unwrap(),
            primary: true,
            kind: ManifestKind::Pyproject { table: None },
        };
        let editor = editor_for(&entry).expect("editor_for should succeed for pyproject");
        let content = "[project]\nname = \"demo\"\nversion = \"1.0.0\"\n";
        let v = editor.read_version(content).unwrap();
        assert_eq!(v, Version::parse("1.0.0").unwrap());
    }

    #[test]
    fn editor_for_pyproject_with_table() {
        let entry = Manifest {
            path: ManifestPath::parse("pyproject.toml").unwrap(),
            primary: true,
            kind: ManifestKind::Pyproject {
                table: Some("tool.poetry".into()),
            },
        };
        let editor = editor_for(&entry).expect("editor_for should succeed for pyproject with table");
        let content = "[tool.poetry]\nname = \"demo\"\nversion = \"2.0.0\"\n";
        let v = editor.read_version(content).unwrap();
        assert_eq!(v, Version::parse("2.0.0").unwrap());
    }
}
