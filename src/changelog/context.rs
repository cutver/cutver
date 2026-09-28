use crate::conventional::ConventionalCommit;

/// Extracted repository forge metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeMetadata {
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub forge: Option<String>,
}

/// Parse owner, repo, and forge type from a Git repository URL (HTTPS or SSH).
pub fn parse_repo_forge(url: &str) -> ForgeMetadata {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return ForgeMetadata {
            owner: None,
            repo: None,
            forge: None,
        };
    }

    // Determine host and path
    let (host, path) = if let Some(stripped) = trimmed.strip_prefix("git@") {
        if let Some((h, p)) = stripped.split_once(':') {
            (h, p)
        } else {
            ("", stripped)
        }
    } else if let Some(stripped) = trimmed.strip_prefix("ssh://git@") {
        if let Some((h, p)) = stripped.split_once('/') {
            (h, p)
        } else {
            ("", stripped)
        }
    } else if let Some(stripped) = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
    {
        if let Some((h, p)) = stripped.split_once('/') {
            (h, p)
        } else {
            (stripped, "")
        }
    } else {
        ("", trimmed)
    };

    let forge = if host.contains("github.com") {
        Some("github".to_string())
    } else if host.contains("gitlab.com") {
        Some("gitlab".to_string())
    } else if host.contains("bitbucket.org") {
        Some("bitbucket".to_string())
    } else if host.contains("codeberg.org") {
        Some("codeberg".to_string())
    } else if host.contains("sourcehut.org") || host.contains("sr.ht") {
        Some("sourcehut".to_string())
    } else if !host.is_empty() {
        let h_lower = host.to_ascii_lowercase();
        if let Some(pos) = h_lower.find('.') {
            Some(h_lower[..pos].to_string())
        } else {
            Some(h_lower)
        }
    } else {
        None
    };

    // Clean path: strip .git suffix and leading/trailing slashes
    let clean_path = path
        .trim_matches('/')
        .strip_suffix(".git")
        .unwrap_or(path.trim_matches('/'));
    let segments: Vec<&str> = clean_path.split('/').filter(|s| !s.is_empty()).collect();

    let (owner, repo) = match segments.len() {
        0 => (None, None),
        1 => (None, Some(segments[0].to_string())),
        _ => {
            // For nested groups (e.g. gitlab subgroups org/subgroup/repo),
            // the last segment is the repo name, and the preceding segments form the owner
            let repo_name = segments[segments.len() - 1].to_string();
            let owner_name = segments[..segments.len() - 1].join("/");
            (Some(owner_name), Some(repo_name))
        }
    };

    ForgeMetadata { owner, repo, forge }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitContext {
    pub commit_type: String,
    pub scope: Option<String>,
    pub description: String,
    pub clean_description: String,
    pub is_breaking: bool,
    pub hash: Option<String>,
    pub short_hash: Option<String>,
    pub author: Option<String>,
    pub author_email: Option<String>,
    pub pr_number: Option<u64>,
    pub pr_url: Option<String>,
    pub issue_numbers: Vec<u64>,
    pub commit_url: Option<String>,
    pub line: String,
    pub bullet: String,
}

impl serde::Serialize for CommitContext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(16))?;
        map.serialize_entry("type", &self.commit_type)?;
        map.serialize_entry("commit_type", &self.commit_type)?;
        map.serialize_entry("scope", &self.scope)?;
        map.serialize_entry("description", &self.description)?;
        map.serialize_entry("clean_description", &self.clean_description)?;
        map.serialize_entry("is_breaking", &self.is_breaking)?;
        map.serialize_entry("hash", &self.hash)?;
        map.serialize_entry("short_hash", &self.short_hash)?;
        map.serialize_entry("author", &self.author)?;
        map.serialize_entry("author_email", &self.author_email)?;
        map.serialize_entry("pr_number", &self.pr_number)?;
        map.serialize_entry("pr_url", &self.pr_url)?;
        map.serialize_entry("issue_numbers", &self.issue_numbers)?;
        map.serialize_entry("commit_url", &self.commit_url)?;
        map.serialize_entry("line", &self.line)?;
        map.serialize_entry("bullet", &self.bullet)?;
        map.end()
    }
}

impl<'de> serde::Deserialize<'de> for CommitContext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Helper {
            #[serde(alias = "commit_type")]
            r#type: String,
            scope: Option<String>,
            description: String,
            #[serde(default)]
            clean_description: Option<String>,
            #[serde(default)]
            is_breaking: bool,
            #[serde(default)]
            hash: Option<String>,
            #[serde(default)]
            short_hash: Option<String>,
            #[serde(default)]
            author: Option<String>,
            #[serde(default)]
            author_email: Option<String>,
            #[serde(default)]
            pr_number: Option<u64>,
            #[serde(default)]
            pr_url: Option<String>,
            #[serde(default)]
            issue_numbers: Vec<u64>,
            #[serde(default)]
            commit_url: Option<String>,
            #[serde(default)]
            line: Option<String>,
            #[serde(default)]
            bullet: Option<String>,
        }
        let h = Helper::deserialize(deserializer)?;
        let clean = h
            .clean_description
            .unwrap_or_else(|| strip_trailing_pr_number(&h.description));
        let mut ctx = CommitContext {
            commit_type: h.r#type,
            scope: h.scope,
            description: h.description,
            clean_description: clean,
            is_breaking: h.is_breaking,
            hash: h.hash,
            short_hash: h.short_hash,
            author: h.author,
            author_email: h.author_email,
            pr_number: h.pr_number,
            pr_url: h.pr_url,
            issue_numbers: h.issue_numbers,
            commit_url: h.commit_url,
            line: String::new(),
            bullet: String::new(),
        };
        ctx.line = h.line.unwrap_or_else(|| format_commit_line(&ctx, true));
        ctx.bullet = h.bullet.unwrap_or_else(|| format!("- {}", ctx.line));
        Ok(ctx)
    }
}

pub fn strip_trailing_pr_number(description: &str) -> String {
    let trimmed = description.trim();
    if let Some(open_paren) = trimmed.rfind("(#")
        && trimmed.ends_with(')')
    {
        let num_str = &trimmed[open_paren + 2..trimmed.len() - 1];
        if num_str.chars().all(|c| c.is_ascii_digit()) && !num_str.is_empty() {
            return trimmed[..open_paren].trim_end().to_string();
        }
    }
    trimmed.to_string()
}

fn extract_pr_number(description: &str, body: Option<&str>) -> Option<u64> {
    // 1. Check end of description for (#123)
    let desc_trimmed = description.trim();
    if let Some(open_paren) = desc_trimmed.rfind("(#")
        && desc_trimmed.ends_with(')')
    {
        let num_str = &desc_trimmed[open_paren + 2..desc_trimmed.len() - 1];
        if let Ok(num) = num_str.parse::<u64>() {
            return Some(num);
        }
    }

    // 2. Check for "Merge pull request #123"
    let check_pr = |s: &str| -> Option<u64> {
        let lower = s.to_ascii_lowercase();
        let target = "merge pull request #";
        if let Some(idx) = lower.find(target) {
            let rest = &s[idx + target.len()..];
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(num) = num_str.parse::<u64>() {
                return Some(num);
            }
        }
        let target2 = "pull request #";
        if let Some(idx) = lower.find(target2) {
            let rest = &s[idx + target2.len()..];
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(num) = num_str.parse::<u64>() {
                return Some(num);
            }
        }
        None
    };

    if let Some(pr) = check_pr(description) {
        return Some(pr);
    }
    if let Some(b) = body
        && let Some(pr) = check_pr(b)
    {
        return Some(pr);
    }

    None
}

fn extract_issue_numbers(description: &str, body: Option<&str>, footers: &[(String, String)]) -> Vec<u64> {
    let mut issues = Vec::new();
    let mut keywords = [
        "fixes #",
        "fix #",
        "fixed #",
        "closes #",
        "close #",
        "closed #",
        "resolves #",
        "resolve #",
        "resolved #",
        "refs #",
        "ref #",
    ];
    keywords.sort_by_key(|b| std::cmp::Reverse(b.len()));

    let mut scan_text = |text: &str| {
        let lower = text.to_ascii_lowercase();
        for kw in &keywords {
            let mut search_idx = 0;
            while let Some(pos) = lower[search_idx..].find(kw) {
                let abs_pos = search_idx + pos;
                let rest = &text[abs_pos + kw.len()..];
                let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(num) = num_str.parse::<u64>()
                    && !issues.contains(&num)
                {
                    issues.push(num);
                }
                search_idx = abs_pos + kw.len();
            }
        }
    };

    scan_text(description);
    if let Some(b) = body {
        scan_text(b);
    }
    for (k, v) in footers {
        let combined = format!("{k}: {v}");
        scan_text(&combined);
        let space_combined = format!("{k} {v}");
        scan_text(&space_combined);
    }

    issues
}

fn build_urls(repo_url: Option<&str>, pr_number: Option<u64>, hash: Option<&str>) -> (Option<String>, Option<String>) {
    let repo = match repo_url {
        Some(r) => r.trim_end_matches('/'),
        None => return (None, None),
    };

    let is_gitlab = repo.contains("gitlab.com") || repo.contains("/-/");

    let pr_url = pr_number.map(|num| {
        if is_gitlab {
            format!("{repo}/-/merge_requests/{num}")
        } else {
            format!("{repo}/pull/{num}")
        }
    });

    let commit_url = hash.map(|h| {
        if is_gitlab {
            format!("{repo}/-/commit/{h}")
        } else {
            format!("{repo}/commit/{h}")
        }
    });

    (pr_url, commit_url)
}

/// Formats a single commit into a canonical, rich Markdown line.
/// Output format: `**scope**: clean description in [#123](url) ([abc1234](url)) by @author`
pub fn format_commit_line(ctx: &CommitContext, include_scope: bool) -> String {
    let mut line = String::new();
    if include_scope && let Some(scope) = &ctx.scope {
        line.push_str(&format!("**{scope}**: "));
    }

    let desc = if ctx.clean_description.is_empty() {
        &ctx.description
    } else {
        &ctx.clean_description
    };
    line.push_str(desc);

    if let Some(pr) = ctx.pr_number {
        match &ctx.pr_url {
            Some(url) => line.push_str(&format!(" in [#{pr}]({url})")),
            None => line.push_str(&format!(" in #{pr}")),
        }
    }

    if let Some(hash) = &ctx.short_hash {
        match &ctx.commit_url {
            Some(url) => line.push_str(&format!(" ([{hash}]({url}))")),
            None => line.push_str(&format!(" ({hash})")),
        }
    }

    if let Some(author) = &ctx.author
        && !author.is_empty()
    {
        line.push_str(&format!(" by @{author}"));
    }

    line
}

pub fn enrich_commit_context(
    mut ctx: CommitContext,
    raw_commit: Option<&crate::git::RawCommit>,
    conventional: Option<&ConventionalCommit>,
    repo_url: Option<&str>,
) -> CommitContext {
    if let Some(raw) = raw_commit {
        ctx.hash = Some(raw.hash.clone());
        ctx.short_hash = Some(raw.short_hash.clone());
        ctx.author = Some(crate::git::resolve_author(&raw.author_name, &raw.author_email));
        ctx.author_email = Some(raw.author_email.clone());
    }

    let (body, footers) = match conventional {
        Some(c) => (c.body.as_deref(), c.footers.as_slice()),
        None => (None, &[][..]),
    };

    ctx.pr_number = extract_pr_number(&ctx.description, body);
    ctx.issue_numbers = extract_issue_numbers(&ctx.description, body, footers);

    let (pr_url, commit_url) = build_urls(repo_url, ctx.pr_number, ctx.hash.as_deref());
    ctx.pr_url = pr_url;
    ctx.commit_url = commit_url;
    ctx.line = format_commit_line(&ctx, true);
    ctx.bullet = format!("- {}", ctx.line);

    ctx
}

impl From<&ConventionalCommit> for CommitContext {
    fn from(c: &ConventionalCommit) -> Self {
        let desc = c.description.trim().to_string();
        let clean = strip_trailing_pr_number(&desc);
        let mut ctx = Self {
            commit_type: c.commit_type.clone(),
            scope: c.scope.clone(),
            description: desc,
            clean_description: clean,
            is_breaking: c.is_breaking,
            hash: None,
            short_hash: None,
            author: None,
            author_email: None,
            pr_number: None,
            pr_url: None,
            issue_numbers: Vec::new(),
            commit_url: None,
            line: String::new(),
            bullet: String::new(),
        };
        ctx.line = format_commit_line(&ctx, true);
        ctx.bullet = format!("- {}", ctx.line);
        ctx
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    // Enriched SemVer breakdown
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub is_prerelease: bool,
    pub prerelease: Option<String>,
    pub build: Option<String>,
    // Enriched repository forge metadata
    pub repo_owner: Option<String>,
    pub repo_name: Option<String>,
    pub forge: Option<String>,
    pub project_name: Option<String>,
}

impl serde::Serialize for ReleaseContext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(31))?;
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

/// Simplified context for universal string interpolation in `commit_message`, `post_bump`, and `publish.commands`.
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
        let parsed_semver = semver::Version::parse(&h.version).ok();
        let major = h.major.or_else(|| parsed_semver.as_ref().map(|v| v.major)).unwrap_or(0);
        let minor = h.minor.or_else(|| parsed_semver.as_ref().map(|v| v.minor)).unwrap_or(0);
        let patch = h.patch.or_else(|| parsed_semver.as_ref().map(|v| v.patch)).unwrap_or(0);
        let is_prerelease = h
            .is_prerelease
            .or_else(|| parsed_semver.as_ref().map(|v| !v.pre.is_empty()))
            .unwrap_or(false);
        let prerelease = h.prerelease.or_else(|| {
            parsed_semver.as_ref().and_then(|v| {
                if v.pre.is_empty() {
                    None
                } else {
                    Some(v.pre.as_str().to_string())
                }
            })
        });
        let build = h.build.or_else(|| {
            parsed_semver.as_ref().and_then(|v| {
                if v.build.is_empty() {
                    None
                } else {
                    Some(v.build.as_str().to_string())
                }
            })
        });

        let project_name = h.project_name.or_else(|| h.repo_name.clone());

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
            repo_owner: h.repo_owner,
            repo_name: h.repo_name,
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
        let parsed_semver = semver::Version::parse(version).ok();
        let major = parsed_semver.as_ref().map(|v| v.major).unwrap_or(0);
        let minor = parsed_semver.as_ref().map(|v| v.minor).unwrap_or(0);
        let patch = parsed_semver.as_ref().map(|v| v.patch).unwrap_or(0);
        let is_prerelease = parsed_semver.as_ref().map(|v| !v.pre.is_empty()).unwrap_or(false);
        let prerelease = parsed_semver.as_ref().and_then(|v| {
            if v.pre.is_empty() {
                None
            } else {
                Some(v.pre.as_str().to_string())
            }
        });
        let build = parsed_semver.as_ref().and_then(|v| {
            if v.build.is_empty() {
                None
            } else {
                Some(v.build.as_str().to_string())
            }
        });

        let forge_meta = repository.map(parse_repo_forge);
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

        // If SemVer fields were not explicitly supplied, attempt to parse from version
        let parsed_semver = semver::Version::parse(&h.version).ok();
        let major = h
            .major
            .unwrap_or_else(|| parsed_semver.as_ref().map(|v| v.major).unwrap_or(0));
        let minor = h
            .minor
            .unwrap_or_else(|| parsed_semver.as_ref().map(|v| v.minor).unwrap_or(0));
        let patch = h
            .patch
            .unwrap_or_else(|| parsed_semver.as_ref().map(|v| v.patch).unwrap_or(0));
        let is_prerelease = h
            .is_prerelease
            .unwrap_or_else(|| parsed_semver.as_ref().map(|v| !v.pre.is_empty()).unwrap_or(false));
        let prerelease = h.prerelease.or_else(|| {
            parsed_semver.as_ref().and_then(|v| {
                if v.pre.is_empty() {
                    None
                } else {
                    Some(v.pre.as_str().to_string())
                }
            })
        });
        let build = h.build.or_else(|| {
            parsed_semver.as_ref().and_then(|v| {
                if v.build.is_empty() {
                    None
                } else {
                    Some(v.build.as_str().to_string())
                }
            })
        });

        // If repo forge fields were not explicitly supplied, attempt to parse from repository
        let parsed_forge = h.repository.as_deref().map(parse_repo_forge);
        let repo_owner = h
            .repo_owner
            .or_else(|| parsed_forge.as_ref().and_then(|f| f.owner.clone()));
        let repo_name = h
            .repo_name
            .or_else(|| parsed_forge.as_ref().and_then(|f| f.repo.clone()));
        let forge = h.forge.or_else(|| parsed_forge.as_ref().and_then(|f| f.forge.clone()));
        let project_name = h.project_name.or_else(|| repo_name.clone());

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
        })
    }
}

pub fn filter_commits(
    commits: &[ConventionalCommit],
    ignore_release_commits: bool,
    ignore_scopes: &[String],
) -> Vec<ConventionalCommit> {
    commits
        .iter()
        .filter(|c| {
            if ignore_release_commits {
                // Exclude commits where commit_type == "chore" and scope.as_deref() == Some("release")
                // Exclude commits where description.trim().starts_with("release:") or
                // description.trim().starts_with("v") with version numbers, or scope == Some("release")
                if c.commit_type.eq_ignore_ascii_case("chore") && c.scope.as_deref() == Some("release") {
                    return false;
                }

                if let Some(scope) = &c.scope
                    && scope.eq_ignore_ascii_case("release")
                {
                    return false;
                }

                let desc = c.description.trim();
                let lower_desc = desc.to_ascii_lowercase();
                if lower_desc.starts_with("release:") || lower_desc.starts_with("release ") {
                    return false;
                }

                // Check for "v" followed by version numbers e.g. "v1", "v0.1.0", "v1.2.3"
                if let Some(rest) = desc.strip_prefix(['v', 'V']) {
                    let rest = rest.trim_start();
                    if rest.starts_with(|ch: char| ch.is_ascii_digit()) {
                        return false;
                    }
                }
            }

            if !ignore_scopes.is_empty()
                && let Some(scope) = &c.scope
                && ignore_scopes.iter().any(|s| s.eq_ignore_ascii_case(scope))
            {
                return false;
            }

            true
        })
        .cloned()
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn build_context_with_raw_and_filter(
    version: &str,
    previous_version: Option<&str>,
    tag: &str,
    previous_tag: Option<&str>,
    date: &str,
    repository: Option<String>,
    commits: &[ConventionalCommit],
    raw_commits: Option<&[crate::git::RawCommit]>,
    contributors: Vec<String>,
    include_scopes: bool,
    fallback_entry: &str,
    ignore_release_commits: bool,
    ignore_scopes: &[String],
) -> ReleaseContext {
    let filtered_commits = filter_commits(commits, ignore_release_commits, ignore_scopes);
    let commits = &filtered_commits;

    struct Category {
        header: &'static str,
        items: Vec<String>,
    }

    let mut categories = [
        Category {
            header: "### ⚠️ Breaking Changes",
            items: Vec::new(),
        },
        Category {
            header: "### Features",
            items: Vec::new(),
        },
        Category {
            header: "### Bug Fixes",
            items: Vec::new(),
        },
        Category {
            header: "### Performance Improvements",
            items: Vec::new(),
        },
        Category {
            header: "### Refactoring",
            items: Vec::new(),
        },
        Category {
            header: "### Documentation",
            items: Vec::new(),
        },
        Category {
            header: "### Maintenance",
            items: Vec::new(),
        },
        Category {
            header: "### Other Changes",
            items: Vec::new(),
        },
    ];

    let mut commit_contexts = Vec::with_capacity(commits.len());
    let mut raw_iter = raw_commits.map(|raws| raws.iter());

    for c in commits {
        // Advance linearly in order through raw commits. This ensures O(N) complexity
        // across the entire commit history and guarantees that duplicate commits with
        // identical messages correctly match their distinct sequential Git commits.
        let matching_raw = if let Some(ref mut iter) = raw_iter {
            iter.find(|r| {
                if let Some(parsed) = ConventionalCommit::parse(&r.message) {
                    parsed == *c
                } else {
                    r.message.starts_with(&c.description)
                        || r.message.lines().next().is_some_and(|l| l.contains(&c.description))
                }
            })
        } else {
            None
        };

        let enriched = enrich_commit_context(CommitContext::from(c), matching_raw, Some(c), repository.as_deref());
        let item = format!("- {}", format_commit_line(&enriched, include_scopes));
        commit_contexts.push(enriched);

        if c.is_breaking {
            categories[0].items.push(item);
        } else {
            match c.commit_type.as_str() {
                "feat" => categories[1].items.push(item),
                "fix" => categories[2].items.push(item),
                "perf" => categories[3].items.push(item),
                "refactor" => categories[4].items.push(item),
                "docs" => categories[5].items.push(item),
                "chore" | "build" | "ci" | "test" => categories[6].items.push(item),
                _ => categories[7].items.push(item),
            }
        }
    }

    let breaking = categories[0].items.join("\n");
    let features = categories[1].items.join("\n");
    let fixes = categories[2].items.join("\n");
    let perf = categories[3].items.join("\n");
    let refactor = categories[4].items.join("\n");
    let docs = categories[5].items.join("\n");
    let maintenance = categories[6].items.join("\n");
    let other = categories[7].items.join("\n");

    let sections: Vec<String> = categories
        .iter()
        .filter(|cat| !cat.items.is_empty())
        .map(|cat| format!("{}\n{}", cat.header, cat.items.join("\n")))
        .collect();

    let all_changes = if sections.is_empty() {
        format!("- {}", fallback_entry.trim())
    } else {
        sections.join("\n\n")
    };

    let compare_url = match (&repository, previous_tag) {
        (Some(repo), Some(prev)) => Some(format!("{repo}/compare/{prev}...{tag}")),
        _ => None,
    };

    let parsed_semver = semver::Version::parse(version).ok();
    let major = parsed_semver.as_ref().map(|v| v.major).unwrap_or(0);
    let minor = parsed_semver.as_ref().map(|v| v.minor).unwrap_or(0);
    let patch = parsed_semver.as_ref().map(|v| v.patch).unwrap_or(0);
    let is_prerelease = parsed_semver.as_ref().map(|v| !v.pre.is_empty()).unwrap_or(false);
    let prerelease = parsed_semver.as_ref().and_then(|v| {
        if v.pre.is_empty() {
            None
        } else {
            Some(v.pre.as_str().to_string())
        }
    });
    let build = parsed_semver.as_ref().and_then(|v| {
        if v.build.is_empty() {
            None
        } else {
            Some(v.build.as_str().to_string())
        }
    });

    let forge_meta = repository.as_deref().map(parse_repo_forge);
    let repo_owner = forge_meta.as_ref().and_then(|f| f.owner.clone());
    let repo_name = forge_meta.as_ref().and_then(|f| f.repo.clone());
    let forge = forge_meta.as_ref().and_then(|f| f.forge.clone());
    let project_name = repo_name.clone();

    ReleaseContext {
        version: version.to_string(),
        previous_version: previous_version.map(String::from),
        tag: tag.to_string(),
        previous_tag: previous_tag.map(String::from),
        date: date.to_string(),
        compare_url,
        repository,
        features,
        fixes,
        breaking,
        perf,
        refactor,
        docs,
        maintenance,
        other,
        all_changes,
        commits: commit_contexts,
        contributors,
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
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_context_with_filter(
    version: &str,
    previous_version: Option<&str>,
    tag: &str,
    previous_tag: Option<&str>,
    date: &str,
    repository: Option<String>,
    commits: &[ConventionalCommit],
    contributors: Vec<String>,
    include_scopes: bool,
    fallback_entry: &str,
    ignore_release_commits: bool,
    ignore_scopes: &[String],
) -> ReleaseContext {
    build_context_with_raw_and_filter(
        version,
        previous_version,
        tag,
        previous_tag,
        date,
        repository,
        commits,
        None,
        contributors,
        include_scopes,
        fallback_entry,
        ignore_release_commits,
        ignore_scopes,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_context_with_raw(
    version: &str,
    previous_version: Option<&str>,
    tag: &str,
    previous_tag: Option<&str>,
    date: &str,
    repository: Option<String>,
    commits: &[ConventionalCommit],
    raw_commits: Option<&[crate::git::RawCommit]>,
    contributors: Vec<String>,
    include_scopes: bool,
    fallback_entry: &str,
) -> ReleaseContext {
    build_context_with_raw_and_filter(
        version,
        previous_version,
        tag,
        previous_tag,
        date,
        repository,
        commits,
        raw_commits,
        contributors,
        include_scopes,
        fallback_entry,
        true,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_context(
    version: &str,
    previous_version: Option<&str>,
    tag: &str,
    previous_tag: Option<&str>,
    date: &str,
    repository: Option<String>,
    commits: &[ConventionalCommit],
    contributors: Vec<String>,
    include_scopes: bool,
    fallback_entry: &str,
) -> ReleaseContext {
    build_context_with_filter(
        version,
        previous_version,
        tag,
        previous_tag,
        date,
        repository,
        commits,
        contributors,
        include_scopes,
        fallback_entry,
        true,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_context_auto(
    version: &str,
    tag_prefix: &str,
    previous_tag: Option<&str>,
    date: Option<&str>,
    repository: Option<String>,
    commits: &[ConventionalCommit],
    contributors: Vec<String>,
    include_scopes: bool,
    fallback_entry: &str,
    ignore_release_commits: bool,
    ignore_scopes: &[String],
) -> ReleaseContext {
    let clean_version = version.strip_prefix(tag_prefix).unwrap_or(version);
    let tag = if version.starts_with(tag_prefix) {
        version.to_string()
    } else {
        format!("{tag_prefix}{version}")
    };
    let previous_version = previous_tag.map(|pt| pt.strip_prefix(tag_prefix).unwrap_or(pt));
    let today = crate::changelog::format_date(std::time::SystemTime::now());
    let date_str = date.unwrap_or(&today);

    build_context_with_filter(
        clean_version,
        previous_version,
        &tag,
        previous_tag,
        date_str,
        repository,
        commits,
        contributors,
        include_scopes,
        fallback_entry,
        ignore_release_commits,
        ignore_scopes,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_commits_release_commits() {
        let commits = vec![
            ConventionalCommit::parse("chore(release): v1.0.0").unwrap(),
            ConventionalCommit::parse("chore(deps): update foo").unwrap(),
            ConventionalCommit::parse("chore: clean up").unwrap(),
            ConventionalCommit::parse("chore: release: 1.0.0").unwrap(),
            ConventionalCommit::parse("fix: v2.0.0 bug").unwrap(), // wait, description is "v2.0.0 bug" -> starts with 'v' and digit
            ConventionalCommit::parse("feat: add feature").unwrap(),
        ];

        // When ignore_release_commits is true
        let filtered = filter_commits(&commits, true, &[]);
        let descriptions: Vec<&str> = filtered.iter().map(|c| c.description.trim()).collect();
        assert_eq!(descriptions, vec!["update foo", "clean up", "add feature"]);

        // When ignore_release_commits is false
        let unfiltered = filter_commits(&commits, false, &[]);
        assert_eq!(unfiltered.len(), commits.len());
    }

    #[test]
    fn test_filter_commits_ignore_scopes() {
        let commits = vec![
            ConventionalCommit::parse("feat(cli): new flag").unwrap(),
            ConventionalCommit::parse("fix(internal): hide debug output").unwrap(),
            ConventionalCommit::parse("docs(WIP): draft docs").unwrap(),
            ConventionalCommit::parse("chore: regular chore").unwrap(),
        ];

        let ignore = vec!["internal".to_string(), "wip".to_string()];
        let filtered = filter_commits(&commits, false, &ignore);
        let descriptions: Vec<&str> = filtered.iter().map(|c| c.description.trim()).collect();
        assert_eq!(descriptions, vec!["new flag", "regular chore"]);
    }

    #[test]
    fn test_format_commit_line_and_bullet() {
        let mut ctx = CommitContext {
            commit_type: "feat".to_string(),
            scope: Some("cli".to_string()),
            description: "add flag (#42)".to_string(),
            clean_description: "add flag".to_string(),
            is_breaking: false,
            hash: Some("1234567890abcdef".to_string()),
            short_hash: Some("1234567".to_string()),
            author: Some("Alice".to_string()),
            author_email: Some("alice@example.com".to_string()),
            pr_number: Some(42),
            pr_url: Some("https://github.com/org/repo/pull/42".to_string()),
            issue_numbers: vec![],
            commit_url: Some("https://github.com/org/repo/commit/1234567890abcdef".to_string()),
            line: String::new(),
            bullet: String::new(),
        };

        let formatted = format_commit_line(&ctx, true);
        assert_eq!(
            formatted,
            "**cli**: add flag in [#42](https://github.com/org/repo/pull/42) ([1234567](https://github.com/org/repo/commit/1234567890abcdef)) by @Alice"
        );
        let no_scope = format_commit_line(&ctx, false);
        assert_eq!(
            no_scope,
            "add flag in [#42](https://github.com/org/repo/pull/42) ([1234567](https://github.com/org/repo/commit/1234567890abcdef)) by @Alice"
        );

        ctx.line = formatted.clone();
        ctx.bullet = format!("- {formatted}");
        assert_eq!(ctx.bullet, format!("- {formatted}"));

        // Serialization & deserialization check
        let json = serde_json::to_string(&ctx).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["line"], formatted);
        assert_eq!(value["bullet"], format!("- {formatted}"));

        // Deserialization check (Helper has type with alias commit_type)
        let json_input = serde_json::json!({
            "type": "feat",
            "scope": "cli",
            "description": "add flag (#42)",
            "line": formatted,
            "bullet": format!("- {formatted}")
        });
        let deserialized: CommitContext = serde_json::from_value(json_input).unwrap();
        assert_eq!(deserialized.line, formatted);
        assert_eq!(deserialized.bullet, format!("- {formatted}"));
    }

    #[test]
    fn test_format_commit_line_without_urls() {
        let ctx = CommitContext {
            commit_type: "fix".to_string(),
            scope: None,
            description: "resolve bug".to_string(),
            clean_description: "resolve bug".to_string(),
            is_breaking: false,
            hash: None,
            short_hash: Some("abc1234".to_string()),
            author: Some("Bob".to_string()),
            author_email: None,
            pr_number: Some(10),
            pr_url: None,
            issue_numbers: vec![],
            commit_url: None,
            line: String::new(),
            bullet: String::new(),
        };
        let formatted = format_commit_line(&ctx, true);
        assert_eq!(formatted, "resolve bug in #10 (abc1234) by @Bob");
    }

    #[test]
    fn test_build_context_full() {
        let commits = vec![
            ConventionalCommit::parse("feat(cli): add template flag").unwrap(),
            ConventionalCommit::parse("fix: small fix").unwrap(),
            ConventionalCommit::parse("feat!: breaking api change").unwrap(),
            ConventionalCommit::parse("chore(release): v1.2.0").unwrap(),
        ];
        let contributors = vec!["Alice".to_string(), "Bob".to_string()];
        let ctx = build_context_with_filter(
            "1.2.0",
            Some("1.1.0"),
            "v1.2.0",
            Some("v1.1.0"),
            "2026-03-30",
            Some("https://github.com/Row0902/cutver".to_string()),
            &commits,
            contributors,
            true,
            "Maintenance and updates.",
            true,
            &[],
        );

        assert_eq!(ctx.version, "1.2.0");
        assert_eq!(ctx.previous_version.as_deref(), Some("1.1.0"));
        assert_eq!(ctx.tag, "v1.2.0");
        assert_eq!(ctx.previous_tag.as_deref(), Some("v1.1.0"));
        assert_eq!(ctx.date, "2026-03-30");
        assert_eq!(
            ctx.compare_url.as_deref(),
            Some("https://github.com/Row0902/cutver/compare/v1.1.0...v1.2.0")
        );
        assert_eq!(ctx.features, "- **cli**: add template flag");
        assert_eq!(ctx.fixes, "- small fix");
        assert_eq!(ctx.breaking, "- breaking api change");
        assert_eq!(ctx.maintenance, "");
        assert_eq!(ctx.commits.len(), 3);
        assert_eq!(ctx.commits[0].commit_type, "feat");
        assert_eq!(ctx.commits[0].scope.as_deref(), Some("cli"));
        assert_eq!(ctx.contributors, vec!["Alice", "Bob"]);
        assert!(ctx.all_changes.contains("### ⚠️ Breaking Changes"));
        assert!(ctx.all_changes.contains("### Features"));
        assert!(ctx.all_changes.contains("### Bug Fixes"));
        assert!(!ctx.all_changes.contains("Maintenance"));
    }

    #[test]
    fn test_build_context_empty_commits() {
        let ctx = build_context_with_filter(
            "0.1.0",
            None,
            "v0.1.0",
            None,
            "2026-01-01",
            None,
            &[],
            vec![],
            true,
            "Initial release.",
            true,
            &[],
        );

        assert_eq!(ctx.compare_url, None);
        assert_eq!(ctx.all_changes, "- Initial release.");
        assert!(ctx.features.is_empty());
        assert!(ctx.fixes.is_empty());
        assert!(ctx.commits.is_empty());
        assert!(ctx.contributors.is_empty());
    }

    #[test]
    fn test_enrich_commit_context_metadata() {
        let raw = crate::git::RawCommit {
            hash: "1234567890abcdef1234567890abcdef12345678".to_string(),
            short_hash: "1234567".to_string(),
            author_name: "Alice Smith".to_string(),
            author_email: "alice@example.com".to_string(),
            message: "feat: add super feature (#42)\n\nCloses #10\nFixes #11".to_string(),
        };
        let c = ConventionalCommit::parse(&raw.message).unwrap();
        let base_ctx = CommitContext::from(&c);
        let enriched = enrich_commit_context(base_ctx, Some(&raw), Some(&c), Some("https://github.com/org/repo"));

        assert_eq!(
            enriched.hash.as_deref(),
            Some("1234567890abcdef1234567890abcdef12345678")
        );
        assert_eq!(enriched.short_hash.as_deref(), Some("1234567"));
        assert_eq!(enriched.author.as_deref(), Some("Alice Smith"));
        assert_eq!(enriched.author_email.as_deref(), Some("alice@example.com"));
        assert_eq!(enriched.pr_number, Some(42));
        assert_eq!(enriched.pr_url.as_deref(), Some("https://github.com/org/repo/pull/42"));
        assert_eq!(enriched.issue_numbers, vec![10, 11]);
        assert_eq!(
            enriched.commit_url.as_deref(),
            Some("https://github.com/org/repo/commit/1234567890abcdef1234567890abcdef12345678")
        );
    }

    #[test]
    fn test_enrich_commit_context_gitlab_url() {
        let raw = crate::git::RawCommit {
            hash: "abcdef1234567890abcdef1234567890abcdef12".to_string(),
            short_hash: "abcdef1".to_string(),
            author_name: "Bob".to_string(),
            author_email: "bob@example.com".to_string(),
            message: "fix: merge request fix\n\nMerge pull request #99 from branch".to_string(),
        };
        let c = ConventionalCommit::parse(&raw.message).unwrap();
        let base_ctx = CommitContext::from(&c);
        let enriched = enrich_commit_context(base_ctx, Some(&raw), Some(&c), Some("https://gitlab.com/group/project"));

        assert_eq!(enriched.pr_number, Some(99));
        assert_eq!(
            enriched.pr_url.as_deref(),
            Some("https://gitlab.com/group/project/-/merge_requests/99")
        );
        assert_eq!(
            enriched.commit_url.as_deref(),
            Some("https://gitlab.com/group/project/-/commit/abcdef1234567890abcdef1234567890abcdef12")
        );
    }

    #[test]
    fn test_linear_raw_commits_distinct_attribution_for_duplicates() {
        let raw1 = crate::git::RawCommit {
            hash: "1111111111111111111111111111111111111111".to_string(),
            short_hash: "1111111".to_string(),
            author_name: "Alice".to_string(),
            author_email: "alice@example.com".to_string(),
            message: "fix: duplicate fix message".to_string(),
        };
        let raw2 = crate::git::RawCommit {
            hash: "2222222222222222222222222222222222222222".to_string(),
            short_hash: "2222222".to_string(),
            author_name: "Bob".to_string(),
            author_email: "bob@example.com".to_string(),
            message: "fix: duplicate fix message".to_string(),
        };
        let c1 = ConventionalCommit::parse(&raw1.message).unwrap();
        let c2 = ConventionalCommit::parse(&raw2.message).unwrap();
        let commits = vec![c1, c2];
        let raws = vec![raw1, raw2];

        let ctx = build_context_with_raw_and_filter(
            "1.0.0",
            None,
            "v1.0.0",
            None,
            "2026-01-01",
            None,
            &commits,
            Some(&raws),
            vec![],
            false,
            "none",
            true,
            &[],
        );

        assert_eq!(ctx.commits.len(), 2);
        assert_eq!(
            ctx.commits[0].hash.as_deref(),
            Some("1111111111111111111111111111111111111111")
        );
        assert_eq!(ctx.commits[0].author.as_deref(), Some("Alice"));
        assert_eq!(
            ctx.commits[1].hash.as_deref(),
            Some("2222222222222222222222222222222222222222")
        );
        assert_eq!(ctx.commits[1].author.as_deref(), Some("Bob"));
    }

    #[test]
    fn test_clean_description_strips_trailing_pr_number() {
        assert_eq!(
            strip_trailing_pr_number("add exciting feature (#42)"),
            "add exciting feature"
        );
        assert_eq!(
            strip_trailing_pr_number("fix issue with (#12) in core (#99)"),
            "fix issue with (#12) in core"
        );
        assert_eq!(
            strip_trailing_pr_number("no pr reference in description"),
            "no pr reference in description"
        );
        assert_eq!(
            strip_trailing_pr_number("invalid pr suffix (#abc)"),
            "invalid pr suffix (#abc)"
        );

        let c = ConventionalCommit::parse("feat: add feature (#100)").unwrap();
        let ctx = CommitContext::from(&c);
        assert_eq!(ctx.description, "add feature (#100)");
        assert_eq!(ctx.clean_description, "add feature");
    }

    #[test]
    fn test_parse_repo_forge() {
        let meta = parse_repo_forge("https://github.com/Row0902/cutver.git");
        assert_eq!(meta.forge.as_deref(), Some("github"));
        assert_eq!(meta.owner.as_deref(), Some("Row0902"));
        assert_eq!(meta.repo.as_deref(), Some("cutver"));

        let meta = parse_repo_forge("git@github.com:Row0902/cutver.git");
        assert_eq!(meta.forge.as_deref(), Some("github"));
        assert_eq!(meta.owner.as_deref(), Some("Row0902"));
        assert_eq!(meta.repo.as_deref(), Some("cutver"));

        let meta = parse_repo_forge("ssh://git@gitlab.com/group/subgroup/project.git");
        assert_eq!(meta.forge.as_deref(), Some("gitlab"));
        assert_eq!(meta.owner.as_deref(), Some("group/subgroup"));
        assert_eq!(meta.repo.as_deref(), Some("project"));

        let meta = parse_repo_forge("https://bitbucket.org/team/repo");
        assert_eq!(meta.forge.as_deref(), Some("bitbucket"));
        assert_eq!(meta.owner.as_deref(), Some("team"));
        assert_eq!(meta.repo.as_deref(), Some("repo"));

        let meta = parse_repo_forge("https://codeberg.org/user/repo.git");
        assert_eq!(meta.forge.as_deref(), Some("codeberg"));
        assert_eq!(meta.owner.as_deref(), Some("user"));
        assert_eq!(meta.repo.as_deref(), Some("repo"));

        let meta = parse_repo_forge("https://git.sr.ht/~user/repo");
        assert_eq!(meta.forge.as_deref(), Some("sourcehut"));
        assert_eq!(meta.owner.as_deref(), Some("~user"));
        assert_eq!(meta.repo.as_deref(), Some("repo"));

        let meta = parse_repo_forge("");
        assert_eq!(meta.forge, None);
        assert_eq!(meta.owner, None);
        assert_eq!(meta.repo, None);
    }

    #[test]
    fn test_release_context_semver_breakdown() {
        let ctx = build_context_with_filter(
            "1.2.3-alpha.1+20230101",
            None,
            "v1.2.3-alpha.1+20230101",
            None,
            "2026-03-30",
            Some("https://github.com/org/repo".to_string()),
            &[],
            vec![],
            true,
            "entry",
            true,
            &[],
        );

        assert_eq!(ctx.major, 1);
        assert_eq!(ctx.minor, 2);
        assert_eq!(ctx.patch, 3);
        assert!(ctx.is_prerelease);
        assert_eq!(ctx.prerelease.as_deref(), Some("alpha.1"));
        assert_eq!(ctx.build.as_deref(), Some("20230101"));
        assert_eq!(ctx.repo_owner.as_deref(), Some("org"));
        assert_eq!(ctx.repo_name.as_deref(), Some("repo"));
        assert_eq!(ctx.forge.as_deref(), Some("github"));
    }

    #[test]
    fn test_release_context_non_semver_fallback() {
        let ctx = build_context_with_filter(
            "non-semver-string",
            None,
            "non-semver-string",
            None,
            "2026-03-30",
            None,
            &[],
            vec![],
            true,
            "entry",
            true,
            &[],
        );

        assert_eq!(ctx.major, 0);
        assert_eq!(ctx.minor, 0);
        assert_eq!(ctx.patch, 0);
        assert!(!ctx.is_prerelease);
        assert_eq!(ctx.prerelease, None);
        assert_eq!(ctx.build, None);
        assert_eq!(ctx.repo_owner, None);
        assert_eq!(ctx.repo_name, None);
        assert_eq!(ctx.forge, None);
    }

    #[test]
    fn test_release_context_serialization_aliases() {
        let ctx = build_context_with_filter(
            "1.0.0",
            None,
            "v1.0.0",
            None,
            "2026-03-30",
            Some("https://github.com/my-org/my-pkg".to_string()),
            &[],
            vec![],
            true,
            "entry",
            true,
            &[],
        );

        let json = serde_json::to_string(&ctx).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["owner"], "my-org");
        assert_eq!(value["repo_owner"], "my-org");
        assert_eq!(value["repo"], "my-pkg");
        assert_eq!(value["repo_name"], "my-pkg");
        assert_eq!(value["forge"], "github");
        assert_eq!(value["project"], "my-pkg");
        assert_eq!(value["project_name"], "my-pkg");

        // Test deserialization with owner/repo aliases
        let json_input = r#"{
            "version": "2.3.4",
            "tag": "v2.3.4",
            "date": "2026-03-30",
            "owner": "custom-owner",
            "repo": "custom-repo",
            "forge": "custom-forge"
        }"#;
        let deserialized: ReleaseContext = serde_json::from_str(json_input).unwrap();
        assert_eq!(deserialized.major, 2);
        assert_eq!(deserialized.minor, 3);
        assert_eq!(deserialized.patch, 4);
        assert_eq!(deserialized.repo_owner.as_deref(), Some("custom-owner"));
        assert_eq!(deserialized.repo_name.as_deref(), Some("custom-repo"));
        assert_eq!(deserialized.forge.as_deref(), Some("custom-forge"));
        assert_eq!(deserialized.project_name.as_deref(), Some("custom-repo"));

        // Test deserialization with explicit project alias
        let json_input_project = r#"{
            "version": "2.3.4",
            "tag": "v2.3.4",
            "date": "2026-03-30",
            "project": "custom-project"
        }"#;
        let deserialized: ReleaseContext = serde_json::from_str(json_input_project).unwrap();
        assert_eq!(deserialized.project_name.as_deref(), Some("custom-project"));
    }

    #[test]
    fn test_interpolation_context_build_and_serialization() {
        let ctx = InterpolationContext::build(
            "1.2.3-rc.1+build.123",
            Some("1.2.2"),
            "v1.2.3-rc.1+build.123",
            Some("v1.2.2"),
            "minor",
            "2026-03-30",
            Some("main"),
            Some("https://github.com/my-org/my-pkg.git"),
        );

        assert_eq!(ctx.version, "1.2.3-rc.1+build.123");
        assert_eq!(ctx.previous_version.as_deref(), Some("1.2.2"));
        assert_eq!(ctx.tag, "v1.2.3-rc.1+build.123");
        assert_eq!(ctx.previous_tag.as_deref(), Some("v1.2.2"));
        assert_eq!(ctx.bump_level, "minor");
        assert!(ctx.is_prerelease);
        assert_eq!(ctx.prerelease.as_deref(), Some("rc.1"));
        assert_eq!(ctx.build.as_deref(), Some("build.123"));
        assert_eq!(ctx.major, 1);
        assert_eq!(ctx.minor, 2);
        assert_eq!(ctx.patch, 3);
        assert_eq!(ctx.date, "2026-03-30");
        assert_eq!(ctx.branch.as_deref(), Some("main"));
        assert_eq!(ctx.repo_owner.as_deref(), Some("my-org"));
        assert_eq!(ctx.repo_name.as_deref(), Some("my-pkg"));
        assert_eq!(ctx.forge.as_deref(), Some("github"));
        assert_eq!(ctx.project_name.as_deref(), Some("my-pkg"));

        let json = serde_json::to_string(&ctx).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["owner"], "my-org");
        assert_eq!(value["repo_owner"], "my-org");
        assert_eq!(value["repo"], "my-pkg");
        assert_eq!(value["repo_name"], "my-pkg");
        assert_eq!(value["forge"], "github");
        assert_eq!(value["branch"], "main");
        assert_eq!(value["bump_level"], "minor");
        assert_eq!(value["is_prerelease"], true);
        assert_eq!(value["project"], "my-pkg");
        assert_eq!(value["project_name"], "my-pkg");

        let from_rc = InterpolationContext::from_release_context(
            &ReleaseContext {
                version: "1.0.0".into(),
                previous_version: None,
                tag: "v1.0.0".into(),
                previous_tag: None,
                date: "2026-03-30".into(),
                compare_url: None,
                repository: None,
                features: String::new(),
                fixes: String::new(),
                breaking: String::new(),
                perf: String::new(),
                refactor: String::new(),
                docs: String::new(),
                maintenance: String::new(),
                other: String::new(),
                all_changes: String::new(),
                commits: Vec::new(),
                contributors: Vec::new(),
                major: 1,
                minor: 0,
                patch: 0,
                is_prerelease: false,
                prerelease: None,
                build: None,
                repo_owner: None,
                repo_name: None,
                forge: None,
                project_name: Some("cutver".into()),
            },
            "major",
            Some("main"),
        );
        assert_eq!(from_rc.project_name.as_deref(), Some("cutver"));
    }

    #[test]
    fn test_template_rendering_with_project_and_project_name() {
        let mut ctx = build_context_with_filter(
            "1.0.0",
            None,
            "v1.0.0",
            None,
            "2026-03-30",
            Some("https://github.com/my-org/my-repo".to_string()),
            &[],
            vec![],
            true,
            "entry",
            true,
            &[],
        );
        assert_eq!(ctx.project_name.as_deref(), Some("my-repo"));

        let template = "Project: {{ project_name }}, Alias: {{ project }}, Repo: {{ repo }}";
        let mut env = minijinja::Environment::new();
        env.add_template("test", template).unwrap();
        let tmpl = env.get_template("test").unwrap();
        let rendered = tmpl.render(&ctx).unwrap();
        assert_eq!(rendered, "Project: my-repo, Alias: my-repo, Repo: my-repo");

        ctx.project_name = Some("custom-project".to_string());
        let rendered = tmpl.render(&ctx).unwrap();
        assert_eq!(
            rendered,
            "Project: custom-project, Alias: custom-project, Repo: my-repo"
        );
    }
}
