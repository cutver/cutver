use serde::Deserialize;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

pub use super::path::ManifestPath;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Read(#[from] io::Error),
    #[error("failed to parse config file: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to parse config document: {0}")]
    DocumentParse(#[from] toml_edit::TomlError),
    #[error("manifest path must not be empty")]
    EmptyManifestPath,
    #[error("duplicate manifest path: {0}")]
    DuplicateManifestPath(String),
    #[error("current_source '{0}' is not declared as a manifest path")]
    CurrentSourceNotFound(String),
    #[error("preflight step '{0}' must be a string or inline table")]
    PreflightNotString(String),
    #[error("preflight step '{0}' is missing a string 'command' field")]
    PreflightMissingCommand(String),
    #[error("preflight timeout for '{0}' must be a positive integer")]
    PreflightInvalidTimeout(String),
    #[error("no cutver.toml found in '{0}' or any parent directory.\n  Get started by running:\n    cutver init")]
    NotFound(String),
    #[error("no manifests declared: at least one [[manifest]] entry is required")]
    NoManifestsDeclared,
    #[error("multiple manifests are marked with primary = true: only one manifest may be primary")]
    MultiplePrimaryManifests,
    #[error("conflicting primary manifest: [version] specifies '{0}' but '{1}' is marked as primary")]
    ConflictingPrimaryManifest(String, String),
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub root_dir: PathBuf,
    #[serde(default)]
    pub project: Project,
    #[serde(default)]
    pub version: VersionSection,
    #[serde(default)]
    pub manifest: Vec<Manifest>,
    #[serde(skip)]
    pub preflight: PreflightSteps,
    #[serde(skip)]
    pub preflight_default_timeout: Option<u64>,
    #[serde(default)]
    pub changelog: Changelog,
    #[serde(default)]
    pub git: Git,
    #[serde(default)]
    pub hooks: Hooks,
    #[serde(default)]
    pub publish: Publish,
    #[serde(default)]
    pub plugins: HashMap<crate::plugin::PluginName, crate::plugin::PluginConfig>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct Project {
    pub name: Option<String>,
}

impl Config {
    /// Returns the primary manifest according to `version.current_source`,
    /// falling back to the first configured manifest.
    pub fn primary_manifest(&self) -> Option<&Manifest> {
        self.manifest
            .iter()
            .find(|m| m.path == self.version.current_source)
            .or_else(|| self.manifest.first())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root_dir: PathBuf::new(),
            project: Project::default(),
            version: VersionSection::default(),
            manifest: Vec::new(),
            preflight: Vec::new(),
            preflight_default_timeout: None,
            changelog: Changelog::default(),
            git: Git::default(),
            hooks: Hooks::default(),
            publish: Publish::default(),
            plugins: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct Hooks {
    pub pre_bump: Option<String>,
    pub post_bump: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct Publish {
    #[serde(default)]
    pub push: bool,
    #[serde(default)]
    pub commands: Vec<String>,
    pub default_timeout: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct VersionSection {
    #[serde(alias = "source", default)]
    pub current_source: String,
    #[serde(default = "default_strategy")]
    pub strategy: String,
}

pub(crate) fn default_strategy() -> String {
    "manual".into()
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ManifestKind {
    Json {
        field: String,
    },
    #[serde(alias = "toml")]
    CargoPackage,
    Gradle {
        version_name_field: String,
        version_code_field: String,
    },
    Regex {
        pattern: String,
        replacement: String,
    },
    #[serde(alias = "pyproject-toml")]
    Pyproject {
        #[serde(default)]
        table: Option<String>,
    },
    Plugin {
        #[serde(default)]
        plugin: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub path: ManifestPath,
    #[serde(default)]
    pub primary: bool,
    #[serde(flatten)]
    pub kind: ManifestKind,
}

#[derive(Debug, Clone)]
pub struct PreflightCommand {
    pub command: String,
    pub timeout: Option<u64>,
}

/// Ordered `[preflight]` steps as declared in `cutver.toml`.
pub type PreflightSteps = Vec<(String, PreflightCommand)>;

/// Global `[preflight] default_timeout`.
pub type PreflightDefault = Option<u64>;

/// Alias for `Changelog` configuration section.
pub type ChangelogConfig = Changelog;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Changelog {
    #[serde(default = "default_changelog_path")]
    pub path: Option<String>,
    #[serde(default = "default_changelog_format")]
    pub format: String,
    #[serde(default = "default_changelog_mode")]
    pub mode: String,
    #[serde(default)]
    pub entry_template: String,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub template_file: Option<String>,
    #[serde(default)]
    pub full_template: bool,
    #[serde(default)]
    pub header_template: Option<String>,
    #[serde(default = "default_true")]
    pub include_scopes: bool,
    #[serde(default = "default_fallback_entry")]
    pub fallback_entry: String,
    #[serde(default = "default_true")]
    pub ignore_release_commits: bool,
    #[serde(default)]
    pub ignore_scopes: Vec<String>,
    #[serde(default)]
    pub plugin: Option<String>,
}

impl Default for Changelog {
    fn default() -> Self {
        Self {
            path: default_changelog_path(),
            format: default_changelog_format(),
            mode: default_changelog_mode(),
            entry_template: String::new(),
            template: None,
            template_file: None,
            full_template: false,
            header_template: None,
            include_scopes: default_true(),
            fallback_entry: default_fallback_entry(),
            ignore_release_commits: default_true(),
            ignore_scopes: Vec::new(),
            plugin: None,
        }
    }
}

fn default_changelog_path() -> Option<String> {
    None
}

fn default_changelog_format() -> String {
    "keep-a-changelog".into()
}

fn default_changelog_mode() -> String {
    "conventional".into()
}

fn default_true() -> bool {
    true
}

fn default_fallback_entry() -> String {
    "Maintenance and updates.".into()
}

#[derive(Debug, Deserialize)]
pub struct Git {
    #[serde(default = "default_tag_prefix")]
    pub tag_prefix: String,
    #[serde(default = "default_commit_message")]
    pub commit_message: String,
    #[serde(default = "default_require_clean_tree")]
    pub require_clean_tree: bool,
    pub require_branch: Option<String>,
    #[serde(default)]
    pub floating_major_tag: bool,
}

impl Default for Git {
    fn default() -> Self {
        Self {
            tag_prefix: default_tag_prefix(),
            commit_message: default_commit_message(),
            require_clean_tree: default_require_clean_tree(),
            require_branch: None,
            floating_major_tag: false,
        }
    }
}

fn default_tag_prefix() -> String {
    "v".into()
}

fn default_commit_message() -> String {
    "chore(release): v{version}".into()
}

fn default_require_clean_tree() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(kind: &str, extra: &str) -> String {
        format!("[version]\ncurrent_source = \"a\"\n\n[[manifest]]\npath = \"a\"\nkind = \"{kind}\"\n{extra}")
    }

    fn load_str(s: &str) -> Result<Config, ConfigError> {
        let mut c: Config = toml::from_str(s).map_err(ConfigError::Parse)?;
        let (p, d) = super::super::preflight::parse_preflight(s)?;
        c.preflight = p;
        c.preflight_default_timeout = d;
        c.root_dir = std::env::current_dir().unwrap();
        super::super::validation::deduce_current_source(&mut c)?;
        super::super::validation::validate(&c)?;
        Ok(c)
    }

    #[test]
    fn changelog_defaults_and_custom_config() {
        let default_cl = Changelog::default();
        assert_eq!(default_cl.format, "keep-a-changelog");
        assert_eq!(default_cl.entry_template, "");
        assert_eq!(default_cl.mode, "conventional");
        assert!(default_cl.include_scopes);
        assert_eq!(default_cl.fallback_entry, "Maintenance and updates.");
        assert_eq!(default_cl.template, None);
        assert_eq!(default_cl.template_file, None);
        assert!(default_cl.ignore_release_commits);
        assert!(default_cl.ignore_scopes.is_empty());

        let toml = r####"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
path = "CHANGELOG.md"
mode = "template"
template = "### Release {{ version }}"
template_file = "templates/release.j2"
include_scopes = false
fallback_entry = "Custom fallback notes."
ignore_release_commits = false
ignore_scopes = ["internal", "wip"]
"####;
        let c = load_str(toml).unwrap();
        assert_eq!(c.changelog.mode, "template");
        assert_eq!(c.changelog.template.as_deref(), Some("### Release {{ version }}"));
        assert_eq!(c.changelog.template_file.as_deref(), Some("templates/release.j2"));
        assert!(!c.changelog.include_scopes);
        assert_eq!(c.changelog.fallback_entry, "Custom fallback notes.");
        assert!(!c.changelog.ignore_release_commits);
        assert_eq!(c.changelog.ignore_scopes, vec!["internal", "wip"]);
        assert_eq!(c.changelog.plugin, None);
    }

    #[test]
    fn changelog_plugin_format_and_plugin_field() {
        let toml = r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
format = "plugin"
plugin = "custom-changelog"
"#;
        let c = load_str(toml).unwrap();
        assert_eq!(c.changelog.format, "plugin");
        assert_eq!(c.changelog.plugin.as_deref(), Some("custom-changelog"));

        let toml_auto = r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[changelog]
format = "plugin"
"#;
        let c_auto = load_str(toml_auto).unwrap();
        assert_eq!(c_auto.changelog.format, "plugin");
        assert_eq!(c_auto.changelog.plugin, None);
    }

    #[test]
    fn hooks_and_publish_parsing_and_defaults() {
        let toml = r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[hooks]
pre_bump = "echo pre"
post_bump = "cargo check"

[publish]
push = true
commands = ["cargo publish", "gh release create v{version}"]
default_timeout = 60
"#;
        let c = load_str(toml).unwrap();
        assert_eq!(c.hooks.pre_bump.as_deref(), Some("echo pre"));
        assert_eq!(c.hooks.post_bump.as_deref(), Some("cargo check"));
        assert!(c.publish.push);
        assert_eq!(
            c.publish.commands,
            vec!["cargo publish", "gh release create v{version}"]
        );
        assert_eq!(c.publish.default_timeout, Some(60));

        let default_cfg = load_str(&manifest("cargo-package", "")).unwrap();
        assert_eq!(default_cfg.hooks.pre_bump, None);
        assert_eq!(default_cfg.hooks.post_bump, None);
        assert!(!default_cfg.publish.push);
        assert!(default_cfg.publish.commands.is_empty());
        assert_eq!(default_cfg.publish.default_timeout, None);
    }

    #[test]
    fn pyproject_manifest_config_parsing() {
        let cfg = load_str(&manifest("pyproject", "")).unwrap();
        assert_eq!(cfg.manifest[0].kind, ManifestKind::Pyproject { table: None });
    }

    #[test]
    fn pyproject_manifest_with_table() {
        let cfg = load_str(&manifest("pyproject", "table = \"tool.poetry\"\n")).unwrap();
        assert_eq!(
            cfg.manifest[0].kind,
            ManifestKind::Pyproject {
                table: Some("tool.poetry".to_string()),
            }
        );
    }

    #[test]
    fn pyproject_manifest_alias() {
        let cfg = load_str(&manifest("pyproject-toml", "")).unwrap();
        assert_eq!(cfg.manifest[0].kind, ManifestKind::Pyproject { table: None });
    }

    #[test]
    fn plugin_manifest_config_parsing() {
        let cfg = load_str(&manifest("plugin", "")).unwrap();
        assert_eq!(cfg.manifest[0].kind, ManifestKind::Plugin { plugin: None });

        let cfg_with_plugin = load_str(&manifest("plugin", "plugin = \"custom-adapter\"\n")).unwrap();
        assert_eq!(
            cfg_with_plugin.manifest[0].kind,
            ManifestKind::Plugin {
                plugin: Some("custom-adapter".to_string()),
            }
        );
    }

    #[test]
    fn git_floating_major_tag_parsing() {
        let toml = r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[git]
floating_major_tag = true
"#;
        let c = load_str(toml).unwrap();
        assert!(c.git.floating_major_tag);

        let default_cfg = load_str(&manifest("cargo-package", "")).unwrap();
        assert!(!default_cfg.git.floating_major_tag);
    }

    #[test]
    fn project_config_parsing_and_defaults() {
        let toml = r#"
[project]
name = "my-tool"

[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"
"#;
        let c = load_str(toml).unwrap();
        assert_eq!(c.project.name.as_deref(), Some("my-tool"));

        let default_cfg = load_str(&manifest("cargo-package", "")).unwrap();
        assert_eq!(default_cfg.project.name, None);
    }

    #[test]
    fn primary_manifest_resolution() {
        let mut cfg = Config::default();
        assert!(cfg.primary_manifest().is_none());

        cfg.manifest.push(Manifest {
            path: ManifestPath::parse("Cargo.toml").unwrap(),
            primary: false,
            kind: ManifestKind::CargoPackage,
        });
        cfg.manifest.push(Manifest {
            path: ManifestPath::parse("package.json").unwrap(),
            primary: false,
            kind: ManifestKind::Json {
                field: "version".into(),
            },
        });

        // When current_source is empty, falls back to first manifest
        assert_eq!(cfg.primary_manifest().map(|m| m.path.as_str()), Some("Cargo.toml"));

        // When current_source matches a manifest, finds that manifest
        cfg.version.current_source = "package.json".into();
        assert_eq!(cfg.primary_manifest().map(|m| m.path.as_str()), Some("package.json"));

        // When current_source does not match any manifest, falls back to first manifest
        cfg.version.current_source = "nonexistent.json".into();
        assert_eq!(cfg.primary_manifest().map(|m| m.path.as_str()), Some("Cargo.toml"));
    }

    #[test]
    fn plugins_table_parsing_and_defaults() {
        let toml = r#"
[[manifest]]
path = "Cargo.toml"
kind = "cargo-package"

[plugins.helm-adapter]
runtime = "process"
command = "helm-plugin"
capabilities = ["manifest.v1", "lifecycle.v1"]
timeout_seconds = 15
"#;
        let c = load_str(toml).unwrap();
        assert_eq!(c.plugins.len(), 1);
        let helm_name = crate::plugin::PluginName::new("helm-adapter").unwrap();
        let plugin_cfg = c.plugins.get(&helm_name).expect("plugin found");
        assert_eq!(plugin_cfg.runtime, crate::plugin::RuntimeKind::Process);
        assert_eq!(plugin_cfg.command.as_deref(), Some("helm-plugin"));
        assert_eq!(plugin_cfg.timeout_seconds, Some(15));
        assert_eq!(
            plugin_cfg.capabilities,
            vec![
                crate::plugin::Capability::ManifestV1,
                crate::plugin::Capability::LifecycleV1
            ]
        );

        let default_cfg = load_str(&manifest("cargo-package", "")).unwrap();
        assert!(default_cfg.plugins.is_empty());
    }
}
