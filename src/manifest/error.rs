//! The module's error vocabulary: every editor and the dispatch fail through this type.

#[derive(Debug, thiserror::Error)]
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
    #[error("failed to read template file '{path}': {source}")]
    TemplateRead {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "manifest '{path}' is generated from a template: it has no version of its own to read. Set [version] current_source to a manifest that holds the version"
    )]
    TemplateSinkHasNoVersion { path: String },
    #[error(
        "manifest '{path}' is generated from a template and needs the release context; render it through the bump pipeline instead of the version-only editor path"
    )]
    TemplateNeedsReleaseContext { path: String },
    #[error("failed to render template: {detail}")]
    TemplateRender { detail: String },
}
