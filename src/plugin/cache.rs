//! Fail-closed resolution of the Cutver plugin cache root.
//!
//! Resolution lives here and nowhere else (Pillar V.6). The pure resolver takes
//! an injected lookup so unit tests never touch process-global environment
//! (Pillar IV.1); the thin [`PluginCacheRoot::resolve`] constructor binds it to
//! `std::env::var` at the I/O boundary.

use std::path::{Component, Path, PathBuf};

use crate::plugin::error::PluginError;

/// Explicit override for the plugin cache directory. Used verbatim, so tests and
/// users can point the cache anywhere without inheriting the default layout.
pub const CACHE_DIR_ENV: &str = "CUTVER_PLUGIN_CACHE_DIR";

/// Default layout appended to the home directory when no override is set.
const CACHE_SUBPATH: [&str; 3] = [".cache", "cutver", "plugins"];

/// Resolved, fail-closed location of the Cutver plugin cache.
///
/// A [`PluginCacheRoot`] is produced once at the boundary and threaded through
/// the driver instead of a bare [`PathBuf`] (Pillar III.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCacheRoot {
    root: PathBuf,
}

impl PluginCacheRoot {
    /// Pure resolver.
    ///
    /// Precedence: [`CACHE_DIR_ENV`] used verbatim, then `$HOME`, then
    /// `%USERPROFILE%`. Fails closed when none is set.
    pub fn from_env_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, PluginError> {
        let root = match non_blank(lookup(CACHE_DIR_ENV)) {
            Some(override_dir) => PathBuf::from(override_dir),
            None => home_cache_root(&lookup)?,
        };

        Ok(Self { root: normalize(root) })
    }

    /// Boundary constructor binding the pure resolver to the process environment.
    pub fn resolve() -> Result<Self, PluginError> {
        Self::from_env_lookup(|key| std::env::var(key).ok())
    }

    /// Returns the resolved cache root directory.
    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Resolves an artifact path inside the cache root.
    pub fn artifact_path(&self, artifact: &str) -> PathBuf {
        self.root.join(artifact)
    }

    /// Where `remove` may delete the artifact for `source`.
    ///
    /// Reading and deleting are different questions. A `source` that is an
    /// existing local file is the user's own file, and a candidate that escapes
    /// the cache root (an absolute path, or `..` after lexical normalization) is
    /// never a cached artifact. Only a path strictly inside the root is
    /// deletable, so a destructive operation cannot escape the cache (D1).
    pub fn delete_path(&self, source: &str) -> DeletePath {
        let candidate = self.artifact_path(source);
        let root = lexical_normalize(&self.root);
        let normalized = lexical_normalize(&candidate);
        if normalized == root || !normalized.starts_with(&root) {
            return DeletePath::OutsideRoot(candidate);
        }

        let local = Path::new(source);
        if local.is_file() {
            return DeletePath::LocalFile(local.to_path_buf());
        }

        DeletePath::Deletable(candidate)
    }
}

/// Where `plugin remove` may delete an artifact (the cache root, and only it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeletePath {
    /// Inside the cache root; safe to delete.
    Deletable(PathBuf),
    /// `source` names an existing local file, which belongs to the user.
    LocalFile(PathBuf),
    /// The resolved path escapes the cache root (absolute or `..`); never delete it.
    OutsideRoot(PathBuf),
}

/// The single canonical, lazy READ resolution for a declared plugin `source`.
///
/// An existing local file wins and the cache root is never resolved in that
/// case; otherwise the artifact lives inside the resolved cache root. Used by
/// `driver::wasm::load_wasm_bytes`, `plugin list` and `plugin remove` (Pillar V.6).
pub fn read_path(
    source: &str,
    resolve_cache: impl FnOnce() -> Result<PluginCacheRoot, PluginError>,
) -> Result<PathBuf, PluginError> {
    let local = Path::new(source);
    if local.is_file() {
        return Ok(local.to_path_buf());
    }

    Ok(resolve_cache()?.artifact_path(source))
}

/// Lexically resolves `.` and `..` without touching the filesystem, so
/// containment can be checked before any deletion.
fn lexical_normalize(path: &Path) -> PathBuf {
    let mut resolved = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            other => resolved.push(other.as_os_str()),
        }
    }
    resolved
}

/// Drops absent and whitespace-only values so a blank override falls through.
fn non_blank(value: Option<String>) -> Option<String> {
    value.filter(|inner| !inner.trim().is_empty())
}

/// `<home>/.cache/cutver/plugins`, the layout used when no override is set.
fn home_cache_root(lookup: &impl Fn(&str) -> Option<String>) -> Result<PathBuf, PluginError> {
    let home = non_blank(lookup("HOME")).or_else(|| non_blank(lookup("USERPROFILE")));

    let Some(home) = home else {
        return Err(PluginError::UnresolvableCacheRoot {
            variable: CACHE_DIR_ENV.to_string(),
            fallback: "$HOME".to_string(),
        });
    };

    let mut root = PathBuf::from(home);
    for segment in CACHE_SUBPATH {
        root.push(segment);
    }

    Ok(root)
}

/// Normalizes stored separators to forward slashes (Pillar II.3).
fn normalize(path: PathBuf) -> PathBuf {
    PathBuf::from(path.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a hermetic lookup over fixed pairs, never touching the process env.
    fn lookup_from<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |key| pairs.iter().find(|(k, _)| *k == key).map(|(_, v)| (*v).to_string())
    }

    #[test]
    fn override_wins_over_home_and_userprofile() {
        let root = PluginCacheRoot::from_env_lookup(lookup_from(&[
            (CACHE_DIR_ENV, "/srv/cutver"),
            ("HOME", "/home/alice"),
            ("USERPROFILE", "C:/Users/alice"),
        ]))
        .unwrap();

        assert_eq!(root.path(), Path::new("/srv/cutver"));
    }

    #[test]
    fn override_is_used_verbatim_as_the_cache_dir() {
        let root = PluginCacheRoot::from_env_lookup(lookup_from(&[(CACHE_DIR_ENV, "/srv/cutver")])).unwrap();

        assert!(
            !root.path().to_string_lossy().contains(".cache/cutver/plugins"),
            "override must not inherit the default layout: {:?}",
            root.path()
        );
        assert_eq!(root.artifact_path("p.wasm"), Path::new("/srv/cutver/p.wasm"));
    }

    #[test]
    fn home_wins_when_override_is_absent() {
        let root = PluginCacheRoot::from_env_lookup(lookup_from(&[
            ("HOME", "/home/alice"),
            ("USERPROFILE", "C:/Users/alice"),
        ]))
        .unwrap();

        assert_eq!(root.path(), Path::new("/home/alice/.cache/cutver/plugins"));
    }

    #[test]
    fn userprofile_used_when_override_and_home_are_absent() {
        let root = PluginCacheRoot::from_env_lookup(lookup_from(&[("USERPROFILE", "C:/Users/alice")])).unwrap();

        assert_eq!(root.path(), Path::new("C:/Users/alice/.cache/cutver/plugins"));
    }

    #[test]
    fn blank_sources_fall_through_and_fail_closed() {
        let err = PluginCacheRoot::from_env_lookup(lookup_from(&[(CACHE_DIR_ENV, "   "), ("HOME", "")])).unwrap_err();

        assert!(matches!(err, PluginError::UnresolvableCacheRoot { .. }));
    }

    #[test]
    fn fails_closed_when_no_source_is_set() {
        let err = PluginCacheRoot::from_env_lookup(|_| None).unwrap_err();

        match err {
            PluginError::UnresolvableCacheRoot { variable, .. } => assert_eq!(variable, CACHE_DIR_ENV),
            other => panic!("expected UnresolvableCacheRoot, got {other:?}"),
        }
    }

    #[test]
    fn error_message_names_variable_and_home_fallback() {
        let err = PluginCacheRoot::from_env_lookup(|_| None).unwrap_err();
        let message = err.to_string();

        assert!(message.contains(CACHE_DIR_ENV), "missing variable: {message}");
        assert!(message.contains("$HOME"), "missing fallback: {message}");
        assert!(message.contains("Fix:"), "missing fix guidance: {message}");
    }

    #[test]
    fn home_layout_is_appended_on_home_branches() {
        let cases = [[("HOME", "/home/alice")], [("USERPROFILE", "/users/alice")]];

        for pairs in cases {
            let root = PluginCacheRoot::from_env_lookup(lookup_from(&pairs)).unwrap();
            assert!(
                root.path().ends_with(".cache/cutver/plugins"),
                "unexpected layout: {:?}",
                root.path()
            );
        }
    }

    #[test]
    fn stored_path_uses_forward_slashes() {
        let root = PluginCacheRoot::from_env_lookup(lookup_from(&[("HOME", r"C:\Users\alice")])).unwrap();

        assert!(!root.path().to_string_lossy().contains('\\'));
    }

    #[test]
    fn artifact_path_stays_inside_the_cache_root() {
        let root = PluginCacheRoot::from_env_lookup(lookup_from(&[("HOME", "/home/alice")])).unwrap();

        assert_eq!(
            root.artifact_path("github-releases.wasm"),
            Path::new("/home/alice/.cache/cutver/plugins/github-releases.wasm")
        );
    }

    #[test]
    fn read_path_prefers_an_existing_local_file_and_stays_lazy() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("local.wasm");
        std::fs::write(&local, b"wasm").unwrap();

        let resolved = read_path(local.to_str().unwrap(), || {
            panic!("cache root must not be resolved for an existing local file")
        })
        .unwrap();
        assert_eq!(resolved, local);
    }

    #[test]
    fn read_path_falls_back_into_the_cache_root() {
        let root = PluginCacheRoot::from_env_lookup(lookup_from(&[("HOME", "/home/alice")])).unwrap();

        let resolved = read_path("absent.wasm", || Ok(root.clone())).unwrap();
        assert_eq!(resolved, Path::new("/home/alice/.cache/cutver/plugins/absent.wasm"));
    }

    #[test]
    fn delete_path_rejects_an_absolute_source() {
        let root = PluginCacheRoot::from_env_lookup(lookup_from(&[("HOME", "/home/alice")])).unwrap();

        let decision = root.delete_path("/etc/passwd");
        assert!(
            matches!(decision, DeletePath::OutsideRoot(_)),
            "absolute source must be un-deletable: {decision:?}"
        );
    }

    #[test]
    fn delete_path_rejects_parent_traversal() {
        let root = PluginCacheRoot::from_env_lookup(lookup_from(&[("HOME", "/home/alice")])).unwrap();

        let decision = root.delete_path("../../escape.wasm");
        assert!(
            matches!(decision, DeletePath::OutsideRoot(_)),
            "traversal source must be un-deletable: {decision:?}"
        );
    }

    #[test]
    fn delete_path_allows_a_subdirectory_inside_the_root() {
        let root = PluginCacheRoot::from_env_lookup(lookup_from(&[("HOME", "/home/alice")])).unwrap();

        match root.delete_path("nested/deep/artifact.wasm") {
            DeletePath::Deletable(path) => {
                assert_eq!(
                    path,
                    Path::new("/home/alice/.cache/cutver/plugins/nested/deep/artifact.wasm")
                );
            }
            other => panic!("expected Deletable, got {other:?}"),
        }
    }

    #[test]
    fn delete_path_keeps_an_existing_local_file() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("rel.wasm");
        std::fs::write(&local, b"wasm").unwrap();
        let root = PluginCacheRoot::from_env_lookup(|key| {
            (key == CACHE_DIR_ENV).then(|| dir.path().to_string_lossy().to_string())
        })
        .unwrap();

        assert_eq!(root.delete_path(local.to_str().unwrap()), DeletePath::LocalFile(local));
    }

    #[test]
    fn without_the_newtype_the_silent_fallback_is_gone() {
        // Regression pin: the former `plugin_cache_dir()` returned `"."` when no
        // home variable was set, placing artifacts in the user's project. The
        // resolver must never produce that working-directory fallback.
        let err = PluginCacheRoot::from_env_lookup(|_| None).unwrap_err();
        let message = err.to_string();

        assert!(
            !message.contains("current directory"),
            "must not fall back to the working directory: {message}"
        );
        assert!(matches!(err, PluginError::UnresolvableCacheRoot { .. }));
    }
}
