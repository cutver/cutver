//! The polymorphic editing contract every manifest kind implements (Pillar IV.3).

use super::Error;
use semver::Version;

pub trait ManifestEditor: std::fmt::Debug + Send + Sync {
    fn read_version(&self, content: &str) -> Result<Version, Error>;
    fn write_version(&self, content: &str, version: &Version) -> Result<String, Error>;
}
