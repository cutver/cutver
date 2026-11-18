//! `[[artifact]]`: a file set a release must produce (#326 slice 3A).
//!
//! The declaration is additive and only says the files must exist: checksums and the target matrix
//! are deliberately absent, because `sha2` is a `plugins`-only optional dependency and the lean
//! build must stay lean (L6). `checksum = "SHA256SUMS"` can become a later key without a new table.

use serde::Deserialize;

use super::glob::{self, GlobError};
use super::types::Config;

/// One declared artifact, exactly as `cutver.toml` spells it.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Artifact {
    /// Workspace-relative glob (`**`, `*`, `?`) expanded against the files that exist at
    /// verification time.
    pub glob: String,
    /// Minimum number of files the glob must match.
    #[serde(default = "default_at_least")]
    pub at_least: usize,
    /// Optional human label, only used in messages.
    #[serde(default)]
    pub description: Option<String>,
}

fn default_at_least() -> usize {
    1
}

impl Artifact {
    /// The name a message uses for this artifact: its description when it has one, else its glob.
    pub fn label(&self) -> &str {
        self.description.as_deref().unwrap_or(&self.glob)
    }
}

/// How many files each declared artifact's glob matches right now, in declaration order.
///
/// One computation for `publish`'s verification and for the count `cutver plan` reports, so the
/// number the closing phase enforces and the number the plan observes cannot disagree (#326 S3, S4).
pub(crate) fn matched_counts(config: &Config) -> Result<Vec<usize>, GlobError> {
    config
        .artifact
        .iter()
        .map(|artifact| glob::expand(&config.root_dir, &artifact.glob).map(|files| files.len()))
        .collect()
}
