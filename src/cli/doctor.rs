use crate::bump::{ChangelogDrift, Drift};
use crate::cli::path::relativize_path;
use crate::config::Config;
use crate::manifest;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// Renders the complete status dashboard grid for valid doctor runs.
pub fn render_dashboard(config: &Config, check_changelog: bool, theme: &crate::cli::style::Theme) -> String {
    let mut out = String::new();
    let filename = "cutver.toml";
    let _ = writeln!(out, "{} {} is valid\n", theme.success_icon(), theme.bold(filename));

    let config_val = format!("valid ({filename})");
    let manifest_val = format_manifest_summary(config, theme);
    let preflight_val = format_preflight_summary(config);

    render_grid_row(&mut out, "Configuration", &config_val, theme);
    render_grid_row(&mut out, "Manifests", &manifest_val, theme);
    render_grid_row(&mut out, "Preflight", &preflight_val, theme);

    if check_changelog {
        render_grid_row(&mut out, "Changelog", "consistent with Git tags", theme);
    }

    out
}

/// Renders a single tabular row aligned to a fixed 16-character label width.
pub fn render_grid_row(out: &mut String, label: &str, value: &str, theme: &crate::cli::style::Theme) {
    let padded = pad_right(label, 16);
    let _ = writeln!(out, "  {}{value}", theme.muted(padded));
}

/// Formats the summary text for manifests row.
fn format_manifest_summary(config: &Config, theme: &crate::cli::style::Theme) -> String {
    let total = config.manifest.len();
    let primary = config.primary_manifest();

    let (path_str, ver_str) = match primary {
        Some(m) => {
            let ver = read_manifest_version(&config.root_dir, m).unwrap_or_else(|| "unknown".into());
            let rel_path = relativize_path(m.path.as_str(), Some(&config.root_dir));
            (rel_path, ver)
        }
        None => return "0 tracked".to_string(),
    };

    let styled_ver = theme.accent(&ver_str);
    if total <= 1 {
        let count_str = theme.accent(total.to_string());
        return format!("{count_str} tracked ({path_str} @ {styled_ver})");
    }

    let remaining = total - 1;
    let count_str = theme.accent(total.to_string());
    let more_str = theme.muted(format!("+{remaining} more"));
    format!("{count_str} tracked ({path_str} @ {styled_ver}, {more_str})")
}

/// Formats the summary text for preflight row.
fn format_preflight_summary(config: &Config) -> String {
    match config.preflight.as_slice() {
        [] => "0 steps defined".to_string(),
        [(name, step)] => format!("1 step defined ({name}: {})", step.command),
        _ => format!("{} steps defined", config.preflight.len()),
    }
}

/// Reads version from manifest file, ignoring read/parse errors for display.
fn read_manifest_version(root_dir: &Path, manifest: &crate::config::Manifest) -> Option<String> {
    let target = root_dir.join(&manifest.path);
    let content = fs::read_to_string(target).ok()?;
    let editor = manifest::editor_for(manifest).ok()?;
    let version = editor.read_version(&content).ok()?;
    Some(version.to_string())
}

/// Renders version drift diagnostic block answering: what, where, and how to fix.
pub fn render_version_drift(drifts: &[Drift], root_dir: Option<&Path>, theme: &crate::cli::style::Theme) -> String {
    let mut out = String::new();
    let count = drifts.len();
    let count_styled = theme.accent(count.to_string());
    let _ = writeln!(
        out,
        "{} Version drift detected ({count_styled} manifest(s) out of sync):",
        theme.error_icon()
    );

    for Drift { path, expected, actual } in drifts {
        let norm_path = relativize_path(path, root_dir);
        let exp = theme.accent(expected);
        let act = theme.error(actual);
        let _ = writeln!(out, "  {} {norm_path}: expected {exp}, found {act}", theme.bullet());
    }

    let _ = writeln!(out, "\nSuggested fix:");
    let _ = writeln!(
        out,
        "  {} Align version numbers manually across all files.",
        theme.bullet()
    );
    let bump_cmd = theme.accent("cutver bump patch");
    let _ = writeln!(
        out,
        "  {} Or run '{bump_cmd}' to synchronize all manifests in one step.",
        theme.bullet()
    );
    out
}

/// Renders changelog drift diagnostic block.
pub fn render_changelog_drift(cl_drift: &ChangelogDrift, theme: &crate::cli::style::Theme) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{} Changelog drift detected:", theme.error_icon());

    append_missing_changelog(&mut out, &cl_drift.missing_in_changelog, theme);
    append_orphan_sections(&mut out, &cl_drift.orphan_sections, theme);

    let _ = writeln!(out, "\nSuggested next steps:");
    let cmd = theme.accent("cutver changelog show <version>");
    let _ = writeln!(
        out,
        "  {} Add missing release sections to CHANGELOG.md or query with '{cmd}'.",
        theme.bullet()
    );
    let _ = writeln!(
        out,
        "  {} Check if orphan versions were tagged with a different prefix or not yet pushed.",
        theme.bullet()
    );
    out
}

fn append_missing_changelog(out: &mut String, tags: &[String], theme: &crate::cli::style::Theme) {
    if tags.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n  Missing in changelog (Git tag exists):");
    for tag in tags {
        let _ = writeln!(out, "    {} {}", theme.bullet(), theme.accent(tag));
    }
}

fn append_orphan_sections(out: &mut String, sections: &[String], theme: &crate::cli::style::Theme) {
    if sections.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n  Orphan changelog sections (no Git tag exists):");
    for sec in sections {
        let _ = writeln!(out, "    {} {}", theme.bullet(), theme.error(sec));
    }
}

fn pad_right(text: &str, width: usize) -> String {
    let count = width.saturating_sub(text.len());
    format!("{text}{}", " ".repeat(count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::style::Theme;
    use crate::config::{Config, Manifest, ManifestKind, ManifestPath, PreflightCommand, VersionSection};
    use std::path::PathBuf;

    #[test]
    fn test_render_grid_row_padding() {
        let theme = Theme::new(false);
        let mut out = String::new();
        render_grid_row(&mut out, "Configuration", "valid (cutver.toml)", &theme);
        assert_eq!(out, "  Configuration   valid (cutver.toml)\n");
    }

    #[test]
    fn test_render_dashboard_layout() {
        let theme = Theme::new(false);
        let config = Config {
            root_dir: PathBuf::from("."),
            project: Default::default(),
            version: VersionSection {
                current_source: "Cargo.toml".into(),
                strategy: "manual".into(),
            },
            manifest: vec![Manifest {
                path: ManifestPath::parse("Cargo.toml").unwrap(),
                primary: true,
                kind: ManifestKind::CargoPackage,
            }],
            preflight: vec![(
                "test".into(),
                PreflightCommand {
                    command: "cargo test".into(),
                    timeout: None,
                },
            )],
            preflight_default_timeout: None,
            changelog: Default::default(),
            git: Default::default(),
            hooks: Default::default(),
            publish: Default::default(),
            plugins: Default::default(),
        };

        let rendered = render_dashboard(&config, true, &theme);
        assert!(rendered.contains("✔ cutver.toml is valid\n\n"));
        assert!(rendered.contains("  Configuration   valid (cutver.toml)\n"));
        assert!(rendered.contains("  Manifests       1 tracked (Cargo.toml @ "));
        assert!(rendered.contains("  Preflight       1 step defined (test: cargo test)\n"));
        assert!(rendered.contains("  Changelog       consistent with Git tags\n"));
    }

    #[test]
    fn test_render_version_drift() {
        let theme = Theme::new(false);
        let drifts = vec![Drift {
            path: "/path/to/repo/Cargo.toml".into(),
            expected: "1.2.0".into(),
            actual: "1.1.0".into(),
        }];
        let root = Path::new("/path/to/repo");
        let rendered = render_version_drift(&drifts, Some(root), &theme);
        assert!(rendered.contains("Version drift detected (1 manifest(s) out of sync):"));
        assert!(rendered.contains("Cargo.toml: expected 1.2.0, found 1.1.0"));
        assert!(rendered.contains("Suggested fix:"));
        assert!(rendered.contains("cutver bump patch"));
    }

    #[test]
    fn test_render_changelog_drift() {
        let theme = Theme::new(false);
        let cl_drift = ChangelogDrift {
            missing_in_changelog: vec!["v1.1.0".into()],
            orphan_sections: vec!["1.2.0".into()],
        };
        let rendered = render_changelog_drift(&cl_drift, &theme);
        assert!(rendered.contains("Changelog drift detected:"));
        assert!(rendered.contains("Missing in changelog (Git tag exists):"));
        assert!(rendered.contains("v1.1.0"));
        assert!(rendered.contains("Orphan changelog sections (no Git tag exists):"));
        assert!(rendered.contains("1.2.0"));
        assert!(rendered.contains("Suggested next steps:"));
    }
}
