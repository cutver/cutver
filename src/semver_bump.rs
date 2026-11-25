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
/// (#326 D1).
pub fn apply_channel(version: &Version, channel: Channel) -> Version {
    let mut next = version.clone();
    if channel != Channel::Stable {
        next.pre = semver::Prerelease::new(&format!("{}.1", channel.as_str()))
            .expect("a ladder name and a counter are a valid prerelease");
    }
    next
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
