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
pub fn resolve_env_grants(plugin: &PluginName, declared: &[String]) -> Result<BTreeMap<String, String>, PluginError> {
    declared
        .iter()
        .map(|key| {
            std::env::var(key)
                .map(|value| (key.clone(), value))
                .map_err(|_| PluginError::UnsetEnvGrant {
                    name: plugin.clone(),
                    key: key.clone(),
                })
        })
        .collect()
}
