use super::commit::CommitContext;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReleaseContext {
    pub version: String,
    pub previous_version: Option<String>,
    pub tag: String,
    pub previous_tag: Option<String>,
    pub date: String,
    pub compare_url: Option<String>,
    pub repository: Option<String>,
    pub features: String,
    pub fixes: String,
    pub breaking: String,
    pub perf: String,
    pub refactor: String,
    pub docs: String,
    pub maintenance: String,
    pub other: String,
    pub all_changes: String,
    pub commits: Vec<CommitContext>,
    pub contributors: Vec<String>,
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub is_prerelease: bool,
    pub prerelease: Option<String>,
    pub build: Option<String>,
    pub repo_owner: Option<String>,
    pub repo_name: Option<String>,
    pub forge: Option<String>,
    pub project_name: Option<String>,
    pub year: u32,
    pub month: u32,
    pub day: u32,
}

impl serde::Serialize for ReleaseContext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(34))?;
        map.serialize_entry("version", &self.version)?;
        map.serialize_entry("previous_version", &self.previous_version)?;
        map.serialize_entry("tag", &self.tag)?;
        map.serialize_entry("previous_tag", &self.previous_tag)?;
        map.serialize_entry("date", &self.date)?;
        map.serialize_entry("compare_url", &self.compare_url)?;
        map.serialize_entry("repository", &self.repository)?;
        map.serialize_entry("features", &self.features)?;
        map.serialize_entry("fixes", &self.fixes)?;
        map.serialize_entry("breaking", &self.breaking)?;
        map.serialize_entry("perf", &self.perf)?;
        map.serialize_entry("refactor", &self.refactor)?;
        map.serialize_entry("docs", &self.docs)?;
        map.serialize_entry("maintenance", &self.maintenance)?;
        map.serialize_entry("other", &self.other)?;
        map.serialize_entry("all_changes", &self.all_changes)?;
        map.serialize_entry("commits", &self.commits)?;
        map.serialize_entry("contributors", &self.contributors)?;

        map.serialize_entry("major", &self.major)?;
        map.serialize_entry("minor", &self.minor)?;
        map.serialize_entry("patch", &self.patch)?;
        map.serialize_entry("is_prerelease", &self.is_prerelease)?;
        map.serialize_entry("prerelease", &self.prerelease)?;
        map.serialize_entry("build", &self.build)?;

        map.serialize_entry("year", &self.year)?;
        map.serialize_entry("month", &self.month)?;
        map.serialize_entry("day", &self.day)?;

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

impl<'de> serde::Deserialize<'de> for ReleaseContext {
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
            date: String,
            #[serde(default)]
            compare_url: Option<String>,
            #[serde(default)]
            repository: Option<String>,
            #[serde(default)]
            features: String,
            #[serde(default)]
            fixes: String,
            #[serde(default)]
            breaking: String,
            #[serde(default)]
            perf: String,
            #[serde(default)]
            refactor: String,
            #[serde(default)]
            docs: String,
            #[serde(default)]
            maintenance: String,
            #[serde(default)]
            other: String,
            #[serde(default)]
            all_changes: String,
            #[serde(default)]
            commits: Vec<CommitContext>,
            #[serde(default)]
            contributors: Vec<String>,
            #[serde(default)]
            major: Option<u64>,
            #[serde(default)]
            minor: Option<u64>,
            #[serde(default)]
            patch: Option<u64>,
            #[serde(default)]
            is_prerelease: Option<bool>,
            #[serde(default)]
            prerelease: Option<String>,
            #[serde(default)]
            build: Option<String>,
            #[serde(default)]
            year: Option<u32>,
            #[serde(default)]
            month: Option<u32>,
            #[serde(default)]
            day: Option<u32>,
            #[serde(default)]
            repo_owner: Option<String>,
            #[serde(default)]
            owner: Option<String>,
            #[serde(default)]
            repo_name: Option<String>,
            #[serde(default)]
            repo: Option<String>,
            #[serde(default)]
            forge: Option<String>,
            #[serde(default)]
            project_name: Option<String>,
            #[serde(default)]
            project: Option<String>,
        }

        let h = Helper::deserialize(deserializer)?;
        let (major, minor, patch, is_prerelease, prerelease, build) = super::types::parse_semver_parts(
            &h.version,
            h.major,
            h.minor,
            h.patch,
            h.is_prerelease,
            h.prerelease,
            h.build,
        );

        let forge_meta = h.repository.as_deref().map(super::parse::parse_repo_forge);
        let repo_owner = h
            .repo_owner
            .or(h.owner)
            .or_else(|| forge_meta.as_ref().and_then(|f| f.owner.clone()));
        let repo_name = h
            .repo_name
            .or(h.repo)
            .or_else(|| forge_meta.as_ref().and_then(|f| f.repo.clone()));
        let forge = h.forge.or_else(|| forge_meta.as_ref().and_then(|f| f.forge.clone()));
        let project_name = h.project_name.or(h.project).or_else(|| repo_name.clone());

        let (year, month, day) = match (h.year, h.month, h.day) {
            (Some(y), Some(m), Some(d)) => (y, m, d),
            _ => parse_date_components(&h.date),
        };

        Ok(ReleaseContext {
            version: h.version,
            previous_version: h.previous_version,
            tag: h.tag,
            previous_tag: h.previous_tag,
            date: h.date,
            compare_url: h.compare_url,
            repository: h.repository,
            features: h.features,
            fixes: h.fixes,
            breaking: h.breaking,
            perf: h.perf,
            refactor: h.refactor,
            docs: h.docs,
            maintenance: h.maintenance,
            other: h.other,
            all_changes: h.all_changes,
            commits: h.commits,
            contributors: h.contributors,
            major,
            minor,
            patch,
            is_prerelease,
            prerelease,
            build,
            repo_owner,
            repo_name,
            forge,
            project_name,
            year,
            month,
            day,
        })
    }
}

pub(crate) fn parse_date_components(date_str: &str) -> (u32, u32, u32) {
    let mut parts = date_str.split('-');
    let y = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    let m = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    let d = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    (y, m, d)
}

impl ReleaseContext {
    /// Builds a `ChangelogRenderRequest` DTO from this `ReleaseContext` and the workspace root directory.
    pub fn to_changelog_render_request(
        &self,
        root_dir: impl Into<String>,
    ) -> crate::plugin::dto::ChangelogRenderRequest {
        let commits = self
            .commits
            .iter()
            .map(|c| crate::plugin::dto::PluginCommitEntry {
                sha: c.hash.clone().unwrap_or_default(),
                message: if !c.description.is_empty() {
                    c.description.clone()
                } else {
                    c.line.clone()
                },
                r#type: if !c.commit_type.is_empty() {
                    Some(c.commit_type.clone())
                } else {
                    None
                },
                scope: c.scope.clone(),
                author_name: c.author.clone(),
                pr_number: c.pr_number.map(|n| n.to_string()),
                is_breaking: c.is_breaking,
            })
            .collect();

        crate::plugin::dto::ChangelogRenderRequest {
            root_dir: root_dir.into(),
            version: self.version.clone(),
            tag_name: self.tag.clone(),
            previous_tag: self.previous_tag.clone(),
            release_date: self.date.clone(),
            commits,
        }
    }
}
