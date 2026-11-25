use semver::Version;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bump {
    Patch,
    Minor,
    Major,
}

pub fn bump(version: &Version, bump: Bump) -> Version {
    let mut next = Version::new(version.major, version.minor, version.patch);
    match bump {
        Bump::Patch => next.patch += 1,
        Bump::Minor => {
            next.minor += 1;
            next.patch = 0;
        }
        Bump::Major => {
            next.major += 1;
            next.minor = 0;
            next.patch = 0;
        }
    }
    next
}

/// The prerelease channel a release is published on (`--channel`, #326 gap 6, ADR-0005).
///
/// The ladder is a channel: `alpha`, `beta` and `rc` carry a counter on the version, and `stable` is
/// the release itself. The bare word "channel" always means this ladder; the CI output channel and the
/// gate channel always carry their qualifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Alpha,
    Beta,
    Rc,
    Stable,
}

impl Channel {
    /// The identifier the version carries, or `stable` for a release.
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Alpha => "alpha",
            Channel::Beta => "beta",
            Channel::Rc => "rc",
            Channel::Stable => "stable",
        }
    }
}

/// The version `channel` makes of the version the level cut.
///
/// A prerelease channel starts its own counter at `1` on the base version, so `1.3.0` on `alpha` is
/// `1.3.0-alpha.1`. `stable` adds nothing: the base already carries no prerelease segment. What a
/// channel does to a version that is *already* a prerelease is promotion, a different rule on purpose
/// (#326 D1) — [`next_on_channel`] is the one place that decides between the two.
pub fn apply_channel(version: &Version, channel: Channel) -> Version {
    let mut next = version.clone();
    if channel != Channel::Stable {
        next.pre = semver::Prerelease::new(&format!("{}.1", channel.as_str()))
            .expect("a ladder name and a counter are a valid prerelease");
    }
    next
}

/// The version a channel makes of the workspace it is asked to release (#326 D1).
///
/// `level_version` is the version the level — or the versioning plugin — cut, and it decides the
/// answer only where the workspace is on a stable version: a named level survives `--channel stable`
/// (`1.2.3` a minor becomes `1.3.0`), and a prerelease channel starts its counter on that base
/// (`1.3.0-alpha.1`, D1a). A workspace that is *already* on a prerelease is promoted instead, and
/// the level is ignored: the counter moves (`1.2.0-alpha.1` → `1.2.0-alpha.2`), another channel
/// restarts on the same base (`1.2.0-alpha.3` on `beta` → `1.2.0-beta.1`), or `stable` drops the
/// suffix without incrementing (`1.2.0-rc.1` → `1.2.0`). No rung is validated: `alpha` promotes to
/// `stable` exactly as `rc` does.
pub fn next_on_channel(current: &Version, level_version: &Version, channel: Channel) -> Version {
    if current.pre.is_empty() {
        return apply_channel(level_version, channel);
    }
    promotion(current, channel)
}

/// The version a promotion makes of a prerelease: the current version decides, and the level never
/// does (#326 D1).
fn promotion(current: &Version, channel: Channel) -> Version {
    let mut next = Version::new(current.major, current.minor, current.patch);
    if channel != Channel::Stable {
        next.pre = if channel.as_str() == channel_of(current) {
            increment_counter(&current.pre)
        } else {
            counter(channel, 1)
        };
    }
    next
}

fn counter(channel: Channel, value: u64) -> semver::Prerelease {
    semver::Prerelease::new(&format!("{}.{value}", channel.as_str()))
        .expect("a ladder name and a counter are a valid prerelease")
}

/// The next counter on the suffix the version already carries: `alpha.1` → `alpha.2`.
///
/// A suffix with no numeric counter (`alpha`) starts at `1`, and a counter that cannot go higher
/// saturates instead of wrapping, so a version the tool did not cut never reads as a lower one.
fn increment_counter(pre: &semver::Prerelease) -> semver::Prerelease {
    let mut parts = pre.as_str().split('.');
    let name = parts.next().unwrap_or_default();
    let value = parts.next().and_then(|n| n.parse::<u64>().ok()).unwrap_or(0);
    semver::Prerelease::new(&format!("{name}.{}", value.saturating_add(1)))
        .expect("a ladder name and a counter are a valid prerelease")
}

/// The channel a version is already published on, as the version itself reads: `stable` for a
/// release, the identifier it carries for a prerelease. An identifier outside the ladder reads as
/// itself, so a version the tool did not cut is never relabelled by a report.
pub fn channel_of(version: &Version) -> &str {
    if version.pre.is_empty() {
        Channel::Stable.as_str()
    } else {
        version
            .pre
            .as_str()
            .split('.')
            .next()
            .unwrap_or(Channel::Stable.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        s.parse().unwrap()
    }

    #[test]
    fn a_channel_starts_its_counter_on_the_version() {
        assert_eq!(apply_channel(&v("1.3.0"), Channel::Alpha), v("1.3.0-alpha.1"));
        assert_eq!(apply_channel(&v("1.3.0"), Channel::Beta), v("1.3.0-beta.1"));
        assert_eq!(apply_channel(&v("1.3.0"), Channel::Rc), v("1.3.0-rc.1"));
    }

    #[test]
    fn stable_adds_no_prerelease_segment() {
        assert_eq!(apply_channel(&v("1.3.0"), Channel::Stable), v("1.3.0"));
    }

    #[test]
    fn promotion_moves_only_the_counter_of_the_channel_it_is_on() {
        assert_eq!(
            next_on_channel(&v("1.2.0-alpha.1"), &v("1.2.1"), Channel::Alpha),
            v("1.2.0-alpha.2")
        );
        assert_eq!(
            next_on_channel(&v("1.2.0-rc.9"), &v("1.2.9"), Channel::Rc),
            v("1.2.0-rc.10")
        );
    }

    #[test]
    fn promotion_to_another_channel_restarts_its_counter_on_the_same_base() {
        assert_eq!(
            next_on_channel(&v("1.2.0-alpha.3"), &v("1.2.1"), Channel::Beta),
            v("1.2.0-beta.1")
        );
        assert_eq!(
            next_on_channel(&v("1.2.0-beta.7"), &v("1.3.0"), Channel::Rc),
            v("1.2.0-rc.1")
        );
    }

    #[test]
    fn promotion_to_stable_drops_the_suffix_without_incrementing() {
        assert_eq!(
            next_on_channel(&v("1.2.0-rc.1"), &v("1.2.1"), Channel::Stable),
            v("1.2.0")
        );
        // D1: no rung is validated — `alpha` promotes as `rc` does.
        assert_eq!(
            next_on_channel(&v("1.2.0-alpha.3"), &v("1.2.1"), Channel::Stable),
            v("1.2.0")
        );
    }

    #[test]
    fn a_stable_version_keeps_the_base_the_level_cut() {
        // D1a: a named level survives `--channel stable`; the channel adds nothing to it.
        assert_eq!(next_on_channel(&v("1.2.3"), &v("1.3.0"), Channel::Stable), v("1.3.0"));
        assert_eq!(
            next_on_channel(&v("1.2.3"), &v("1.3.0"), Channel::Beta),
            v("1.3.0-beta.1")
        );
    }

    #[test]
    fn promotion_of_a_suffix_with_no_counter_starts_it_at_one() {
        assert_eq!(
            next_on_channel(&v("1.2.0-alpha"), &v("1.2.1"), Channel::Alpha),
            v("1.2.0-alpha.1")
        );
    }

    #[test]
    fn promotion_drops_build_metadata_like_a_level_does() {
        assert_eq!(
            next_on_channel(&v("1.2.0-alpha.1+build.7"), &v("1.2.1"), Channel::Alpha),
            v("1.2.0-alpha.2")
        );
    }

    #[test]
    fn a_version_reads_back_the_channel_it_carries() {
        assert_eq!(channel_of(&v("1.3.0")), "stable");
        assert_eq!(channel_of(&v("1.3.0-alpha.1")), "alpha");
        assert_eq!(channel_of(&v("1.3.0-rc.12")), "rc");
        assert_eq!(channel_of(&v("1.3.0-experimental.1")), "experimental");
    }

    #[test]
    fn patch_bump() {
        assert_eq!(bump(&v("1.2.3"), Bump::Patch), v("1.2.4"));
    }

    #[test]
    fn minor_bump() {
        assert_eq!(bump(&v("1.2.3"), Bump::Minor), v("1.3.0"));
    }

    #[test]
    fn major_bump() {
        assert_eq!(bump(&v("1.2.3"), Bump::Major), v("2.0.0"));
    }

    #[test]
    fn drops_prerelease_and_build_metadata() {
        assert_eq!(bump(&v("1.2.3-alpha.1"), Bump::Minor), v("1.3.0"));
        assert_eq!(bump(&v("1.2.3+build.42"), Bump::Patch), v("1.2.4"));
        assert_eq!(bump(&v("1.2.3-rc.1+build.2"), Bump::Major), v("2.0.0"));
    }

    #[test]
    fn zero_components_stay_zero() {
        assert_eq!(bump(&v("0.5.1"), Bump::Minor), v("0.6.0"));
    }

    #[test]
    fn major_bump_from_zero_zero_x() {
        assert_eq!(bump(&v("0.0.7"), Bump::Major), v("1.0.0"));
    }

    #[test]
    fn patch_and_minor_from_zero_zero_x() {
        assert_eq!(bump(&v("0.0.7"), Bump::Patch), v("0.0.8"));
        assert_eq!(bump(&v("0.0.7"), Bump::Minor), v("0.1.0"));
    }

    #[test]
    fn field_math_idempotency() {
        let base = v("1.2.3");
        assert_eq!(bump(&bump(&base, Bump::Patch), Bump::Patch), v("1.2.5"));
        assert_eq!(bump(&bump(&base, Bump::Minor), Bump::Minor), v("1.4.0"));
        assert_eq!(bump(&bump(&base, Bump::Major), Bump::Major), v("3.0.0"));
    }
}
