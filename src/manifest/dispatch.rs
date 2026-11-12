//! Kind-to-editor dispatch: the single place that knows every manifest format.

use super::plugin::PluginManifestEditor;
use super::{Error, ManifestEditor};
use super::{cargo_toml, gradle, json, pyproject, regex};
use crate::plugin::manager::PluginManager;
use std::sync::Arc;

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
