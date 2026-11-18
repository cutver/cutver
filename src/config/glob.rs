//! The one glob authority shared by `publish`, `plan` and the plugin manager (#326 S3).
//!
//! Matching and the directory walk live here, outside the `plugins` feature, so the lean build ships
//! them too. The plugin manager's `manifest_match` keeps using [`glob_matches`]; `[[artifact]]`
//! verification and `cutver plan` use [`expand`], which turns a workspace-relative pattern into the
//! files that exist right now.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use thiserror::Error;

/// Why a declared glob could not be expanded.
#[derive(Debug, Error)]
pub enum GlobError {
    #[error("declared artifact glob '{pattern}' escapes the workspace root '{root}'")]
    OutsideWorkspace { pattern: String, root: String },
    #[error("failed to read '{path}' while expanding the declared artifact glob '{pattern}': {source}")]
    Read {
        pattern: String,
        path: String,
        source: io::Error,
    },
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

/// Expands a workspace-relative pattern into the files that exist under `root` right now.
///
/// The walk never leaves the workspace: a pattern that is absolute or carries a `..` component is an
/// error instead of an empty match, and a symlink is never followed, because a link inside the root
/// can name any directory on the machine. `.git` is skipped everywhere it appears. The result is
/// sorted so the count a caller reports is stable.
pub(crate) fn expand(root: &Path, pattern: &str) -> Result<Vec<PathBuf>, GlobError> {
    let pattern = normalize_pattern(pattern);
    reject_escape(root, &pattern)?;
    let start = root.join(literal_prefix(&pattern));
    let mut found = Vec::new();
    walk(root, &start, &pattern, &mut found)?;
    found.sort();
    Ok(found)
}

/// The declared spelling, with separators unified and a leading `./` removed, so the same glob
/// behaves the same on every platform without changing what it means.
fn normalize_pattern(pattern: &str) -> String {
    let mut normalized = pattern.replace('\\', "/");
    while let Some(rest) = normalized.strip_prefix("./") {
        normalized = rest.to_string();
    }
    normalized
}

/// Refuses a pattern that would leave the workspace before any file is read.
fn reject_escape(root: &Path, pattern: &str) -> Result<(), GlobError> {
    let path = Path::new(pattern);
    let escapes = pattern.starts_with('/')
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        });
    if escapes {
        return Err(GlobError::OutsideWorkspace {
            pattern: pattern.to_string(),
            root: root.to_string_lossy().replace('\\', "/"),
        });
    }
    Ok(())
}

/// The leading path components that carry no wildcard, so the walk starts at the narrowest directory
/// the pattern already names instead of scanning the whole workspace.
fn literal_prefix(pattern: &str) -> PathBuf {
    let mut prefix = PathBuf::new();
    for component in pattern.split('/') {
        if component.contains('*') || component.contains('?') {
            break;
        }
        prefix.push(component);
    }
    prefix
}

fn walk(root: &Path, path: &Path, pattern: &str, found: &mut Vec<PathBuf>) -> Result<(), GlobError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        // A build that has not produced the directory yet matches nothing; the caller reports the
        // count and decides what it means.
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(read_error(pattern, path, source)),
    };
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.is_file() {
        push_if_matches(root, path, pattern, found);
        return Ok(());
    }
    if !metadata.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(path).map_err(|source| read_error(pattern, path, source))?;
    for entry in entries {
        let entry = entry.map_err(|source| read_error(pattern, path, source))?;
        let file_type = entry.file_type().map_err(|source| read_error(pattern, path, source))?;
        if file_type.is_symlink() {
            continue;
        }
        let child = entry.path();
        if file_type.is_dir() {
            if entry.file_name() == ".git" {
                continue;
            }
            walk(root, &child, pattern, found)?;
        } else if file_type.is_file() {
            push_if_matches(root, &child, pattern, found);
        }
    }
    Ok(())
}

fn push_if_matches(root: &Path, path: &Path, pattern: &str, found: &mut Vec<PathBuf>) {
    let Ok(relative) = path.strip_prefix(root) else {
        return;
    };
    let relative = relative.to_string_lossy().replace('\\', "/");
    if glob_matches(pattern, &relative) {
        found.push(path.to_path_buf());
    }
}

fn read_error(pattern: &str, path: &Path, source: io::Error) -> GlobError {
    GlobError::Read {
        pattern: pattern.to_string(),
        path: path.to_string_lossy().replace('\\', "/"),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    /// One scratch workspace per test, outside the repository.
    fn workspace() -> tempfile::TempDir {
        tempfile::tempdir().expect("a scratch workspace")
    }

    fn write(root: &Path, relative: &str, bytes: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("the fixture directory is writable");
        }
        fs::write(path, bytes).expect("the fixture file is writable");
    }

    fn expanded(root: &Path, pattern: &str) -> Vec<String> {
        expand(root, pattern)
            .unwrap_or_else(|err| panic!("the glob {pattern:?} stays inside the workspace: {err}"))
            .iter()
            .map(|path| {
                path.strip_prefix(root)
                    .expect("an expanded path is inside the workspace")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect()
    }

    /// S3: a workspace-relative pattern expands into the files it matches, and only those.
    #[test]
    fn a_workspace_relative_glob_expands_to_the_files_it_matches() {
        let dir = workspace();
        let root = dir.path();
        write(root, "dist/pkg-1.0.tar.gz", "a");
        write(root, "dist/pkg-2.0.tar.gz", "b");
        write(root, "dist/notes.txt", "c");

        assert_eq!(
            expanded(root, "dist/*.tar.gz"),
            vec!["dist/pkg-1.0.tar.gz", "dist/pkg-2.0.tar.gz"]
        );
        assert_eq!(expanded(root, "dist/notes.txt"), vec!["dist/notes.txt"]);
        assert_eq!(expanded(root, "dist/note?.txt"), vec!["dist/notes.txt"]);
    }

    /// S3: `**` is the token that reaches into subdirectories, at any depth, including none.
    #[test]
    fn a_recursive_glob_reaches_every_subdirectory() {
        let dir = workspace();
        let root = dir.path();
        write(root, "artifacts/pkg.whl", "a");
        write(root, "artifacts/linux/x86_64/pkg.whl", "b");
        write(root, "artifacts/linux/aarch64/pkg.whl", "c");
        write(root, "artifacts/linux/x86_64/pkg.tar.gz", "d");

        assert_eq!(
            expanded(root, "artifacts/**/*.whl"),
            vec![
                "artifacts/linux/aarch64/pkg.whl",
                "artifacts/linux/x86_64/pkg.whl",
                "artifacts/pkg.whl",
            ]
        );
    }

    /// S3: the walk never descends into `.git`, and a directory whose name looks like a match is not
    /// a file the declaration asks for.
    #[test]
    fn the_walk_skips_git_and_matches_only_files() {
        let dir = workspace();
        let root = dir.path();
        write(root, "dist/pkg.tar.gz", "a");
        write(root, ".git/objects/loose.tar.gz", "b");
        fs::create_dir_all(root.join("dist/directory.tar.gz")).expect("the fixture directory exists");

        assert_eq!(expanded(root, "**/*.tar.gz"), vec!["dist/pkg.tar.gz"]);
    }

    /// S3: a pattern that would walk out of the workspace is an error, not a silent empty match.
    #[test]
    fn a_glob_that_escapes_the_workspace_is_an_error_not_an_empty_match() {
        let dir = workspace();
        let root = dir.path();
        write(root, "keep.txt", "a");

        for pattern in ["../*", "a/../../*.txt", "/etc/*"] {
            match expand(root, pattern) {
                Err(GlobError::OutsideWorkspace { pattern: reported, .. }) => {
                    assert_eq!(reported, pattern, "the error names the offending glob");
                }
                other => panic!("the glob {pattern:?} must be refused, got {other:?}"),
            }
        }
    }

    /// A declared directory that the build has not produced yet matches nothing; it is not an error
    /// of its own, because the count is what tells the caller how far the build got.
    #[test]
    fn a_glob_over_a_missing_directory_matches_nothing() {
        let dir = workspace();
        assert!(expanded(dir.path(), "dist/*.tar.gz").is_empty());
    }

    /// The matcher keeps the semantics the plugin manager pinned before the move
    /// (`src/plugin/manager/tests.rs::test_glob_matches`).
    #[test]
    fn the_matcher_keeps_the_plugin_managers_semantics() {
        assert!(glob_matches("Chart.yaml", "Chart.yaml"));
        assert!(!glob_matches("Chart.yaml", "other.yaml"));
        assert!(glob_matches("*.yaml", "Chart.yaml"));
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

    /// A symlink is not followed: a link inside the workspace can name any directory on the machine,
    /// so following it would let a glob reach outside the root the declaration promised to stay in.
    #[cfg(unix)]
    #[test]
    fn the_walk_does_not_follow_a_symlink_out_of_the_workspace() {
        let dir = workspace();
        let root = dir.path();
        let outside = tempfile::tempdir().expect("a scratch directory outside the workspace");
        fs::write(outside.path().join("secret.txt"), "x").expect("the outside file is writable");
        std::os::unix::fs::symlink(outside.path(), root.join("link")).expect("the symlink is created");

        assert!(expanded(root, "link/*.txt").is_empty());
        assert!(expanded(root, "**/*.txt").is_empty());
    }
}
