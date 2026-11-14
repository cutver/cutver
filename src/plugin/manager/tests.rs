//! Unit and regression tests for `PluginManager` orchestration.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

use super::core::PluginManager;
use super::glob::glob_matches;
use crate::plugin::driver::PluginDriver;
use crate::plugin::dto::{
    ChangelogRenderResponse, ManifestReadRequest, PluginInvocation, PostBumpPayload, PostReleasePayload,
    PreBumpPayload, PreBumpResponse,
};
use crate::plugin::error::PluginError;
use crate::plugin::types::{
    Capability, PermissionsConfig, PluginCall, PluginConfig, PluginName, PluginOperation, RuntimeKind,
};

#[derive(Debug)]
struct MockEchoDriver {
    name: PluginName,
    expected: Mutex<VecDeque<(Capability, PluginOperation)>>,
}

impl MockEchoDriver {
    fn new(name: PluginName, expected: &[(Capability, PluginOperation)]) -> Self {
        Self {
            name,
            expected: Mutex::new(expected.iter().copied().collect()),
        }
    }
}

impl PluginDriver for MockEchoDriver {
    fn name(&self) -> &PluginName {
        &self.name
    }

    fn invoke(&self, invocation: &PluginInvocation) -> Result<Vec<u8>, PluginError> {
        let actual = (invocation.capability, invocation.operation);
        let next = self
            .expected
            .lock()
            .expect("mock driver expectation lock poisoned")
            .pop_front();
        assert_eq!(
            next,
            Some(actual),
            "unexpected envelope at call: got capability={:?} operation={:?}, expected {:?}",
            invocation.capability,
            invocation.operation,
            next
        );
        Ok(serde_json::to_vec(&invocation.payload).unwrap())
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
        config: None,
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
fn test_from_config_wasm_registers_driver() {
    let dir = tempfile::tempdir().unwrap();
    let wasm_file = dir.path().join("cutver_mgr_test.wasm");
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
        config: None,
    };

    let mut map = HashMap::new();
    map.insert(name.clone(), config);

    let manager = PluginManager::from_config(&map).unwrap();
    assert!(manager.driver(&name).is_some());
}

/// The process runtime resolves `[plugins.<name>.permissions] env` through the shared grant
/// authority, so an unset grant fails closed when the driver is built instead of the child
/// inheriting the whole parent environment (issue #264).
#[test]
fn test_from_config_process_env_grant_is_resolved() {
    const KEY: &str = "CUTVER_TEST_UNSET_FACTORY_GRANT";
    unsafe {
        std::env::set_var(KEY, "granted-value");
    }

    let name = PluginName::new("env-grant-plugin").unwrap();
    let config = PluginConfig {
        runtime: RuntimeKind::Process,
        source: None,
        hash: None,
        command: Some("env-grant-plugin".to_string()),
        capabilities: vec![Capability::ManifestV1],
        events: vec![],
        manifest_match: vec![],
        permissions: PermissionsConfig {
            env: vec![KEY.to_string()],
            ..Default::default()
        },
        timeout_seconds: None,
        config: None,
    };

    let mut map = HashMap::new();
    map.insert(name.clone(), config);
    assert!(
        PluginManager::from_config(&map).is_ok(),
        "a declared grant that is set must resolve"
    );

    unsafe {
        std::env::remove_var(KEY);
    }
    let err = PluginManager::from_config(&map).unwrap_err();

    match err {
        PluginError::UnsetEnvGrant {
            name: err_name,
            key: err_key,
        } => {
            assert_eq!(err_name, name);
            assert_eq!(err_key, KEY);
        }
        other => panic!("expected UnsetEnvGrant, got {other:?}"),
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
        config: None,
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
        config: None,
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
        config: None,
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
    let driver = MockEchoDriver::new(name.clone(), &[]);

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
        config: None,
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
    let driver = MockEchoDriver::new(
        name.clone(),
        &[
            (Capability::LifecycleV1, PluginOperation::OnPreBump),
            (Capability::ChangelogV1, PluginOperation::Render),
            (Capability::ManifestV1, PluginOperation::Read),
            (Capability::LifecycleV1, PluginOperation::OnPreBump),
            (Capability::LifecycleV1, PluginOperation::OnPostBump),
            (Capability::LifecycleV1, PluginOperation::OnPostRelease),
        ],
    );

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
        config: None,
    };

    let mut drivers: HashMap<PluginName, Box<dyn PluginDriver>> = HashMap::new();
    drivers.insert(name.clone(), Box::new(driver));
    let mut configs = HashMap::new();
    configs.insert(name.clone(), cfg);

    let manager = PluginManager::new(drivers, configs);

    let read_req = ManifestReadRequest {
        path: "Chart.yaml".to_string(),
        content: "version: 1.2.3".to_string(),
    };

    let pre_req = PreBumpResponse {
        allow: true,
        reason: Some("Passed pre-bump checks".to_string()),
    };
    let pre_res: PreBumpResponse = manager
        .dispatch_raw_for_test(&name, PluginCall::PreBump, &pre_req)
        .unwrap();
    assert_eq!(pre_res, pre_req);

    let changelog_res = ChangelogRenderResponse {
        body: "## [1.0.0] - notes".to_string(),
    };
    let cl_out: ChangelogRenderResponse = manager
        .dispatch_raw_for_test(&name, PluginCall::ChangelogRender, &changelog_res)
        .unwrap();
    assert_eq!(cl_out.body, "## [1.0.0] - notes");

    let res = manager.dispatch_manifest_read(&name, &read_req).unwrap_err();
    match res {
        PluginError::InvalidPayload {
            name: n, capability, ..
        } => {
            assert_eq!(n, name);
            assert_eq!(capability, "manifest.v1");
        }
        other => panic!("expected InvalidPayload, got {other:?}"),
    }

    let pre_bump_payload = PreBumpPayload {
        root_dir: "/workspace".to_string(),
        current_version: "1.0.0".to_string(),
        next_version: "1.1.0".to_string(),
        bump_level: "minor".to_string(),
        tag_name: "v1.1.0".to_string(),
        dry_run: false,
    };
    let pre_err = manager.dispatch_pre_bump(&name, &pre_bump_payload).unwrap_err();
    assert!(matches!(pre_err, PluginError::InvalidPayload { .. }));

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
        notes: Some(
            "## [v1.1.0] - 2026-11-12
### Features
- a thing"
                .to_string(),
        ),
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
        config: None,
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
        config: None,
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
        config: None,
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
        config: None,
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
    assert!(glob_matches("Chart.yaml", "Chart.yaml"));
    assert!(!glob_matches("Chart.yaml", "other.yaml"));
    assert!(glob_matches("*.yaml", "Chart.yaml"));
    assert!(glob_matches("*.yaml", "foo.yaml"));
    assert!(!glob_matches("*.yaml", "subdir/foo.yaml"));
    assert!(glob_matches("file?.txt", "file1.txt"));
    assert!(!glob_matches("file?.txt", "file10.txt"));
    assert!(glob_matches("**/Chart.yaml", "Chart.yaml"));
    assert!(glob_matches("**/Chart.yaml", "charts/nested/Chart.yaml"));
    assert!(glob_matches("charts/**", "charts/foo/bar.yaml"));
    assert!(glob_matches("**/*.json", "sub/dir/package.json"));
    assert!(glob_matches("**/*.json", "package.json"));
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
        config: None,
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
        config: None,
    };

    let cfg_other = PluginConfig {
        runtime: RuntimeKind::Process,
        source: None,
        hash: None,
        command: Some("other".into()),
        capabilities: vec![Capability::LifecycleV1],
        events: vec![],
        manifest_match: vec!["other.yaml".into()],
        permissions: Default::default(),
        timeout_seconds: None,
        config: None,
    };

    let mut configs = HashMap::new();
    configs.insert(p_helm.clone(), cfg_helm);
    configs.insert(p_k8s.clone(), cfg_k8s);
    configs.insert(p_other.clone(), cfg_other);

    let manager = PluginManager::new(HashMap::new(), configs);

    let res = manager.resolve_plugin_for_manifest("Chart.yaml").unwrap();
    assert_eq!(res, &p_helm);

    let res_k8s = manager.resolve_plugin_for_manifest("k8s/deploy.yaml").unwrap();
    assert_eq!(res_k8s, &p_k8s);

    let err_nomatch = manager.resolve_plugin_for_manifest("Cargo.toml").unwrap_err();
    assert!(matches!(err_nomatch, PluginError::NoPluginForManifest { ref path } if path == "Cargo.toml"));

    let err_nocap = manager.resolve_plugin_for_manifest("other.yaml").unwrap_err();
    assert!(matches!(err_nocap, PluginError::NoPluginForManifest { ref path } if path == "other.yaml"));

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

#[test]
fn test_resolve_changelog_plugin() {
    let p_cl = PluginName::new("custom-cl").unwrap();
    let p_extra = PluginName::new("extra-cl").unwrap();
    let p_other = PluginName::new("manifest-only").unwrap();

    let cfg_cl = PluginConfig {
        runtime: RuntimeKind::Process,
        source: None,
        hash: None,
        command: Some("cl".into()),
        capabilities: vec![Capability::ChangelogV1],
        events: vec![],
        manifest_match: vec![],
        permissions: Default::default(),
        timeout_seconds: None,
        config: None,
    };

    let cfg_extra = PluginConfig {
        runtime: RuntimeKind::Process,
        source: None,
        hash: None,
        command: Some("extra".into()),
        capabilities: vec![Capability::ChangelogV1],
        events: vec![],
        manifest_match: vec![],
        permissions: Default::default(),
        timeout_seconds: None,
        config: None,
    };

    let cfg_other = PluginConfig {
        runtime: RuntimeKind::Process,
        source: None,
        hash: None,
        command: Some("other".into()),
        capabilities: vec![Capability::ManifestV1],
        events: vec![],
        manifest_match: vec![],
        permissions: Default::default(),
        timeout_seconds: None,
        config: None,
    };

    let mut configs_empty = HashMap::new();
    configs_empty.insert(p_other.clone(), cfg_other);
    let mgr_empty = PluginManager::new(HashMap::new(), configs_empty);
    assert!(matches!(
        mgr_empty.resolve_changelog_plugin(),
        Err(PluginError::NoChangelogPlugin)
    ));

    let mut configs_single = HashMap::new();
    configs_single.insert(p_cl.clone(), cfg_cl.clone());
    let mgr_single = PluginManager::new(HashMap::new(), configs_single);
    let res = mgr_single.resolve_changelog_plugin().unwrap();
    assert_eq!(res, &p_cl);

    let mut configs_multi = HashMap::new();
    configs_multi.insert(p_cl.clone(), cfg_cl);
    configs_multi.insert(p_extra.clone(), cfg_extra);
    let mgr_multi = PluginManager::new(HashMap::new(), configs_multi);
    match mgr_multi.resolve_changelog_plugin().unwrap_err() {
        PluginError::AmbiguousChangelogPlugin { matches } => {
            assert_eq!(matches, vec![p_cl, p_extra]);
        }
        other => panic!("expected AmbiguousChangelogPlugin, got {other:?}"),
    }
}

#[test]
fn test_resolve_versioning_plugin() {
    let p_ver = PluginName::new("custom-semver").unwrap();
    let p_extra = PluginName::new("calver-strategy").unwrap();
    let p_other = PluginName::new("manifest-only").unwrap();

    let cfg_ver = PluginConfig {
        runtime: RuntimeKind::Process,
        source: None,
        hash: None,
        command: Some("custom-semver".into()),
        capabilities: vec![Capability::VersioningV1],
        events: vec![],
        manifest_match: vec![],
        permissions: Default::default(),
        timeout_seconds: None,
        config: None,
    };

    let cfg_extra = PluginConfig {
        runtime: RuntimeKind::Process,
        source: None,
        hash: None,
        command: Some("calver-strategy".into()),
        capabilities: vec![Capability::VersioningV1],
        events: vec![],
        manifest_match: vec![],
        permissions: Default::default(),
        timeout_seconds: None,
        config: None,
    };

    let cfg_other = PluginConfig {
        runtime: RuntimeKind::Process,
        source: None,
        hash: None,
        command: Some("manifest-only".into()),
        capabilities: vec![Capability::ManifestV1],
        events: vec![],
        manifest_match: vec![],
        permissions: Default::default(),
        timeout_seconds: None,
        config: None,
    };

    let mut configs_empty = HashMap::new();
    configs_empty.insert(p_other.clone(), cfg_other);
    let mgr_empty = PluginManager::new(HashMap::new(), configs_empty);
    assert!(matches!(
        mgr_empty.resolve_versioning_plugin(),
        Err(PluginError::NoVersioningPlugin)
    ));

    let mut configs_single = HashMap::new();
    configs_single.insert(p_ver.clone(), cfg_ver.clone());
    let mgr_single = PluginManager::new(HashMap::new(), configs_single);
    let res = mgr_single.resolve_versioning_plugin().unwrap();
    assert_eq!(res, &p_ver);

    let mut configs_multi = HashMap::new();
    configs_multi.insert(p_ver.clone(), cfg_ver);
    configs_multi.insert(p_extra.clone(), cfg_extra);
    let mgr_multi = PluginManager::new(HashMap::new(), configs_multi);
    match mgr_multi.resolve_versioning_plugin().unwrap_err() {
        PluginError::AmbiguousVersioningPlugin { matches } => {
            assert_eq!(matches, vec![p_extra, p_ver]);
        }
        other => panic!("expected AmbiguousVersioningPlugin, got {other:?}"),
    }
}
