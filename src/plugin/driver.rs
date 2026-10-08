pub mod process;
#[cfg(feature = "plugins")]
pub mod wasm;

pub use process::ProcessDriver;
#[cfg(feature = "plugins")]
pub use wasm::WasmDriver;

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
