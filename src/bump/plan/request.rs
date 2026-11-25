//! What a caller asks a plan to compute: the knobs `bump` and `cutver plan` share.
//!
//! Extracted from `plan.rs` unchanged when the prerelease channel (#326 gap 6) gave the request a
//! knob of its own: this is where a run says what it wants, and the plan is what it computed.

use crate::bump::types::BumpLevel;
use crate::semver_bump::Channel;

/// How the release described by the plan will treat its changelog entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlanMode {
    /// A real `bump`: the entry is planned and then written.
    Write,
    /// `bump --dry-run`: nothing is rendered, exactly as before (#326 S8).
    DryRun,
    /// `cutver plan`: the entry is rendered and never written, and the guards report instead of
    /// aborting (#326 S3, S4).
    Preview,
}

impl PlanMode {
    /// Whether this mode may only report what a real run would abort on.
    pub(crate) fn read_only(self) -> bool {
        matches!(self, PlanMode::Preview)
    }

    /// Whether the changelog entry is prepared for writing.
    pub(crate) fn writes_changelog(self) -> bool {
        matches!(self, PlanMode::Write)
    }
}

/// What `cutver plan` asks for. One struct instead of six positional parameters.
#[derive(Clone, Copy)]
pub(crate) struct ComputeRequest<'a> {
    pub(crate) level: BumpLevel,
    pub(crate) skip_preflight: &'a [String],
    pub(crate) first_release: bool,
    /// The prerelease channel the caller asked for, if any (#326 gap 6). `None` is a caller that named
    /// no channel, and the version keeps whatever the level and the manifests already say.
    pub(crate) channel: Option<Channel>,
    pub(crate) mode: PlanMode,
}
