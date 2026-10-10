//! Install-time errors for `cutver plugin add`.
//!
//! These cover the preflight checks, the `--registry` base, and the artifact
//! cache writes and reads. They are embedded transparently by
//! [`PluginError`][super::PluginError], so their rendered text is unchanged.

use thiserror::Error;

use crate::plugin::types::{Capability, PluginName};

/// A failure raised while planning or applying a plugin installation.
#[derive(Debug, Error)]
pub enum InstallError {
    #[error(
        "plugin '{name}' is already declared under [plugins.{name}] in '{config_path}'.\n  \
        Where: preflighting a plugin installation against the existing configuration\n  \
        Fix: pass --force to replace the existing declaration, or remove it first with 'cutver plugin remove {name}'."
    )]
    PluginAlreadyConfigured { name: PluginName, config_path: String },

    #[error(
        "plugin '{name}' declares capability '{capability}', which requires at least one {flag} (the '{key}' key).\n  \
        Where: validating the '{key}' requirement before installing '{name}'\n  \
        Fix: pass at least one {flag} argument; capability '{capability}' cannot express an empty '{key}'."
    )]
    PluginFlagRequired {
        name: PluginName,
        capability: Capability,
        key: &'static str,
        flag: &'static str,
    },

    #[error(
        "registry index base '{base}' is not a valid absolute https:// URL: {reason}.\n  \
        Where: validating the --registry value\n  \
        Fix: pass an https:// base such as https://raw.githubusercontent.com/cutver/registry/main/plugins."
    )]
    IndexUrl { base: String, reason: String },

    #[error(
        "failed to write plugin artifact at '{path}': {reason}.\n  \
        Where: installing the verified plugin artifact into the cache\n  \
        Fix: ensure the plugin cache directory exists and is writable, then retry."
    )]
    ArtifactWrite { path: String, reason: String },

    #[error(
        "failed to read plugin artifact at '{path}': {reason}.\n  \
        Where: reading the existing plugin artifact before replacing it\n  \
        Fix: ensure the file is readable, or remove it before retrying."
    )]
    ArtifactRead { path: String, reason: String },
}
