pub mod process;
#[cfg(feature = "plugins")]
pub mod wasm;

pub use process::ProcessDriver;
#[cfg(feature = "plugins")]
pub use wasm::WasmDriver;

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::plugin::dto::PluginInvocation;
use crate::plugin::error::PluginError;
use crate::plugin::types::PluginName;

/// The environment seam every driver resolves its declared grants through.
///
/// One lookup, injected: a test drives the whole factory with a map instead of mutating the process
/// environment, whose mutators are unsafe under parallel tests (#344). [`process_env_lookup`] is the
/// only adapter bound to the real environment.
pub type EnvLookup = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// The process environment as a grant lookup: the one boundary adapter (#344).
pub fn process_env_lookup() -> EnvLookup {
    Arc::new(|key: &str| std::env::var(key).ok())
}

/// Unified driver trait for executing Cutver plugins across runtimes.
pub trait PluginDriver: std::fmt::Debug + Send + Sync {
    /// Returns the validated identifier of the plugin.
    fn name(&self) -> &PluginName;

    /// Invokes a capability on the plugin with the shared invocation envelope.
    fn invoke(&self, invocation: &PluginInvocation) -> Result<Vec<u8>, PluginError>;
}

/// The declared `[plugins.<name>.permissions] env` grants of one driver, resolved on invocation.
///
/// Construction records the keys and the lookup, and resolves nothing (#300): loading a configuration
/// must not need the secrets the plugin would receive, or the command that only inspects the
/// configuration cannot run on the host that needs it — every CI runner and most developer machines.
/// The map is built at the one moment the grant is handed over (Pillar IV.3): at spawn for the
/// process runtime, at instantiation for WASM. Fail-closed is unchanged there: a declared key the
/// host cannot produce is [`PluginError::UnsetEnvGrant`], never a silent `""` (Pillar VI.2).
#[derive(Clone)]
pub struct DeclaredEnvGrants {
    declared: Vec<String>,
    lookup: EnvLookup,
}

impl DeclaredEnvGrants {
    /// Records `declared` and binds them to the process environment.
    pub fn from_env(declared: Vec<String>) -> Self {
        Self::with_lookup(declared, process_env_lookup())
    }

    /// Records `declared` and resolves them through `lookup`.
    pub fn with_lookup(declared: Vec<String>, lookup: EnvLookup) -> Self {
        Self { declared, lookup }
    }

    /// Grants a caller has already resolved: the map is its own answer, key by key.
    ///
    /// This is how a driver is handed a literal environment — a fixture, or a caller that resolved
    /// the grants itself — without a second copy of the lookup.
    pub fn resolved(env: BTreeMap<String, String>) -> Self {
        let declared: Vec<String> = env.keys().cloned().collect();
        Self::with_lookup(declared, Arc::new(move |key: &str| env.get(key).cloned()))
    }

    /// The declared keys, in declaration order.
    pub fn declared(&self) -> &[String] {
        &self.declared
    }

    /// Whether the lookup can currently produce `key`, discarding the value.
    ///
    /// `cutver doctor` asks this, so the row it prints and the invocation that later resolves the
    /// same key cannot disagree about what the host offers (#268 S5).
    pub fn is_set(&self, key: &str) -> bool {
        (self.lookup)(key).is_some()
    }

    /// The whole declared set as the child or guest receives it, or the first key that is unset.
    ///
    /// `plugin` travels in the error: it is what tells a runner which declaration to fix (#264).
    pub fn resolve(&self, plugin: &PluginName) -> Result<BTreeMap<String, String>, PluginError> {
        env_grants_from_lookup(plugin, &self.declared, |key| (self.lookup)(key))
    }
}

/// A literal map is a set of grants the caller has already resolved.
impl From<BTreeMap<String, String>> for DeclaredEnvGrants {
    fn from(env: BTreeMap<String, String>) -> Self {
        Self::resolved(env)
    }
}

/// [`DeclaredEnvGrants`] prints its declarations, never a resolved value: a grant is a secret until
/// the plugin it was granted to reads it.
impl std::fmt::Debug for DeclaredEnvGrants {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeclaredEnvGrants")
            .field("declared", &self.declared)
            .finish_non_exhaustive()
    }
}

/// Resolves the declared `[plugins.<name>.permissions] env` grants against the host environment, now.
///
/// A grant means the same thing everywhere (Pillar VI.1): only the declared keys cross the boundary,
/// and each declared key must really exist. A declared key that is unset is an error naming the
/// plugin and the key, never a silent `""`, because an empty grant hides a broken pipeline instead
/// of failing closed (Pillar VI.2).
///
/// The drivers do not call this: they keep a [`DeclaredEnvGrants`] and resolve it at invocation
/// (#300), so nothing resolves while a configuration is only being loaded. This is the eager form,
/// for a caller that needs the map immediately — `tests/process_driver_l3_budget.rs` and the driver
/// fixtures that must hand a native shell the keys it cannot start without.
pub fn resolve_env_grants(plugin: &PluginName, declared: &[String]) -> Result<BTreeMap<String, String>, PluginError> {
    env_grants_from_lookup(plugin, declared, |key| std::env::var(key).ok())
}

/// Pure resolver over an injected `lookup`, so a test never mutates the process environment.
///
/// A key the lookup cannot produce is [`PluginError::UnsetEnvGrant`] naming the plugin and the
/// key — both an absent variable and a value that is not valid UTF-8, which the `std::env::var`
/// boundary adapter collapses into `None`. A grant is therefore never a silent empty string
/// (Pillar VI.2).
pub fn env_grants_from_lookup(
    plugin: &PluginName,
    declared: &[String],
    lookup: impl Fn(&str) -> Option<String>,
) -> Result<BTreeMap<String, String>, PluginError> {
    declared
        .iter()
        .map(|key| {
            lookup(key)
                .map(|value| (key.clone(), value))
                .ok_or_else(|| PluginError::UnsetEnvGrant {
                    name: plugin.clone(),
                    key: key.clone(),
                })
        })
        .collect()
}
