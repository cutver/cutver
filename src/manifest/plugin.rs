use std::sync::Arc;

use semver::Version;

use crate::manifest::{Error, ManifestEditor};
use crate::plugin::dto::{ManifestReadRequest, ManifestWriteRequest};
use crate::plugin::manager::PluginManager;
use crate::plugin::types::PluginName;

/// Manifest editor implementation backed by a plugin implementing capability `manifest.v1`.
#[derive(Debug)]
pub struct PluginManifestEditor {
    plugin_name: PluginName,
    path: String,
    manager: Arc<PluginManager>,
}

impl PluginManifestEditor {
    pub fn new(plugin_name: PluginName, path: String, manager: Arc<PluginManager>) -> Self {
        Self {
            plugin_name,
            path,
            manager,
        }
    }
}

impl ManifestEditor for PluginManifestEditor {
    fn read_version(&self, content: &str) -> Result<Version, Error> {
        let req = ManifestReadRequest {
            path: self.path.clone(),
            content: content.to_string(),
        };
        let res = self
            .manager
            .dispatch_manifest_read(&self.plugin_name, &req)
            .map_err(Error::Plugin)?;

        Version::parse(&res.version).map_err(|e| Error::InvalidVersion(res.version, e))
    }

    fn write_version(&self, content: &str, version: &Version) -> Result<String, Error> {
        let current_version = self.read_version(content)?.to_string();
        let req = ManifestWriteRequest {
            path: self.path.clone(),
            content: content.to_string(),
            current_version,
            next_version: version.to_string(),
        };
        let res = self
            .manager
            .dispatch_manifest_write(&self.plugin_name, &req)
            .map_err(Error::Plugin)?;

        Ok(res.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::driver::PluginDriver;
    use crate::plugin::dto::{ManifestReadResponse, ManifestWriteResponse, PluginInvocation};
    use crate::plugin::types::{Capability, PluginConfig, PluginOperation, RuntimeKind};
    use std::collections::HashMap;

    #[derive(Debug)]
    struct MockManifestDriver {
        name: PluginName,
    }

    impl PluginDriver for MockManifestDriver {
        fn name(&self) -> &PluginName {
            &self.name
        }

        fn invoke(&self, invocation: &PluginInvocation) -> Result<Vec<u8>, crate::plugin::PluginError> {
            assert_eq!(invocation.capability, Capability::ManifestV1);
            let payload = serde_json::to_vec(&invocation.payload).unwrap();
            if let Ok(req) = serde_json::from_slice::<ManifestWriteRequest>(&payload) {
                assert_eq!(invocation.operation, PluginOperation::Write);
                let replaced = req.content.replace(
                    &format!("version = {}", req.current_version),
                    &format!("version = {}", req.next_version),
                );
                let resp = ManifestWriteResponse { content: replaced };
                return Ok(serde_json::to_vec(&resp).unwrap());
            }

            if let Ok(req) = serde_json::from_slice::<ManifestReadRequest>(&payload) {
                assert_eq!(invocation.operation, PluginOperation::Read);
                // Mock parse logic: e.g. line starts with "version = "
                let ver = req
                    .content
                    .lines()
                    .find_map(|l| l.strip_prefix("version = "))
                    .unwrap_or("0.0.0");
                let resp = ManifestReadResponse {
                    version: ver.to_string(),
                };
                return Ok(serde_json::to_vec(&resp).unwrap());
            }

            panic!("unknown payload");
        }
    }

    #[test]
    fn test_plugin_manifest_editor_read_and_write() {
        let p_name = PluginName::new("mock-manifest").unwrap();
        let mut drivers: HashMap<PluginName, Box<dyn PluginDriver>> = HashMap::new();
        drivers.insert(p_name.clone(), Box::new(MockManifestDriver { name: p_name.clone() }));

        let mut configs = HashMap::new();
        configs.insert(
            p_name.clone(),
            PluginConfig {
                runtime: RuntimeKind::Process,
                source: None,
                hash: None,
                command: Some("mock".into()),
                capabilities: vec![Capability::ManifestV1],
                events: vec![],
                manifest_match: vec!["custom.manifest".into()],
                permissions: Default::default(),
                timeout_seconds: None,
            },
        );

        let manager = Arc::new(PluginManager::new(drivers, configs));
        let editor = PluginManifestEditor::new(p_name, "custom.manifest".into(), manager);

        let content = "name = foo\nversion = 1.2.3\n";
        let v = editor.read_version(content).unwrap();
        assert_eq!(v, Version::parse("1.2.3").unwrap());

        let next = Version::parse("1.2.4").unwrap();
        let new_content = editor.write_version(content, &next).unwrap();
        assert_eq!(new_content, "name = foo\nversion = 1.2.4\n");
    }
}
