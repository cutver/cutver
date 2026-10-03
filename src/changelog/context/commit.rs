use crate::conventional::ConventionalCommit;

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
        let (clean, prs) = super::parse::strip_trailing_pr_numbers(&h.description);
        let clean_desc = h.clean_description.unwrap_or(clean);
        let pr_num = h.pr_number.or_else(|| prs.first().copied());
        let mut issues = h.issue_numbers;
        if prs.len() > 1 {
            for &num in &prs[1..] {
                if !issues.contains(&num) {
                    issues.push(num);
                }
            }
        }

        let mut ctx = CommitContext {
            commit_type: h.r#type,
            scope: h.scope,
            description: h.description,
            clean_description: clean_desc,
            is_breaking: h.is_breaking,
            hash: h.hash,
            short_hash: h.short_hash,
            author: h.author,
            author_email: h.author_email,
            pr_number: pr_num,
            pr_url: h.pr_url,
            issue_numbers: issues,
            commit_url: h.commit_url,
            line: String::new(),
            bullet: String::new(),
        };
        ctx.line = h.line.unwrap_or_else(|| super::enrich::format_commit_line(&ctx, true));
        ctx.bullet = h.bullet.unwrap_or_else(|| format!("- {}", ctx.line));
        Ok(ctx)
    }
}

impl From<&ConventionalCommit> for CommitContext {
    fn from(c: &ConventionalCommit) -> Self {
        let (clean, prs) = super::parse::strip_trailing_pr_numbers(&c.description);
        let pr_number = prs.first().copied();
        let issue_numbers = if prs.len() > 1 { prs[1..].to_vec() } else { Vec::new() };

        let mut ctx = Self {
            commit_type: c.commit_type.to_string(),
            scope: c.scope.clone(),
            description: c.description.clone(),
            clean_description: clean,
            is_breaking: c.is_breaking,
            hash: None,
            short_hash: None,
            author: None,
            author_email: None,
            pr_number,
            pr_url: None,
            issue_numbers,
            commit_url: None,
            line: String::new(),
            bullet: String::new(),
        };
        ctx.line = super::enrich::format_commit_line(&ctx, true);
        ctx.bullet = format!("- {}", ctx.line);
        ctx
    }
}
