use std::collections::{BTreeMap, HashMap, HashSet};

use crate::plugin::driver::{PluginDriver, ProcessDriver};
use crate::plugin::dto::{
    ChangelogRenderRequest, ChangelogRenderResponse, ManifestReadRequest, ManifestReadResponse, ManifestWriteRequest,
    ManifestWriteResponse, PostBumpPayload, PostReleasePayload, PreBumpPayload, PreBumpResponse,
};
use crate::plugin::error::PluginError;
use crate::plugin::types::{Capability, PluginConfig, PluginName, RuntimeKind};

/// Default timeout in seconds for process plugins when unspecified in config.
const DEFAULT_PLUGIN_TIMEOUT_SECS: u64 = 30;

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

    /// Dispatches a `manifest.read` request to the target plugin.
    pub fn dispatch_manifest_read(
        &self,
        name: &PluginName,
        req: &ManifestReadRequest,
    ) -> Result<ManifestReadResponse, PluginError> {
        self.dispatch_raw(name, Capability::ManifestV1, req)
    }

    /// Dispatches a `manifest.write` request to the target plugin.
    pub fn dispatch_manifest_write(
        &self,
        name: &PluginName,
        req: &ManifestWriteRequest,
    ) -> Result<ManifestWriteResponse, PluginError> {
        self.dispatch_raw(name, Capability::ManifestV1, req)
    }

    /// Dispatches a lifecycle `pre_bump` check to the target plugin.
    pub fn dispatch_pre_bump(
        &self,
        name: &PluginName,
        payload: &PreBumpPayload,
    ) -> Result<PreBumpResponse, PluginError> {
        self.dispatch_raw(name, Capability::LifecycleV1, payload)
    }

    /// Dispatches a lifecycle `post_bump` notification to the target plugin.
    pub fn dispatch_post_bump(&self, name: &PluginName, payload: &PostBumpPayload) -> Result<(), PluginError> {
        let _: serde_json::Value = self.dispatch_raw(name, Capability::LifecycleV1, payload)?;
        Ok(())
    }

    /// Dispatches a lifecycle `post_release` notification to the target plugin.
    pub fn dispatch_post_release(&self, name: &PluginName, payload: &PostReleasePayload) -> Result<(), PluginError> {
        let _: serde_json::Value = self.dispatch_raw(name, Capability::LifecycleV1, payload)?;
        Ok(())
    }

    /// Dispatches a changelog rendering request to the target plugin.
    pub fn dispatch_changelog(
        &self,
        name: &PluginName,
        req: &ChangelogRenderRequest,
    ) -> Result<ChangelogRenderResponse, PluginError> {
        self.dispatch_raw(name, Capability::ChangelogV1, req)
    }

    /// Helper dispatch method validating capability and executing IPC over driver.
    fn dispatch_raw<Req: serde::Serialize, Res: serde::de::DeserializeOwned>(
        &self,
        name: &PluginName,
        capability: Capability,
        payload: &Req,
    ) -> Result<Res, PluginError> {
        let driver = self
            .drivers
            .get(name)
            .ok_or_else(|| PluginError::PluginNotFound { name: name.clone() })?;

        validate_capability(self.configs.get(name), name, capability)?;

        let bytes = serde_json::to_vec(payload).map_err(|source| PluginError::InvalidPayload {
            name: name.clone(),
            capability: capability.to_string(),
            source,
        })?;

        let response_bytes = driver.invoke(capability.as_str(), &bytes)?;
        serde_json::from_slice(&response_bytes).map_err(|source| PluginError::InvalidPayload {
            name: name.clone(),
            capability: capability.to_string(),
            source,
        })
    }
}

/// Creates a driver instance for a plugin based on its runtime configuration.
fn create_driver(name: &PluginName, config: &PluginConfig) -> Result<Box<dyn PluginDriver>, PluginError> {
    match config.runtime {
        RuntimeKind::Wasm => Err(PluginError::WasmNotSupported { name: name.clone() }),
        RuntimeKind::Process => create_process_driver(name, config),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn echo_command() -> (String, Vec<String>) {
        if cfg!(windows) {
            (
                "cmd".to_string(),
                vec!["/C".to_string(), "findstr".to_string(), "^".to_string()],
            )
        } else {
            ("cat".to_string(), vec![])
        }
    }

    #[test]
    fn test_from_config_wasm_unsupported() {
        let name = PluginName::new("wasm-plugin").unwrap();
        let config = PluginConfig {
            runtime: RuntimeKind::Wasm,
            source: Some("plugins/wasm.wasm".to_string()),
            hash: None,
            command: None,
            capabilities: vec![Capability::ManifestV1],
            events: vec![],
            manifest_match: vec![],
            permissions: Default::default(),
            timeout_seconds: Some(10),
        };

        let mut map = HashMap::new();
        map.insert(name.clone(), config);

        let err = PluginManager::from_config(&map).unwrap_err();
        match err {
            PluginError::WasmNotSupported { name: err_name } => {
                assert_eq!(err_name, name);
            }
            other => panic!("expected WasmNotSupported, got {other:?}"),
        }
    }

    #[test]
    fn test_from_config_missing_command() {
        let name = PluginName::new("proc-plugin").unwrap();
        let config = PluginConfig {
            runtime: RuntimeKind::Process,
            source: None,
            hash: None,
            command: Some("   ".to_string()),
            capabilities: vec![Capability::ManifestV1],
            events: vec![],
            manifest_match: vec![],
            permissions: Default::default(),
            timeout_seconds: None,
        };

        let mut map = HashMap::new();
        map.insert(name.clone(), config);

        let err = PluginManager::from_config(&map).unwrap_err();
        match err {
            PluginError::MissingConfiguration { name: err_name, reason } => {
                assert_eq!(err_name, name);
                assert!(reason.contains("command"));
            }
            other => panic!("expected MissingConfiguration, got {other:?}"),
        }
    }

    #[test]
    fn test_from_config_multiple_plugins_success() {
        let p1 = PluginName::new("plugin-one").unwrap();
        let p2 = PluginName::new("plugin-two").unwrap();

        let cfg1 = PluginConfig {
            runtime: RuntimeKind::Process,
            source: None,
            hash: None,
            command: Some("cat".to_string()),
            capabilities: vec![Capability::ManifestV1],
            events: vec![],
            manifest_match: vec![],
            permissions: Default::default(),
            timeout_seconds: Some(5),
        };

        let cfg2 = PluginConfig {
            runtime: RuntimeKind::Process,
            source: None,
            hash: None,
            command: Some("cat".to_string()),
            capabilities: vec![Capability::LifecycleV1, Capability::ChangelogV1],
            events: vec![],
            manifest_match: vec![],
            permissions: Default::default(),
            timeout_seconds: None,
        };

        let mut map = HashMap::new();
        map.insert(p1.clone(), cfg1);
        map.insert(p2.clone(), cfg2);

        let manager = PluginManager::from_config(&map).unwrap();
        assert!(manager.driver(&p1).is_some());
        assert!(manager.driver(&p2).is_some());
        assert_eq!(manager.config(&p1).unwrap().timeout_seconds, Some(5));
        assert_eq!(manager.config(&p2).unwrap().timeout_seconds, None);
    }

    #[test]
    fn test_dispatch_plugin_not_found() {
        let manager = PluginManager::new(HashMap::new(), HashMap::new());
        let missing = PluginName::new("nonexistent").unwrap();
        let req = ManifestReadRequest {
            path: "Chart.yaml".to_string(),
            content: "version: 0.1.0\n".to_string(),
        };

        let err = manager.dispatch_manifest_read(&missing, &req).unwrap_err();
        match err {
            PluginError::PluginNotFound { name } => {
                assert_eq!(name, missing);
            }
            other => panic!("expected PluginNotFound, got {other:?}"),
        }
    }

    #[test]
    fn test_dispatch_missing_capability() {
        let name = PluginName::new("limited-plugin").unwrap();
        let (cmd, args) = echo_command();

        let mut caps = HashSet::new();
        caps.insert(Capability::LifecycleV1);

        let driver = ProcessDriver::new(name.clone(), cmd, args, BTreeMap::new(), 5, caps);

        let cfg = PluginConfig {
            runtime: RuntimeKind::Process,
            source: None,
            hash: None,
            command: Some("dummy".to_string()),
            capabilities: vec![Capability::LifecycleV1],
            events: vec![],
            manifest_match: vec![],
            permissions: Default::default(),
            timeout_seconds: Some(5),
        };

        let mut drivers: HashMap<PluginName, Box<dyn PluginDriver>> = HashMap::new();
        drivers.insert(name.clone(), Box::new(driver));

        let mut configs = HashMap::new();
        configs.insert(name.clone(), cfg);

        let manager = PluginManager::new(drivers, configs);

        let req = ManifestReadRequest {
            path: "Chart.yaml".to_string(),
            content: "version: 1.0.0".to_string(),
        };

        let err = manager.dispatch_manifest_read(&name, &req).unwrap_err();
        match err {
            PluginError::UnsupportedCapability { name: n, capability } => {
                assert_eq!(n, name);
                assert_eq!(capability, "manifest.v1");
            }
            other => panic!("expected UnsupportedCapability, got {other:?}"),
        }
    }

    #[test]
    fn test_dispatch_roundtrip_with_echo_driver() {
        let name = PluginName::new("echo-plugin").unwrap();
        let (cmd, args) = echo_command();

        let mut caps = HashSet::new();
        caps.insert(Capability::ManifestV1);
        caps.insert(Capability::LifecycleV1);
        caps.insert(Capability::ChangelogV1);

        let driver = ProcessDriver::new(name.clone(), cmd, args, BTreeMap::new(), 5, caps);

        let cfg = PluginConfig {
            runtime: RuntimeKind::Process,
            source: None,
            hash: None,
            command: Some("echo".to_string()),
            capabilities: vec![Capability::ManifestV1, Capability::LifecycleV1, Capability::ChangelogV1],
            events: vec![],
            manifest_match: vec![],
            permissions: Default::default(),
            timeout_seconds: Some(5),
        };

        let mut drivers: HashMap<PluginName, Box<dyn PluginDriver>> = HashMap::new();
        drivers.insert(name.clone(), Box::new(driver));
        let mut configs = HashMap::new();
        configs.insert(name.clone(), cfg);

        let manager = PluginManager::new(drivers, configs);

        // Test manifest read
        let read_req = ManifestReadRequest {
            path: "Chart.yaml".to_string(),
            content: "version: 1.2.3".to_string(),
        };
        // Using cat/echo: echo returns the exact JSON input, which matches ManifestReadResponse
        // if the JSON shape aligns. Let's test PreBumpResponse where echo returns allow: true
        let pre_req = PreBumpResponse {
            allow: true,
            reason: Some("Passed pre-bump checks".to_string()),
        };
        // Raw dispatch directly testing roundtrip deserialization
        let pre_res: PreBumpResponse = manager.dispatch_raw(&name, Capability::LifecycleV1, &pre_req).unwrap();
        assert_eq!(pre_res, pre_req);

        // Test changelog render roundtrip
        let changelog_res = ChangelogRenderResponse {
            body: "## [1.0.0] - notes".to_string(),
        };
        let cl_out: ChangelogRenderResponse = manager
            .dispatch_raw(&name, Capability::ChangelogV1, &changelog_res)
            .unwrap();
        assert_eq!(cl_out.body, "## [1.0.0] - notes");

        let res = manager.dispatch_manifest_read(&name, &read_req).unwrap_err();
        // echo returns ManifestReadRequest JSON, which fails to deserialize into ManifestReadResponse ("missing field `version`")
        match res {
            PluginError::InvalidPayload {
                name: n, capability, ..
            } => {
                assert_eq!(n, name);
                assert_eq!(capability, "manifest.v1");
            }
            other => panic!("expected InvalidPayload, got {other:?}"),
        }

        // Test dispatch_pre_bump success with echo driver
        // If driver returns valid JSON for PreBumpResponse:
        let pre_bump_payload = PreBumpPayload {
            root_dir: "/workspace".to_string(),
            current_version: "1.0.0".to_string(),
            next_version: "1.1.0".to_string(),
            bump_level: "minor".to_string(),
            tag_name: "v1.1.0".to_string(),
            dry_run: false,
        };
        // PreBumpPayload sent to echo won't deserialize into PreBumpResponse (missing `allow`)
        let pre_err = manager.dispatch_pre_bump(&name, &pre_bump_payload).unwrap_err();
        assert!(matches!(pre_err, PluginError::InvalidPayload { .. }));

        // Test dispatch_post_bump and dispatch_post_release with echo driver
        let post_bump_payload = PostBumpPayload {
            root_dir: "/workspace".to_string(),
            current_version: "1.0.0".to_string(),
            next_version: "1.1.0".to_string(),
            tag_name: "v1.1.0".to_string(),
            modified_files: vec![],
            dry_run: false,
        };
        assert!(manager.dispatch_post_bump(&name, &post_bump_payload).is_ok());

        let post_rel_payload = PostReleasePayload {
            root_dir: "/workspace".to_string(),
            version: "1.1.0".to_string(),
            tag_name: "v1.1.0".to_string(),
            commit_sha: "abcdef".to_string(),
            dry_run: false,
        };
        assert!(manager.dispatch_post_release(&name, &post_rel_payload).is_ok());
    }
}
