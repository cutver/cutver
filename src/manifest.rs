//! Manifest version editing: the polymorphic contract, the kind dispatch and the editors.
//!
//! A pure facade, per Contract Pillar IV.4. Every other file under `manifest/` is a focused
//! submodule: [`error`] owns the failure vocabulary, [`editor`] the [`ManifestEditor`] contract,
//! [`dispatch`] the only `match` that knows every kind, and one module per format implements it.
//! This file declares and re-exports; it carries no implementation.

pub mod cargo_toml;
mod dispatch;
mod editor;
mod error;
pub mod gradle;
pub mod json;
pub mod json_scan;
pub mod plugin;
pub mod pyproject;
pub mod regex;
pub mod template;

#[cfg(test)]
mod tests;

pub use dispatch::{editor_for, editor_for_with_manager};
pub use editor::ManifestEditor;
pub use error::Error;
pub use plugin::PluginManifestEditor;
pub use template::TemplateEditor;
