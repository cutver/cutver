pub use super::commit::CommitContext;
pub use super::release::ReleaseContext;

/// Extracted repository forge metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeMetadata {
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub forge: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpolationContext {
    pub version: String,
    pub previous_version: Option<String>,
    pub tag: String,
    pub previous_tag: Option<String>,
    pub bump_level: String,
    pub is_prerelease: bool,
    pub prerelease: Option<String>,
    pub build: Option<String>,
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub date: String,
    pub branch: Option<String>,
    pub repo_owner: Option<String>,
    pub repo_name: Option<String>,
    pub forge: Option<String>,
    pub project_name: Option<String>,
}

impl serde::Serialize for InterpolationContext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(20))?;
        map.serialize_entry("version", &self.version)?;
        map.serialize_entry("previous_version", &self.previous_version)?;
        map.serialize_entry("tag", &self.tag)?;
        map.serialize_entry("previous_tag", &self.previous_tag)?;
        map.serialize_entry("bump_level", &self.bump_level)?;
        map.serialize_entry("is_prerelease", &self.is_prerelease)?;
        map.serialize_entry("prerelease", &self.prerelease)?;
        map.serialize_entry("build", &self.build)?;
        map.serialize_entry("major", &self.major)?;
        map.serialize_entry("minor", &self.minor)?;
        map.serialize_entry("patch", &self.patch)?;
        map.serialize_entry("date", &self.date)?;
        map.serialize_entry("branch", &self.branch)?;
        map.serialize_entry("repo_owner", &self.repo_owner)?;
        map.serialize_entry("owner", &self.repo_owner)?;
        map.serialize_entry("repo_name", &self.repo_name)?;
        map.serialize_entry("repo", &self.repo_name)?;
        map.serialize_entry("forge", &self.forge)?;
        map.serialize_entry("project_name", &self.project_name)?;
        map.serialize_entry("project", &self.project_name)?;
        map.end()
    }
}

impl<'de> serde::Deserialize<'de> for InterpolationContext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Helper {
            version: String,
            #[serde(default)]
            previous_version: Option<String>,
            tag: String,
            #[serde(default)]
            previous_tag: Option<String>,
            #[serde(default)]
            bump_level: String,
            #[serde(default)]
            is_prerelease: Option<bool>,
            #[serde(default)]
            prerelease: Option<String>,
            #[serde(default)]
            build: Option<String>,
            #[serde(default)]
            major: Option<u64>,
            #[serde(default)]
            minor: Option<u64>,
            #[serde(default)]
            patch: Option<u64>,
            #[serde(default)]
            date: String,
            #[serde(default)]
            branch: Option<String>,
            #[serde(default, alias = "owner")]
            repo_owner: Option<String>,
            #[serde(default, alias = "repo")]
            repo_name: Option<String>,
            #[serde(default)]
            forge: Option<String>,
            #[serde(default, alias = "project")]
            project_name: Option<String>,
        }

        let h = Helper::deserialize(deserializer)?;
        let (major, minor, patch, is_prerelease, prerelease, build) = parse_semver_parts(
            &h.version,
            h.major,
            h.minor,
            h.patch,
            h.is_prerelease,
            h.prerelease,
            h.build,
        );

        let repo_owner = h.repo_owner;
        let repo_name = h.repo_name;
        let project_name = h.project_name.or_else(|| repo_name.clone());

        Ok(InterpolationContext {
            version: h.version,
            previous_version: h.previous_version,
            tag: h.tag,
            previous_tag: h.previous_tag,
            bump_level: h.bump_level,
            is_prerelease,
            prerelease,
            build,
            major,
            minor,
            patch,
            date: h.date,
            branch: h.branch,
            repo_owner,
            repo_name,
            forge: h.forge,
            project_name,
        })
    }
}

impl InterpolationContext {
    /// Build an `InterpolationContext` from release parameters and optional repository information.
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        version: &str,
        previous_version: Option<&str>,
        tag: &str,
        previous_tag: Option<&str>,
        bump_level: &str,
        date: &str,
        branch: Option<&str>,
        repository: Option<&str>,
    ) -> Self {
        let (major, minor, patch, is_prerelease, prerelease, build) =
            parse_semver_parts(version, None, None, None, None, None, None);
        let forge_meta = repository.map(super::parse::parse_repo_forge);
        let repo_owner = forge_meta.as_ref().and_then(|f| f.owner.clone());
        let repo_name = forge_meta.as_ref().and_then(|f| f.repo.clone());
        let forge = forge_meta.as_ref().and_then(|f| f.forge.clone());
        let project_name = repo_name.clone();

        InterpolationContext {
            version: version.to_string(),
            previous_version: previous_version.map(String::from),
            tag: tag.to_string(),
            previous_tag: previous_tag.map(String::from),
            bump_level: bump_level.to_string(),
            is_prerelease,
            prerelease,
            build,
            major,
            minor,
            patch,
            date: date.to_string(),
            branch: branch.map(String::from),
            repo_owner,
            repo_name,
            forge,
            project_name,
        }
    }

    /// Build an `InterpolationContext` from a `ReleaseContext`.
    pub fn from_release_context(ctx: &ReleaseContext, bump_level: &str, branch: Option<&str>) -> Self {
        InterpolationContext {
            version: ctx.version.clone(),
            previous_version: ctx.previous_version.clone(),
            tag: ctx.tag.clone(),
            previous_tag: ctx.previous_tag.clone(),
            bump_level: bump_level.to_string(),
            is_prerelease: ctx.is_prerelease,
            prerelease: ctx.prerelease.clone(),
            build: ctx.build.clone(),
            major: ctx.major,
            minor: ctx.minor,
            patch: ctx.patch,
            date: ctx.date.clone(),
            branch: branch.map(String::from),
            repo_owner: ctx.repo_owner.clone(),
            repo_name: ctx.repo_name.clone(),
            forge: ctx.forge.clone(),
            project_name: ctx.project_name.clone(),
        }
    }
}

pub struct AssembleContextParams<'a> {
    pub version: &'a str,
    pub prev_version: Option<&'a str>,
    pub tag: &'a str,
    pub prev_tag: Option<&'a str>,
    pub date: &'a str,
    pub repository: Option<String>,
    pub parsed_commits: &'a [crate::conventional::ConventionalCommit],
    pub raw_commits: Option<&'a [crate::git::RawCommit]>,
    pub contributors: Vec<String>,
    pub first_time_contributors: Vec<String>,
    pub changelog_config: &'a crate::config::ChangelogConfig,
}

pub(crate) fn parse_semver_parts(
    v: &str,
    major: Option<u64>,
    minor: Option<u64>,
    patch: Option<u64>,
    is_prerelease: Option<bool>,
    prerelease: Option<String>,
    build: Option<String>,
) -> (u64, u64, u64, bool, Option<String>, Option<String>) {
    let s = semver::Version::parse(v).ok();
    let maj = major.or_else(|| s.as_ref().map(|x| x.major)).unwrap_or(0);
    let min = minor.or_else(|| s.as_ref().map(|x| x.minor)).unwrap_or(0);
    let pat = patch.or_else(|| s.as_ref().map(|x| x.patch)).unwrap_or(0);
    let pre_b = is_prerelease
        .or_else(|| s.as_ref().map(|x| !x.pre.is_empty()))
        .unwrap_or(false);
    let pre_s = prerelease.or_else(|| {
        s.as_ref().and_then(|x| {
            if x.pre.is_empty() {
                None
            } else {
                Some(x.pre.as_str().to_string())
            }
        })
    });
    let bld = build.or_else(|| {
        s.as_ref().and_then(|x| {
            if x.build.is_empty() {
                None
            } else {
                Some(x.build.as_str().to_string())
            }
        })
    });
    (maj, min, pat, pre_b, pre_s, bld)
}
