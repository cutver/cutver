pub mod process;
#[cfg(feature = "plugins")]
pub mod wasm;

pub use process::ProcessDriver;
#[cfg(feature = "plugins")]
pub use wasm::WasmDriver;

use std::collections::BTreeMap;

use crate::plugin::dto::PluginInvocation;
use crate::plugin::error::PluginError;
use crate::plugin::types::PluginName;

/// Unified driver trait for executing Cutver plugins across runtimes.
pub trait PluginDriver: std::fmt::Debug + Send + Sync {
    /// Returns the validated identifier of the plugin.
    fn name(&self) -> &PluginName;

    /// Invokes a capability on the plugin with the shared invocation envelope.
    fn invoke(&self, invocation: &PluginInvocation) -> Result<Vec<u8>, PluginError>;
}

/// Resolves the declared `[plugins.<name>.permissions] env` grants against the host environment.
///
/// Both runtimes resolve their grants here, so a grant means the same thing everywhere (Pillar
/// VI.1): only the declared keys cross the boundary, and each declared key must really exist.
/// A declared key that is unset is an error naming the plugin and the key, never a silent `""`,
/// because an empty grant hides a broken pipeline instead of failing closed (Pillar VI.2).
///
/// Boundary constructor binding the pure [`env_grants_from_lookup`] resolver to the process
/// environment, the same seam `crate::plugin::cache` and `crate::cli::ci` use.
pub fn resolve_env_grants(plugin: &PluginName, declared: &[String]) -> Result<BTreeMap<String, String>, PluginError> {
    env_grants_from_lookup(plugin, declared, |key| std::env::var(key).ok())
}

/// Pure resolver over an injected `lookup`, so a test never mutates the process environment.
///
/// A key the lookup cannot produce is [`PluginError::UnsetEnvGrant`] naming the plugin and the
/// key — both an absent variable and a value that is not valid UTF-8, which the `std::env::var`
/// boundary adapter collapses into `None`. A grant is therefore never a silent empty string
/// (Pillar VI.2).
pub fn env_grants_from_lookup(
    plugin: &PluginName,
    declared: &[String],
    lookup: impl Fn(&str) -> Option<String>,
) -> Result<BTreeMap<String, String>, PluginError> {
    declared
        .iter()
        .map(|key| {
            lookup(key)
                .map(|value| (key.clone(), value))
                .ok_or_else(|| PluginError::UnsetEnvGrant {
                    name: plugin.clone(),
                    key: key.clone(),
                })
        })
        .collect()
}
