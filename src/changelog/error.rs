//! The module's error vocabulary: reading, writing, extraction, rendering and plugins all fail
//! through this type.

use crate::atomic;
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read changelog '{path}': {source}")]
    Read {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("failed to write changelog '{path}': {source}")]
    Write {
        path: String,
        #[source]
        source: atomic::Error,
    },
    #[error("no release section found in changelog '{path}'")]
    NoReleaseSection { path: String },
    #[error("version '{version}' not found in changelog '{path}'")]
    VersionNotFound { version: String, path: String },
    #[error("failed to render changelog template: {detail}")]
    TemplateRender { detail: String },
    #[error("failed to read template file '{path}': {source}")]
    TemplateFileRead {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("plugin error: {0}")]
    Plugin(#[from] crate::plugin::PluginError),
    #[error("plugin manager is required to format changelog with format = \"plugin\"")]
    PluginManagerRequired,
    #[error("invalid plugin name '{0}': {1}")]
    InvalidPluginName(String, crate::plugin::types::PluginNameError),
}

/// Backwards-compatible alias kept for callers that spell the type with its module prefix.
pub type ChangelogError = Error;
