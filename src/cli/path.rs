use std::path::Path;

/// Relativizes a given path string against an optional root directory.
///
/// If `root_dir` is provided and `path` starts with it, returns the relative path.
/// Normalizes backslashes to forward slashes across all platforms.
pub fn relativize_path(path: &str, root_dir: Option<&Path>) -> String {
    let normalized_input = path.replace('\\', "/");
    let p = Path::new(&normalized_input);
    let rel_p = match root_dir {
        Some(root) => {
            let normalized_root_str = root.to_string_lossy().replace('\\', "/");
            let norm_root = Path::new(&normalized_root_str);
            p.strip_prefix(norm_root).unwrap_or(p)
        }
        None => p,
    };
    rel_p.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relativize_path_nested() {
        let root = Path::new("/workspace/project");
        assert_eq!(
            relativize_path("/workspace/project/crates/app/Cargo.toml", Some(root)),
            "crates/app/Cargo.toml"
        );
    }

    #[test]
    fn test_relativize_path_root() {
        let root = Path::new("/workspace/project");
        assert_eq!(
            relativize_path("/workspace/project/Cargo.toml", Some(root)),
            "Cargo.toml"
        );
    }

    #[test]
    fn test_relativize_path_outside() {
        let root = Path::new("/workspace/project");
        assert_eq!(
            relativize_path("/other/location/Cargo.toml", Some(root)),
            "/other/location/Cargo.toml"
        );
        assert_eq!(relativize_path("Cargo.toml", None), "Cargo.toml");
    }

    #[test]
    fn test_relativize_path_backslash_normalization() {
        let root = Path::new("C:\\repo");
        assert_eq!(
            relativize_path("C:\\repo\\src\\Cargo.toml", Some(root)),
            "src/Cargo.toml"
        );
        let root_forward = Path::new("C:/repo");
        assert_eq!(
            relativize_path("C:\\repo\\src\\Cargo.toml", Some(root_forward)),
            "src/Cargo.toml"
        );
    }
}
