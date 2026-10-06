use crate::plugin::error::PluginError;
use crate::plugin::types::PluginName;

/// Unified driver trait for executing Cutver plugins across runtimes.
pub trait PluginDriver: std::fmt::Debug + Send + Sync {
    /// Returns the validated identifier of the plugin.
    fn name(&self) -> &PluginName;

    /// Invokes a capability on the plugin passing raw serialized JSON bytes.
    fn invoke(&self, capability: &str, payload: &[u8]) -> Result<Vec<u8>, PluginError>;
}
