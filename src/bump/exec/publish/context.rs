//! What the closing phase reads before it writes: the interpolation context `[publish].commands`
//! render against, the notes this invocation carries, and the version the previous tag names. Out of
//! the facade to stay under the source ceiling; behaviour is unchanged.

use std::io::{self, Read};

use semver::Version;

use crate::bump::Error;
use crate::bump::exec::command::read;
use crate::bump::exec::guard;
use crate::bump::types::BumpLevel;
use crate::changelog;
use crate::config::Config;
use crate::git;

use super::{NOTES_STDIN, PublishRequest, notes_are_streamed};

pub(super) fn publish_context(
    config: &Config,
    request: &PublishRequest,
    version: &Version,
) -> Result<(changelog::InterpolationContext, Option<String>), Error> {
    let repo = &config.root_dir;
    let prev_tag = git::latest_tag(repo, Some(&config.git.tag_prefix))?;
    // The closing phase runs after the bump, so the previous version is only readable from the tag.
    let previous = previous_version(prev_tag.as_deref(), &config.git.tag_prefix, version);
    let context = guard::build_context(
        repo,
        &previous,
        version,
        &request.tag,
        prev_tag.as_deref(),
        BumpLevel::Auto,
        false,
    );
    let notes = read_notes(request)?;
    Ok((context, notes))
}

/// The release notes this invocation carries: the declared file, or the stream when the path is `-`.
///
/// A stream that carries nothing is an error rather than `Some("")`: the closing phase publishes a
/// release nobody can correct afterwards, so empty notes are a refusal, not a default (#306).
fn read_notes(request: &PublishRequest) -> Result<Option<String>, Error> {
    match request.notes_file.as_deref() {
        None => Ok(None),
        Some(_) if notes_are_streamed(request) => {
            let mut notes = String::new();
            io::stdin().read_to_string(&mut notes).map_err(|source| Error::Read {
                path: NOTES_STDIN.to_string(),
                source,
            })?;
            if notes.trim().is_empty() {
                return Err(Error::EmptyNotesStream);
            }
            Ok(Some(notes))
        }
        Some(path) => Ok(Some(read(path)?)),
    }
}

/// The version the previous tag names, so the closing phase renders `previous_version` the way the
/// inline phase does. A tag that does not carry `tag_prefix` or a semver after it names no version.
fn previous_version(prev_tag: Option<&str>, tag_prefix: &str, version: &Version) -> Version {
    prev_tag
        .and_then(|tag| tag.strip_prefix(tag_prefix))
        .and_then(|rest| Version::parse(rest).ok())
        .unwrap_or_else(|| version.clone())
}
