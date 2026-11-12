//! `kind = "template"`: a version **sink** that generates its target from a Jinja template.
//!
//! Unlike every other editor this one does not read a version out of its file, because the file is
//! output, not input. `read_version` therefore fails with an actionable message instead of guessing,
//! and drift is detected by re-rendering the template and comparing bytes (see
//! [`TemplateEditor::render`]), which is what `cutver doctor` uses.

use super::{Error, ManifestEditor};
use crate::changelog::{ReleaseContext, render_source};
use semver::Version;
use std::fs;
use std::path::{Path, PathBuf};

/// Default template file for a target path: the target with `.jinja` appended.
fn default_template_for(target: &Path) -> PathBuf {
    let mut name = target.as_os_str().to_os_string();
    name.push(".jinja");
    PathBuf::from(name)
}

#[derive(Debug, Clone)]
pub struct TemplateEditor {
    target: PathBuf,
    template: PathBuf,
}

impl TemplateEditor {
    /// Builds the editor for a target path and an optional configured template path.
    ///
    /// Both paths are already repo-relative-resolved by the caller, which owns `cutver.toml`'s
    /// root directory.
    pub fn new(target: impl Into<PathBuf>, template: Option<impl Into<PathBuf>>) -> Self {
        let target = target.into();
        let template = template.map_or_else(|| default_template_for(&target), Into::into);
        Self { target, template }
    }

    pub fn target(&self) -> &Path {
        &self.target
    }

    pub fn template(&self) -> &Path {
        &self.template
    }

    /// Renders the template for `context`, byte-exact, without touching the target.
    ///
    /// A missing or unreadable template is an actionable error naming the path, never a silent
    /// empty render.
    pub fn render(&self, context: &ReleaseContext) -> Result<String, Error> {
        let source = fs::read_to_string(&self.template).map_err(|source| Error::TemplateRead {
            path: self.template.display().to_string(),
            source,
        })?;
        render_source(&source, context).map_err(|e| Error::TemplateRender { detail: e.to_string() })
    }
}

impl ManifestEditor for TemplateEditor {
    fn as_template(&self) -> Option<&TemplateEditor> {
        Some(self)
    }

    /// A template sink generates its file from the version; it never supplies one.
    fn read_version(&self, _content: &str) -> Result<Version, Error> {
        Err(Error::TemplateSinkHasNoVersion {
            path: self.target.display().to_string(),
        })
    }

    /// Renders with `version` alone, for the editors that only know a version.
    ///
    /// Everything richer — `tag`, `date`, `owner` — comes from [`TemplateEditor::render`], which the
    /// pipeline calls with the assembled [`ReleaseContext`].
    fn write_version(&self, _content: &str, _version: &Version) -> Result<String, Error> {
        Err(Error::TemplateNeedsReleaseContext {
            path: self.target.display().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changelog::assemble_release_context;
    use crate::config::Changelog;

    fn context(version: &str, tag: &str) -> ReleaseContext {
        assemble_release_context(crate::changelog::AssembleContextParams {
            version,
            prev_version: None,
            tag,
            prev_tag: None,
            date: "2026-11-12",
            repository: Some("https://github.com/acme/demo".to_string()),
            parsed_commits: &[],
            raw_commits: None,
            contributors: Vec::new(),
            first_time_contributors: Vec::new(),
            changelog_config: &Changelog::default(),
        })
    }

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn default_template_appends_jinja_to_the_target_name() {
        let editor = TemplateEditor::new("/repo/src/version.ts", None::<PathBuf>);
        assert_eq!(editor.template(), Path::new("/repo/src/version.ts.jinja"));
        assert_eq!(editor.target(), Path::new("/repo/src/version.ts"));
    }

    #[test]
    fn renders_the_target_byte_exact_including_the_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("version.ts");
        let template = dir.path().join("v.jinja");
        write(&template, "export const VERSION = \"{{ version }}\";\n");
        let editor = TemplateEditor::new(&target, Some(&template));

        let rendered = editor.render(&context("1.4.2", "v1.4.2")).unwrap();
        assert_eq!(rendered, "export const VERSION = \"1.4.2\";\n");
    }

    #[test]
    fn reads_a_version_from_the_release_context() {
        let dir = tempfile::tempdir().unwrap();
        let template = dir.path().join("v.jinja");
        write(&template, "{{ tag }} / {{ major }}.{{ minor }}.{{ patch }}");
        let editor = TemplateEditor::new(dir.path().join("version.h"), Some(&template));

        let rendered = editor.render(&context("2.3.4", "v2.3.4")).unwrap();
        assert_eq!(rendered, "v2.3.4 / 2.3.4");
    }

    #[test]
    fn missing_template_file_names_the_path() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.jinja");
        let editor = TemplateEditor::new(dir.path().join("version.h"), Some(&missing));

        let err = editor.render(&context("1.0.0", "v1.0.0")).unwrap_err();
        assert!(matches!(err, Error::TemplateRead { .. }), "unexpected error: {err}");
        assert!(
            err.to_string().contains("nope.jinja"),
            "the error must name the template: {err}"
        );
    }

    #[test]
    fn reading_a_version_from_a_sink_fails_closed() {
        let editor = TemplateEditor::new("/repo/version.h", None::<PathBuf>);
        let err = editor.read_version("anything").unwrap_err();
        assert!(
            matches!(err, Error::TemplateSinkHasNoVersion { .. }),
            "unexpected error: {err}"
        );
        assert!(err.to_string().contains("version.h"));
    }
}
