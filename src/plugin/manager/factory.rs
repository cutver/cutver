//! Factory functions for creating plugin drivers according to their runtime configuration.

use std::collections::{BTreeMap, HashSet};

use crate::plugin::driver::{PluginDriver, ProcessDriver};
use crate::plugin::error::PluginError;
use crate::plugin::types::{PluginConfig, PluginName, RuntimeKind};

/// Default timeout in seconds for process plugins when unspecified in config.
pub const DEFAULT_PLUGIN_TIMEOUT_SECS: u64 = 30;

/// Creates a driver instance for a plugin based on its runtime configuration.
pub(crate) fn create_driver(name: &PluginName, config: &PluginConfig) -> Result<Box<dyn PluginDriver>, PluginError> {
    match config.runtime {
        RuntimeKind::Wasm => create_wasm_runtime_driver(name, config),
        RuntimeKind::Process => create_process_driver(name, config),
    }
}

#[cfg(not(feature = "plugins"))]
fn create_wasm_runtime_driver(name: &PluginName, _config: &PluginConfig) -> Result<Box<dyn PluginDriver>, PluginError> {
    Err(PluginError::WasmNotSupported { name: name.clone() })
}

#[cfg(feature = "plugins")]
fn create_wasm_runtime_driver(name: &PluginName, config: &PluginConfig) -> Result<Box<dyn PluginDriver>, PluginError> {
    create_wasm_driver(name, config)
}

#[cfg(feature = "plugins")]
fn create_wasm_driver(name: &PluginName, config: &PluginConfig) -> Result<Box<dyn PluginDriver>, PluginError> {
    let source = config
        .source
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| PluginError::MissingConfiguration {
            name: name.clone(),
            reason: "'source' is required for wasm runtime and cannot be empty".to_string(),
        })?;

    let bytes = crate::plugin::driver::wasm::load_wasm_bytes(name, source, config.hash.as_deref())?;
    let capabilities = config.capabilities.iter().copied().collect::<HashSet<_>>();

    let driver = crate::plugin::driver::WasmDriver::new(
        name.clone(),
        &bytes,
        &config.permissions,
        config.timeout_seconds,
        capabilities,
        config.hash.as_deref(),
    )?;

    Ok(Box::new(driver))
}

/// Creates a `ProcessDriver` from plugin configuration.
fn create_process_driver(name: &PluginName, config: &PluginConfig) -> Result<Box<dyn PluginDriver>, PluginError> {
    let command = config
        .command
        .as_deref()
        .filter(|cmd| !cmd.trim().is_empty())
        .ok_or_else(|| PluginError::MissingConfiguration {
            name: name.clone(),
            reason: "'command' is required for process runtime and cannot be empty".to_string(),
        })?;

    let timeout_secs = config.timeout_seconds.unwrap_or(DEFAULT_PLUGIN_TIMEOUT_SECS);
    let capabilities = config.capabilities.iter().copied().collect::<HashSet<_>>();

    Ok(Box::new(ProcessDriver::new(
        name.clone(),
        command,
        Vec::new(),
        BTreeMap::new(),
        timeout_secs,
        capabilities,
    )))
}
