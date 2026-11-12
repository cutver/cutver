//! The polymorphic editing contract every manifest kind implements (Pillar IV.3).

use super::Error;
use semver::Version;

pub trait ManifestEditor: std::fmt::Debug + Send + Sync {
    fn read_version(&self, content: &str) -> Result<Version, Error>;
    fn write_version(&self, content: &str, version: &Version) -> Result<String, Error>;

    /// The template renderer behind this editor, when it is a `kind = "template"` sink.
    ///
    /// Defaults to `None` so the five in-place editors need no change: a sink is the only editor
    /// that generates its file from the release context instead of rewriting a version token.
    fn as_template(&self) -> Option<&super::TemplateEditor> {
        None
    }
}
