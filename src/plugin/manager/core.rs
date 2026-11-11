//! Core `PluginManager` orchestration and capability dispatch.

use std::collections::HashMap;

use super::factory::create_driver;
use super::glob::glob_matches;
use crate::plugin::driver::PluginDriver;
use crate::plugin::dto::{
    ChangelogRenderRequest, ChangelogRenderResponse, ManifestReadRequest, ManifestReadResponse, ManifestWriteRequest,
    ManifestWriteResponse, PluginInvocation, PostBumpPayload, PostReleasePayload, PreBumpPayload, PreBumpResponse,
    VersioningComputeRequest, VersioningComputeResponse,
};
use crate::plugin::error::PluginError;
use crate::plugin::types::{Capability, PluginCall, PluginConfig, PluginName};

/// Central facade managing plugin drivers and configuration orchestration.
#[derive(Debug)]
pub struct PluginManager {
    drivers: HashMap<PluginName, Box<dyn PluginDriver>>,
    configs: HashMap<PluginName, PluginConfig>,
}

impl PluginManager {
    /// Creates a new `PluginManager` directly from pre-instantiated drivers and configs.
    pub fn new(
        drivers: HashMap<PluginName, Box<dyn PluginDriver>>,
        configs: HashMap<PluginName, PluginConfig>,
    ) -> Self {
        Self { drivers, configs }
    }

    /// Instantiates a `PluginManager` from a map of plugin declarations.
    pub fn from_config(plugins: &HashMap<PluginName, PluginConfig>) -> Result<Self, PluginError> {
        let mut drivers: HashMap<PluginName, Box<dyn PluginDriver>> = HashMap::new();
        let mut configs = HashMap::new();

        for (name, config) in plugins {
            let driver = create_driver(name, config)?;
            drivers.insert(name.clone(), driver);
            configs.insert(name.clone(), config.clone());
        }

        Ok(Self { drivers, configs })
    }

    /// Returns the declared configuration for a plugin if registered.
    pub fn config(&self, name: &PluginName) -> Option<&PluginConfig> {
        self.configs.get(name)
    }

    /// Returns the driver instance for a plugin if registered.
    pub fn driver(&self, name: &PluginName) -> Option<&dyn PluginDriver> {
        self.drivers.get(name).map(|d| d.as_ref())
    }

    /// Returns an iterator over all registered plugin names.
    pub fn plugin_names(&self) -> impl Iterator<Item = &PluginName> {
        self.drivers.keys()
    }

    /// Returns an iterator over plugin names that declared `Capability::LifecycleV1`
    /// and whose `events` list is empty or explicitly contains the specified `event`.
    pub fn plugins_for_event<'a>(&'a self, event: &'a str) -> impl Iterator<Item = &'a PluginName> {
        self.configs.iter().filter_map(move |(name, config)| {
            let has_lifecycle = config.capabilities.contains(&Capability::LifecycleV1);
            let matches_event = config.events.is_empty() || config.events.iter().any(|e| e == event);
            if has_lifecycle && matches_event {
                Some(name)
            } else {
                None
            }
        })
    }

    /// Resolves the plugin responsible for handling a manifest path.
    ///
    /// Finds plugins declaring capability `manifest.v1` whose `manifest_match`
    /// patterns match `path`.
    /// - If 1 match found, returns `&PluginName`.
    /// - If 0 matches, returns `Err(PluginError::NoPluginForManifest)`.
    /// - If >1 matches, returns `Err(PluginError::AmbiguousManifestPlugin)`.
    pub fn resolve_plugin_for_manifest(&self, path: &str) -> Result<&PluginName, PluginError> {
        let mut matched: Vec<&PluginName> = self
            .configs
            .iter()
            .filter(|(_name, config)| {
                config.capabilities.contains(&Capability::ManifestV1)
                    && config.manifest_match.iter().any(|pattern| glob_matches(pattern, path))
            })
            .map(|(name, _config)| name)
            .collect();

        matched.sort();

        match matched.len() {
            1 => Ok(matched[0]),
            0 => Err(PluginError::NoPluginForManifest { path: path.to_string() }),
            _ => Err(PluginError::AmbiguousManifestPlugin {
                path: path.to_string(),
                matches: matched.into_iter().cloned().collect(),
            }),
        }
    }

    /// Finds plugins declaring capability `changelog.v1`.
    /// - If 1 match found, returns `&PluginName`.
    /// - If 0 matches, returns `Err(PluginError::NoChangelogPlugin)`.
    /// - If >1 matches, returns `Err(PluginError::AmbiguousChangelogPlugin)`.
    pub fn resolve_changelog_plugin(&self) -> Result<&PluginName, PluginError> {
        let mut matched: Vec<&PluginName> = self
            .configs
            .iter()
            .filter(|(_name, config)| config.capabilities.contains(&Capability::ChangelogV1))
            .map(|(name, _config)| name)
            .collect();

        matched.sort();

        match matched.len() {
            1 => Ok(matched[0]),
            0 => Err(PluginError::NoChangelogPlugin),
            _ => Err(PluginError::AmbiguousChangelogPlugin {
                matches: matched.into_iter().cloned().collect(),
            }),
        }
    }

    /// Finds plugins declaring capability `versioning.v1`.
    /// - If 1 match found, returns `&PluginName`.
    /// - If 0 matches, returns `Err(PluginError::NoVersioningPlugin)`.
    /// - If >1 matches, returns `Err(PluginError::AmbiguousVersioningPlugin)`.
    pub fn resolve_versioning_plugin(&self) -> Result<&PluginName, PluginError> {
        let mut matched: Vec<&PluginName> = self
            .configs
            .iter()
            .filter(|(_name, config)| config.capabilities.contains(&Capability::VersioningV1))
            .map(|(name, _config)| name)
            .collect();

        matched.sort();

        match matched.len() {
            1 => Ok(matched[0]),
            0 => Err(PluginError::NoVersioningPlugin),
            _ => Err(PluginError::AmbiguousVersioningPlugin {
                matches: matched.into_iter().cloned().collect(),
            }),
        }
    }

    /// Dispatches a `manifest.read` request to the target plugin.
    pub fn dispatch_manifest_read(
        &self,
        name: &PluginName,
        req: &ManifestReadRequest,
    ) -> Result<ManifestReadResponse, PluginError> {
        self.dispatch_raw(name, PluginCall::ManifestRead, req)
    }

    /// Dispatches a `manifest.write` request to the target plugin.
    pub fn dispatch_manifest_write(
        &self,
        name: &PluginName,
        req: &ManifestWriteRequest,
    ) -> Result<ManifestWriteResponse, PluginError> {
        self.dispatch_raw(name, PluginCall::ManifestWrite, req)
    }

    /// Dispatches a lifecycle `pre_bump` check to the target plugin.
    pub fn dispatch_pre_bump(
        &self,
        name: &PluginName,
        payload: &PreBumpPayload,
    ) -> Result<PreBumpResponse, PluginError> {
        self.dispatch_raw(name, PluginCall::PreBump, payload)
    }

    /// Dispatches a lifecycle `post_bump` notification to the target plugin.
    pub fn dispatch_post_bump(&self, name: &PluginName, payload: &PostBumpPayload) -> Result<(), PluginError> {
        let _: serde_json::Value = self.dispatch_raw(name, PluginCall::PostBump, payload)?;
        Ok(())
    }

    /// Dispatches a lifecycle `post_release` notification to the target plugin.
    pub fn dispatch_post_release(&self, name: &PluginName, payload: &PostReleasePayload) -> Result<(), PluginError> {
        let _: serde_json::Value = self.dispatch_raw(name, PluginCall::PostRelease, payload)?;
        Ok(())
    }

    /// Dispatches a changelog rendering request to the target plugin.
    pub fn dispatch_changelog(
        &self,
        name: &PluginName,
        req: &ChangelogRenderRequest,
    ) -> Result<ChangelogRenderResponse, PluginError> {
        self.dispatch_raw(name, PluginCall::ChangelogRender, req)
    }

    /// Dispatches a versioning compute request to the target plugin.
    pub fn dispatch_versioning(
        &self,
        name: &PluginName,
        req: &VersioningComputeRequest,
    ) -> Result<VersioningComputeResponse, PluginError> {
        self.dispatch_raw(name, PluginCall::VersioningCompute, req)
    }

    /// Helper dispatch method validating capability and executing IPC over driver.
    fn dispatch_raw<Req: serde::Serialize, Res: serde::de::DeserializeOwned>(
        &self,
        name: &PluginName,
        call: PluginCall,
        payload: &Req,
    ) -> Result<Res, PluginError> {
        let driver = self
            .drivers
            .get(name)
            .ok_or_else(|| PluginError::PluginNotFound { name: name.clone() })?;

        let capability = call.capability();
        validate_capability(self.configs.get(name), name, capability)?;

        let invocation = PluginInvocation::new(call, payload).map_err(|source| PluginError::InvalidPayload {
            name: name.clone(),
            capability: capability.to_string(),
            source,
        })?;

        let response_bytes = driver.invoke(&invocation)?;
        serde_json::from_slice(&response_bytes).map_err(|source| PluginError::InvalidPayload {
            name: name.clone(),
            capability: capability.to_string(),
            source,
        })
    }

    #[cfg(test)]
    pub(crate) fn dispatch_raw_for_test<Req: serde::Serialize, Res: serde::de::DeserializeOwned>(
        &self,
        name: &PluginName,
        call: PluginCall,
        payload: &Req,
    ) -> Result<Res, PluginError> {
        self.dispatch_raw(name, call, payload)
    }
}

/// Validates that the plugin declared the requested capability.
fn validate_capability(
    config: Option<&PluginConfig>,
    name: &PluginName,
    capability: Capability,
) -> Result<(), PluginError> {
    let has_cap = config.map(|c| c.capabilities.contains(&capability)).unwrap_or(false);

    if !has_cap {
        return Err(PluginError::UnsupportedCapability {
            name: name.clone(),
            capability: capability.to_string(),
        });
    }

    Ok(())
}
