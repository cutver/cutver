use std::fmt;
use std::ops::Deref;
use std::process::ExitStatus;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TagError {
    #[error("git tag cannot be empty")]
    Empty,
    #[error("git tag cannot contain whitespace or control characters: '{0}'")]
    InvalidCharacter(String),
    #[error("git tag cannot contain '..': '{0}'")]
    ContainsDoubleDot(String),
    #[error("git tag cannot start or end with '/': '{0}'")]
    SlashBoundary(String),
    #[error("git tag cannot end with '.lock': '{0}'")]
    EndsWithLock(String),
    #[error("git tag cannot end with a dot: '{0}'")]
    EndsWithDot(String),
    #[error("git tag cannot be '@': '{0}'")]
    IsAt(String),
    #[error("git tag cannot contain forbidden ref characters (~, ^, :, ?, *, [, \\, @{{): '{0}'")]
    ForbiddenRefChar(String),
}

/// Validates that a string is a valid Git tag reference name according to git-check-ref-format rules.
fn validate_git_tag_name(s: &str) -> Result<(), TagError> {
    if s.is_empty() {
        return Err(TagError::Empty);
    }
    if s == "@" {
        return Err(TagError::IsAt(s.to_string()));
    }
    if s.starts_with('/') || s.ends_with('/') {
        return Err(TagError::SlashBoundary(s.to_string()));
    }
    if s.ends_with('.') {
        return Err(TagError::EndsWithDot(s.to_string()));
    }
    if s.ends_with(".lock") {
        return Err(TagError::EndsWithLock(s.to_string()));
    }
    if s.contains("..") {
        return Err(TagError::ContainsDoubleDot(s.to_string()));
    }
    if s.contains("@{") {
        return Err(TagError::ForbiddenRefChar(s.to_string()));
    }
    for c in s.chars() {
        if c.is_whitespace() || c.is_control() {
            return Err(TagError::InvalidCharacter(s.to_string()));
        }
        if matches!(c, '~' | '^' | ':' | '?' | '*' | '[' | '\\') {
            return Err(TagError::ForbiddenRefChar(s.to_string()));
        }
    }
    Ok(())
}

/// TagPrefix domain newtype wrapping a git tag prefix (e.g. "v", "", "release/").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub struct TagPrefix(String);

impl TagPrefix {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self(prefix.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn format_tag(&self, version: &str) -> TagName {
        let tag_str = if version == "HEAD" || self.0.is_empty() || version.starts_with(&self.0) {
            version.to_string()
        } else {
            format!("{}{version}", self.0)
        };
        TagName(tag_str)
    }
}

impl Deref for TagPrefix {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for TagPrefix {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TagPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for TagPrefix {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for TagPrefix {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// TagName domain newtype wrapping a validated Git tag ref string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TagName(String);

impl TagName {
    pub fn parse(s: impl AsRef<str>) -> Result<Self, TagError> {
        let text = s.as_ref();
        validate_git_tag_name(text)?;
        Ok(Self(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn strip_prefix<'a>(&'a self, prefix: &TagPrefix) -> Option<&'a str> {
        if prefix.as_str().is_empty() {
            Some(&self.0)
        } else {
            self.0.strip_prefix(prefix.as_str())
        }
    }

    pub fn normalize_version<'a>(&'a self, prefix: &TagPrefix) -> &'a str {
        let mut norm = self.0.as_str();
        if !prefix.is_empty()
            && let Some(rest) = norm.strip_prefix(prefix.as_str())
        {
            norm = rest;
        }
        if let Some(rest) = norm.strip_prefix(['v', 'V']) {
            norm = rest;
        }
        norm
    }

    pub fn is_floating_major(&self, prefix: &TagPrefix) -> bool {
        let mut s = self.0.as_str();
        if !prefix.is_empty() {
            if let Some(rest) = s.strip_prefix(prefix.as_str()) {
                s = rest;
            } else {
                return false;
            }
        } else if let Some(rest) = s.strip_prefix(['v', 'V']) {
            s = rest;
        }
        !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
    }
}

impl Deref for TagName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for TagName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TagName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for TagName {
    type Error = TagError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for TagName {
    type Error = TagError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_git_tag_name(&value)?;
        Ok(Self(value))
    }
}

impl FromStr for TagName {
    type Err = TagError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// CommitSha domain newtype representing a validated Git commit SHA (40 or 64 hex characters).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CommitSha(String);

impl CommitSha {
    /// Parses and validates a 40-character (SHA-1) or 64-character (SHA-256) hexadecimal Git commit SHA.
    pub fn parse(s: impl AsRef<str>) -> Result<Self, Error> {
        let text = s.as_ref();
        if text.len() != 40 && text.len() != 64 {
            return Err(Error::InvalidSha {
                sha: text.to_string(),
                reason: format!("expected 40 or 64 hex characters, got length {}", text.len()),
            });
        }
        if let Some(c) = text.chars().find(|c| !c.is_ascii_hexdigit()) {
            return Err(Error::InvalidSha {
                sha: text.to_string(),
                reason: format!("contains non-hexadecimal character '{c}'"),
            });
        }
        Ok(Self(text.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn short(&self) -> &str {
        let len = self.0.len().min(7);
        &self.0[..len]
    }
}

impl Deref for CommitSha {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for CommitSha {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommitSha {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for CommitSha {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for CommitSha {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl FromStr for CommitSha {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Raw representation of a Git commit parsed from history logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawCommit {
    pub hash: String,
    pub short_hash: String,
    pub author_name: String,
    pub author_email: String,
    pub message: String,
}

impl RawCommit {
    /// Attempts to parse the commit hash into a strongly-typed `CommitSha`.
    pub fn commit_sha(&self) -> Result<CommitSha, Error> {
        CommitSha::parse(&self.hash)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("guard failed: {detail}")]
    Guard { detail: String },
    #[error("git command '{command}' failed to run: {source}")]
    Command { command: String, source: std::io::Error },
    #[error("git command '{command}' exited with status {status}")]
    Status { command: String, status: ExitStatus },
    #[error("failed to read git output: {source}")]
    Output { source: std::io::Error },
    #[error("invalid commit sha '{sha}': {reason}")]
    InvalidSha { sha: String, reason: String },
}
