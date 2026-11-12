//! Dispatch behaviour tests: the kind-to-editor mapping a configured manifest resolves to.

use super::editor_for;
use crate::config::{Manifest, ManifestKind, ManifestPath};
use semver::Version;

fn pyproject_entry(table: Option<&str>) -> Manifest {
    Manifest {
        path: ManifestPath::parse("pyproject.toml").unwrap(),
        package: None,
        primary: true,
        kind: ManifestKind::Pyproject {
            table: table.map(str::to_string),
        },
    }
}

#[test]
fn editor_for_pyproject() {
    let editor = editor_for(&pyproject_entry(None)).expect("editor_for should succeed for pyproject");
    let content = "[project]\nname = \"demo\"\nversion = \"1.0.0\"\n";
    let v = editor.read_version(content).unwrap();
    assert_eq!(v, Version::parse("1.0.0").unwrap());
}

#[test]
fn editor_for_pyproject_with_table() {
    let editor =
        editor_for(&pyproject_entry(Some("tool.poetry"))).expect("editor_for should succeed for pyproject with table");
    let content = "[tool.poetry]\nname = \"demo\"\nversion = \"2.0.0\"\n";
    let v = editor.read_version(content).unwrap();
    assert_eq!(v, Version::parse("2.0.0").unwrap());
}
