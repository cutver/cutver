//! Factory functions for creating plugin drivers according to their runtime configuration.

use std::collections::HashSet;
use std::sync::Arc;

use crate::plugin::driver::{DeclaredEnvGrants, EnvLookup, PluginDriver, ProcessDriver};
use crate::plugin::error::PluginError;
use crate::plugin::types::{PluginConfig, PluginName, RuntimeKind};

/// Default timeout in seconds for process plugins when unspecified in config.
pub const DEFAULT_PLUGIN_TIMEOUT_SECS: u64 = 30;

/// Creates a driver instance for a plugin based on its runtime configuration.
///
/// `lookup` is the environment seam every runtime resolves its declared grants through, so a test
/// drives the whole factory with an injected map instead of mutating the process environment. The
/// drivers keep it unresolved: the grants are handed over at invocation, never here (#300).
pub(crate) fn create_driver(
    name: &PluginName,
    config: &PluginConfig,
    lookup: &EnvLookup,
) -> Result<Box<dyn PluginDriver>, PluginError> {
    match config.runtime {
        RuntimeKind::Wasm => create_wasm_runtime_driver(name, config, lookup),
        RuntimeKind::Process => create_process_driver(name, config, lookup),
    }
}

#[cfg(not(feature = "plugins"))]
fn create_wasm_runtime_driver(
    name: &PluginName,
    _config: &PluginConfig,
    _lookup: &EnvLookup,
) -> Result<Box<dyn PluginDriver>, PluginError> {
    Err(PluginError::WasmNotSupported { name: name.clone() })
}

#[cfg(feature = "plugins")]
fn create_wasm_runtime_driver(
    name: &PluginName,
    config: &PluginConfig,
    lookup: &EnvLookup,
) -> Result<Box<dyn PluginDriver>, PluginError> {
    create_wasm_driver(name, config, lookup)
}

#[cfg(feature = "plugins")]
fn create_wasm_driver(
    name: &PluginName,
    config: &PluginConfig,
    lookup: &EnvLookup,
) -> Result<Box<dyn PluginDriver>, PluginError> {
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

    let driver = crate::plugin::driver::WasmDriver::from_env_lookup(
        name.clone(),
        &bytes,
        &config.permissions,
        config.timeout_seconds,
        capabilities,
        config.hash.as_deref(),
        Arc::clone(lookup),
    )?;

    Ok(Box::new(driver))
}

/// Creates a `ProcessDriver` from plugin configuration.
fn create_process_driver(
    name: &PluginName,
    config: &PluginConfig,
    lookup: &EnvLookup,
) -> Result<Box<dyn PluginDriver>, PluginError> {
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
    // The declared keys travel unresolved (#300). Resolution runs through the shared grant authority
    // when the child is spawned, and the child is spawned with a cleared environment, so the map
    // resolved there is still the complete set of keys the plugin can read.
    let grants = DeclaredEnvGrants::with_lookup(config.permissions.env.clone(), Arc::clone(lookup));

    Ok(Box::new(ProcessDriver::new(
        name.clone(),
        command,
        Vec::new(),
        grants,
        timeout_secs,
        capabilities,
    )))
}
