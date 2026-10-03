use crate::semver_bump::Bump;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConventionalCommit {
    pub commit_type: String,
    pub scope: Option<String>,
    pub is_breaking: bool,
    pub description: String,
    pub body: Option<String>,
    pub footers: Vec<(String, String)>,
}

/// SemVer bump deduction rationale with commit counts and breaking change sample.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BumpRationale {
    pub level: Bump,
    pub breaking_count: usize,
    pub feat_count: usize,
    pub fix_count: usize,
    pub other_count: usize,
    pub breaking_sample: Option<String>,
}

impl BumpRationale {
    /// Describes the SemVer deduction decision cleanly.
    ///
    /// Examples:
    /// - "major (deduced from breaking change: feat!: ...)"
    /// - "minor (deduced from 3 features, 1 bugfix)"
    /// - "patch (default - no breaking changes or features)"
    pub fn summary(&self) -> String {
        let level_str = match self.level {
            Bump::Major => "major",
            Bump::Minor => "minor",
            Bump::Patch => "patch",
        };
        match self.level {
            Bump::Major => {
                if let Some(sample) = &self.breaking_sample {
                    format!("{level_str} (deduced from breaking change: {sample})")
                } else if self.breaking_count > 0 {
                    let s = if self.breaking_count == 1 { "" } else { "s" };
                    format!("{level_str} (deduced from {} breaking change{s})", self.breaking_count)
                } else {
                    format!("{level_str} (deduced from breaking changes)")
                }
            }
            Bump::Minor => {
                let mut parts = Vec::new();
                if self.feat_count > 0 {
                    let s = if self.feat_count == 1 { "" } else { "s" };
                    parts.push(format!("{} feature{s}", self.feat_count));
                }
                if self.fix_count > 0 {
                    let s = if self.fix_count == 1 { "" } else { "es" };
                    parts.push(format!("{} bugfix{s}", self.fix_count));
                }
                if parts.is_empty() {
                    format!("{level_str} (deduced from features)")
                } else {
                    format!("{level_str} (deduced from {})", parts.join(", "))
                }
            }
            Bump::Patch => {
                let mut parts = Vec::new();
                if self.fix_count > 0 {
                    let s = if self.fix_count == 1 { "" } else { "es" };
                    parts.push(format!("{} bugfix{s}", self.fix_count));
                }
                if self.other_count > 0 {
                    let s = if self.other_count == 1 { "" } else { "s" };
                    parts.push(format!("{} other change{s}", self.other_count));
                }
                if parts.is_empty() {
                    format!("{level_str} (default - no breaking changes or features)")
                } else {
                    format!("{level_str} (deduced from {})", parts.join(", "))
                }
            }
        }
    }
}

impl std::fmt::Display for BumpRationale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}
