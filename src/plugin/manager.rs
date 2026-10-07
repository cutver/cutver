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

/// Matches a glob pattern against a normalized path using regex.
///
/// Supported tokens:
/// - `**`: matches arbitrary directories across `/`
/// - `*`: matches any characters except `/`
/// - `?`: matches any single character except `/`
/// - Literal text: matches exactly
pub(crate) fn glob_matches(pattern: &str, path: &str) -> bool {
    // Normalize backslashes to forward slashes for cross-platform glob matching
    let norm_path = path.replace('\\', "/");
    let norm_pattern = pattern.replace('\\', "/");

    let regex_pattern = glob_to_regex(&norm_pattern);
    match regex::Regex::new(&regex_pattern) {
        Ok(re) => re.is_match(&norm_path),
        Err(_) => false,
    }
}

fn glob_to_regex(pattern: &str) -> String {
    let mut regex = String::from("^");
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' => {
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    // Match '**'
                    i += 2;
                    if i < chars.len() && chars[i] == '/' {
                        // "**/": matches zero or more directories
                        regex.push_str("(?:.*/)?");
                        i += 1;
                    } else {
                        // "**": matches anything
                        regex.push_str(".*");
                    }
                } else {
                    // Single '*': matches non-slash characters
                    regex.push_str("[^/]*");
                    i += 1;
                }
            }
            '?' => {
                regex.push_str("[^/]");
                i += 1;
            }
            c => {
                regex.push_str(&regex::escape(&c.to_string()));
                i += 1;
            }
        }
    }
    regex.push('$');
    regex
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Debug)]
    struct MockEchoDriver {
        name: PluginName,
    }

    impl PluginDriver for MockEchoDriver {
        fn name(&self) -> &PluginName {
            &self.name
        }

        fn invoke(&self, _capability: &str, payload: &[u8]) -> Result<Vec<u8>, PluginError> {
            Ok(payload.to_vec())
        }
    }

    #[test]
    #[cfg(not(feature = "plugins"))]
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
    #[cfg(feature = "plugins")]
    fn test_from_config_wasm_supported_and_executes() {
        let temp_dir = std::env::temp_dir();
        let wasm_file = temp_dir.join("cutver_mgr_test.wasm");
        let minimal_wasm: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        std::fs::write(&wasm_file, minimal_wasm).unwrap();

        let name = PluginName::new("wasm-plugin-feature").unwrap();
        let config = PluginConfig {
            runtime: RuntimeKind::Wasm,
            source: Some(wasm_file.to_str().unwrap().to_string()),
            hash: None,
            command: None,
            capabilities: vec![Capability::ManifestV1],
            events: vec![],
            manifest_match: vec![],
            permissions: Default::default(),
            timeout_seconds: Some(5),
        };

        let mut map = HashMap::new();
        map.insert(name.clone(), config);

        let manager = PluginManager::from_config(&map).unwrap();
        assert!(manager.driver(&name).is_some());
        let _ = std::fs::remove_file(wasm_file);
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
        let driver = MockEchoDriver { name: name.clone() };

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
        let driver = MockEchoDriver { name: name.clone() };

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

    #[test]
    fn test_plugins_for_event() {
        let p_all = PluginName::new("all-events").unwrap();
        let p_pre = PluginName::new("pre-bump-only").unwrap();
        let p_post = PluginName::new("post-bump-only").unwrap();
        let p_non_lc = PluginName::new("non-lifecycle").unwrap();

        let cfg_all = PluginConfig {
            runtime: RuntimeKind::Process,
            source: None,
            hash: None,
            command: Some("cat".to_string()),
            capabilities: vec![Capability::LifecycleV1],
            events: vec![],
            manifest_match: vec![],
            permissions: Default::default(),
            timeout_seconds: None,
        };

        let cfg_pre = PluginConfig {
            runtime: RuntimeKind::Process,
            source: None,
            hash: None,
            command: Some("cat".to_string()),
            capabilities: vec![Capability::LifecycleV1],
            events: vec!["on_pre_bump".to_string()],
            manifest_match: vec![],
            permissions: Default::default(),
            timeout_seconds: None,
        };

        let cfg_post = PluginConfig {
            runtime: RuntimeKind::Process,
            source: None,
            hash: None,
            command: Some("cat".to_string()),
            capabilities: vec![Capability::LifecycleV1],
            events: vec!["on_post_bump".to_string()],
            manifest_match: vec![],
            permissions: Default::default(),
            timeout_seconds: None,
        };

        let cfg_non_lc = PluginConfig {
            runtime: RuntimeKind::Process,
            source: None,
            hash: None,
            command: Some("cat".to_string()),
            capabilities: vec![Capability::ManifestV1],
            events: vec!["on_pre_bump".to_string()],
            manifest_match: vec![],
            permissions: Default::default(),
            timeout_seconds: None,
        };

        let mut configs = HashMap::new();
        configs.insert(p_all.clone(), cfg_all);
        configs.insert(p_pre.clone(), cfg_pre);
        configs.insert(p_post.clone(), cfg_post);
        configs.insert(p_non_lc.clone(), cfg_non_lc);

        let manager = PluginManager::new(HashMap::new(), configs);

        let pre_plugins: HashSet<&PluginName> = manager.plugins_for_event("on_pre_bump").collect();
        assert_eq!(pre_plugins.len(), 2);
        assert!(pre_plugins.contains(&p_all));
        assert!(pre_plugins.contains(&p_pre));
        assert!(!pre_plugins.contains(&p_post));
        assert!(!pre_plugins.contains(&p_non_lc));

        let post_plugins: HashSet<&PluginName> = manager.plugins_for_event("on_post_bump").collect();
        assert_eq!(post_plugins.len(), 2);
        assert!(post_plugins.contains(&p_all));
        assert!(post_plugins.contains(&p_post));
        assert!(!post_plugins.contains(&p_pre));

        let rel_plugins: HashSet<&PluginName> = manager.plugins_for_event("on_post_release").collect();
        assert_eq!(rel_plugins.len(), 1);
        assert!(rel_plugins.contains(&p_all));
    }

    #[test]
    fn test_glob_matches() {
        // Exact
        assert!(glob_matches("Chart.yaml", "Chart.yaml"));
        assert!(!glob_matches("Chart.yaml", "other.yaml"));

        // Single asterisk
        assert!(glob_matches("*.yaml", "Chart.yaml"));
        assert!(glob_matches("*.yaml", "foo.yaml"));
        assert!(!glob_matches("*.yaml", "subdir/foo.yaml"));

        // Question mark
        assert!(glob_matches("file?.txt", "file1.txt"));
        assert!(!glob_matches("file?.txt", "file10.txt"));

        // Double asterisk
        assert!(glob_matches("**/Chart.yaml", "Chart.yaml"));
        assert!(glob_matches("**/Chart.yaml", "charts/nested/Chart.yaml"));
        assert!(glob_matches("charts/**", "charts/foo/bar.yaml"));
        assert!(glob_matches("**/*.json", "sub/dir/package.json"));
        assert!(glob_matches("**/*.json", "package.json"));

        // Backslash normalization
        assert!(glob_matches("charts/**/*.yaml", "charts\\sub\\Chart.yaml"));
    }

    #[test]
    fn test_resolve_plugin_for_manifest() {
        let p_helm = PluginName::new("helm").unwrap();
        let p_k8s = PluginName::new("k8s").unwrap();
        let p_other = PluginName::new("other").unwrap();

        let cfg_helm = PluginConfig {
            runtime: RuntimeKind::Process,
            source: None,
            hash: None,
            command: Some("helm".into()),
            capabilities: vec![Capability::ManifestV1],
            events: vec![],
            manifest_match: vec!["Chart.yaml".into(), "charts/**/Chart.yaml".into()],
            permissions: Default::default(),
            timeout_seconds: None,
        };

        let cfg_k8s = PluginConfig {
            runtime: RuntimeKind::Process,
            source: None,
            hash: None,
            command: Some("k8s".into()),
            capabilities: vec![Capability::ManifestV1],
            events: vec![],
            manifest_match: vec!["k8s/*.yaml".into(), "charts/**/Chart.yaml".into()],
            permissions: Default::default(),
            timeout_seconds: None,
        };

        let cfg_other = PluginConfig {
            runtime: RuntimeKind::Process,
            source: None,
            hash: None,
            command: Some("other".into()),
            capabilities: vec![Capability::LifecycleV1], // Note: ManifestV1 NOT declared
            events: vec![],
            manifest_match: vec!["other.yaml".into()],
            permissions: Default::default(),
            timeout_seconds: None,
        };

        let mut configs = HashMap::new();
        configs.insert(p_helm.clone(), cfg_helm);
        configs.insert(p_k8s.clone(), cfg_k8s);
        configs.insert(p_other.clone(), cfg_other);

        let manager = PluginManager::new(HashMap::new(), configs);

        // 1 match -> Ok
        let res = manager.resolve_plugin_for_manifest("Chart.yaml").unwrap();
        assert_eq!(res, &p_helm);

        let res_k8s = manager.resolve_plugin_for_manifest("k8s/deploy.yaml").unwrap();
        assert_eq!(res_k8s, &p_k8s);

        // 0 matches (either no pattern match or capability missing) -> Err(NoPluginForManifest)
        let err_nomatch = manager.resolve_plugin_for_manifest("Cargo.toml").unwrap_err();
        assert!(matches!(err_nomatch, PluginError::NoPluginForManifest { ref path } if path == "Cargo.toml"));

        let err_nocap = manager.resolve_plugin_for_manifest("other.yaml").unwrap_err();
        assert!(matches!(err_nocap, PluginError::NoPluginForManifest { ref path } if path == "other.yaml"));

        // >1 matches -> Err(AmbiguousManifestPlugin)
        let err_ambig = manager
            .resolve_plugin_for_manifest("charts/foo/Chart.yaml")
            .unwrap_err();
        match err_ambig {
            PluginError::AmbiguousManifestPlugin { path, matches } => {
                assert_eq!(path, "charts/foo/Chart.yaml");
                assert_eq!(matches, vec![p_helm.clone(), p_k8s.clone()]);
            }
            other => panic!("expected AmbiguousManifestPlugin, got {other:?}"),
        }
    }
}
