use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::config::discovery::strip_verbatim_prefix;
use crate::config::types::ConfigError;

/// Domain newtype representing a validated, forward-slash-normalized manifest path.
///
/// Invariants enforced at construction (*Parse, Don't Validate*):
/// 1. Backslashes (`\`) are normalized to forward slashes (`/`).
/// 2. Windows verbatim prefix (`\\?\` or `\\?\UNC\`) is stripped.
/// 3. Leading `./` prefixes are normalized (e.g. `./Cargo.toml` -> `Cargo.toml`).
/// 4. Cannot be empty or whitespace-only.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManifestPath(String);

impl ManifestPath {
    /// Parses and normalizes a manifest path string.
    pub fn parse(s: impl AsRef<str>) -> Result<Self, ConfigError> {
        let raw = s.as_ref();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(ConfigError::EmptyManifestPath);
        }

        let stripped_buf = strip_verbatim_prefix(std::path::PathBuf::from(trimmed));
        let mut normalized = stripped_buf.to_string_lossy().replace('\\', "/");

        while let Some(rest) = normalized.strip_prefix("./") {
            normalized = rest.to_string();
        }

        if normalized.is_empty() {
            return Err(ConfigError::EmptyManifestPath);
        }

        Ok(Self(normalized))
    }

    /// Borrows the manifest path as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Borrows the manifest path as a `std::path::Path`.
    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl Deref for ManifestPath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for ManifestPath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<Path> for ManifestPath {
    fn as_ref(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl Borrow<str> for ManifestPath {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ManifestPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for ManifestPath {
    type Error = ConfigError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for ManifestPath {
    type Error = ConfigError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl FromStr for ManifestPath {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl From<ManifestPath> for String {
    fn from(p: ManifestPath) -> Self {
        p.0
    }
}

impl PartialEq<str> for ManifestPath {
    fn eq(&self, other: &str) -> bool {
        self.0 == other || (other.contains('\\') && self.0 == other.replace('\\', "/"))
    }
}

impl PartialEq<&str> for ManifestPath {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other || (other.contains('\\') && self.0 == other.replace('\\', "/"))
    }
}

impl PartialEq<String> for ManifestPath {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other || (other.contains('\\') && self.0 == other.replace('\\', "/"))
    }
}

impl PartialEq<ManifestPath> for str {
    fn eq(&self, other: &ManifestPath) -> bool {
        other == self
    }
}

impl PartialEq<ManifestPath> for &str {
    fn eq(&self, other: &ManifestPath) -> bool {
        other == *self
    }
}

impl PartialEq<ManifestPath> for String {
    fn eq(&self, other: &ManifestPath) -> bool {
        other == self
    }
}

impl Serialize for ManifestPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ManifestPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::parse(s).map_err(serde::de::Error::custom)
    }
}

/// Domain newtype representing a validated package identity slug.
///
/// Invariants enforced at construction (*Parse, Don't Validate*):
/// 1. Cannot be empty or whitespace-only.
/// 2. Contains no whitespace and no path separators (`/` or `\`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageName(String);

impl PackageName {
    /// Parses and validates a package name slug.
    pub fn parse(s: impl AsRef<str>) -> Result<Self, ConfigError> {
        let raw = s.as_ref();
        if raw.trim().is_empty() {
            return Err(ConfigError::EmptyPackageName);
        }
        if raw.chars().any(|c| c.is_whitespace() || c == '/' || c == '\\') {
            return Err(ConfigError::InvalidPackageName(raw.to_string()));
        }
        Ok(Self(raw.to_string()))
    }

    /// Borrows the package name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for PackageName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for PackageName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for PackageName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for PackageName {
    type Error = ConfigError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for PackageName {
    type Error = ConfigError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl FromStr for PackageName {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl From<PackageName> for String {
    fn from(p: PackageName) -> Self {
        p.0
    }
}

impl Serialize for PackageName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for PackageName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::parse(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_relative_path() {
        let p = ManifestPath::parse("Cargo.toml").unwrap();
        assert_eq!(p.as_str(), "Cargo.toml");
        assert_eq!(p, "Cargo.toml");
        assert_eq!(p, *"Cargo.toml");
        assert_eq!(p, "Cargo.toml".to_string());
        assert_eq!("Cargo.toml", p);
        assert_eq!(*"Cargo.toml", p);
        assert_eq!("Cargo.toml".to_string(), p);
    }

    #[test]
    fn normalizes_backslashes() {
        let p = ManifestPath::parse(r"crates\core\Cargo.toml").unwrap();
        assert_eq!(p.as_str(), "crates/core/Cargo.toml");
    }

    #[test]
    fn strips_leading_dotslash() {
        let p = ManifestPath::parse("./Cargo.toml").unwrap();
        assert_eq!(p.as_str(), "Cargo.toml");

        let p2 = ManifestPath::parse(r".\crates\sub\Cargo.toml").unwrap();
        assert_eq!(p2.as_str(), "crates/sub/Cargo.toml");

        let p3 = ManifestPath::parse("././Cargo.toml").unwrap();
        assert_eq!(p3.as_str(), "Cargo.toml");
    }

    #[test]
    fn strips_verbatim_prefix() {
        let p = ManifestPath::parse(r"\\?\C:\project\Cargo.toml").unwrap();
        assert_eq!(p.as_str(), "C:/project/Cargo.toml");
    }

    #[test]
    fn empty_or_whitespace_fails() {
        assert!(matches!(ManifestPath::parse(""), Err(ConfigError::EmptyManifestPath)));
        assert!(matches!(
            ManifestPath::parse("   "),
            Err(ConfigError::EmptyManifestPath)
        ));
        assert!(matches!(ManifestPath::parse("./"), Err(ConfigError::EmptyManifestPath)));
        assert!(matches!(
            ManifestPath::parse(r".\"),
            Err(ConfigError::EmptyManifestPath)
        ));
    }

    #[test]
    fn trait_implementations() {
        let p: ManifestPath = "Cargo.toml".parse().unwrap();
        assert_eq!(&*p, "Cargo.toml");
        assert_eq!(p.as_ref() as &str, "Cargo.toml");
        assert_eq!(p.as_ref() as &Path, Path::new("Cargo.toml"));
        assert_eq!(format!("{p}"), "Cargo.toml");

        let s: String = p.clone().into();
        assert_eq!(s, "Cargo.toml");

        let from_str_res = ManifestPath::try_from("package.json").unwrap();
        assert_eq!(from_str_res.as_str(), "package.json");

        let from_string_res = ManifestPath::try_from("package.json".to_string()).unwrap();
        assert_eq!(from_string_res.as_str(), "package.json");
    }

    #[test]
    fn serde_roundtrip() {
        let original = ManifestPath::parse(r"crates\sub\Cargo.toml").unwrap();
        let serialized = serde_json::to_string(&original).unwrap();
        assert_eq!(serialized, r#""crates/sub/Cargo.toml""#);

        let deserialized: ManifestPath = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, original);

        let err: Result<ManifestPath, _> = serde_json::from_str(r#""""#);
        assert!(err.is_err());
    }

    #[test]
    fn package_name_accepts_slug() {
        let p = PackageName::parse("cutver-pdk").unwrap();
        assert_eq!(p.as_str(), "cutver-pdk");
        assert_eq!(format!("{p}"), "cutver-pdk");
        assert_eq!(&*p, "cutver-pdk");

        let from_str: PackageName = "cutver.core".parse().unwrap();
        assert_eq!(from_str.as_str(), "cutver.core");
    }

    #[test]
    fn package_name_rejects_empty_whitespace_and_separators() {
        assert!(matches!(PackageName::parse(""), Err(ConfigError::EmptyPackageName)));
        assert!(matches!(
            PackageName::parse("   "),
            Err(ConfigError::EmptyPackageName)
        ));
        for invalid in ["two words", " leading", "trailing ", "a/b", r"a\b"] {
            assert!(
                matches!(PackageName::parse(invalid), Err(ConfigError::InvalidPackageName(_))),
                "{invalid:?} should be rejected"
            );
        }
    }

    #[test]
    fn package_name_serde_roundtrip() {
        let original = PackageName::parse("cutver-registry").unwrap();
        let serialized = serde_json::to_string(&original).unwrap();
        assert_eq!(serialized, r#""cutver-registry""#);

        let deserialized: PackageName = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, original);

        let err: Result<PackageName, _> = serde_json::from_str(r#""two words""#);
        assert!(err.is_err());
    }

    #[test]
    fn ord_and_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        let p1 = ManifestPath::parse("Cargo.toml").unwrap();
        let p2 = ManifestPath::parse("./Cargo.toml").unwrap();
        set.insert(p1.clone());
        assert!(set.contains(&p2));

        let p3 = ManifestPath::parse("package.json").unwrap();
        assert!(p1 < p3);
    }
}
