pub mod io;
pub mod parser;
#[cfg(test)]
mod tests;

pub use io::{read_latest, read_version};
pub use parser::{extract_latest, extract_version, list_versions};

/// The release entry a published body should carry: the newest section of an already-rendered
/// changelog, heading included.
///
/// The host renders the changelog through the project's template first, so this is the same text
/// the release entry holds — one source, no second rendering. A plugin receives it instead of
/// re-deriving notes from the file, which is what keeps a published release and its changelog
/// byte-identical.
pub fn release_notes(rendered_changelog: &str) -> Option<String> {
    extract_latest(rendered_changelog, true)
}
