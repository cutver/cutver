//! Default template definitions and content constants for `cutver init`.

pub const STARTER_CHANGELOG: &str = "# Changelog

All notable changes to this project will be documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com).

## [Unreleased]
";

/// The release entry template Cutver scaffolds into new projects.
///
/// Embedded from the repository's own template so the file this project renders and the
/// file users receive cannot drift apart: one artifact, two consumers.
pub const DEFAULT_RELEASE_TEMPLATE: &str = include_str!("../../../.github/templates/cutver/RELEASE.md");

pub const DEFAULT_TEMPLATE_PATH: &str = ".github/templates/cutver/RELEASE.md";
