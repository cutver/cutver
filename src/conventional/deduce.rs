use super::types::{BumpRationale, ConventionalCommit};
use crate::semver_bump::Bump;

/// Deduce the SemVer bump rationale from a list of parsed conventional commits.
pub fn deduce_rationale(commits: &[ConventionalCommit]) -> BumpRationale {
    let mut breaking_count = 0;
    let mut feat_count = 0;
    let mut fix_count = 0;
    let mut other_count = 0;
    let mut breaking_sample = None;

    for commit in commits {
        if commit.is_breaking {
            breaking_count += 1;
            if breaking_sample.is_none() {
                let scope_str = commit.scope.as_deref().map(|s| format!("({s})")).unwrap_or_default();
                breaking_sample = Some(format!("{}{scope_str}!: {}", commit.commit_type, commit.description));
            }
        }
        match commit.commit_type.as_str() {
            "feat" => feat_count += 1,
            "fix" => fix_count += 1,
            _ => other_count += 1,
        }
    }

    let level = if breaking_count > 0 {
        Bump::Major
    } else if feat_count > 0 {
        Bump::Minor
    } else {
        Bump::Patch
    };

    BumpRationale {
        level,
        breaking_count,
        feat_count,
        fix_count,
        other_count,
        breaking_sample,
    }
}

/// Deduce the SemVer bump level from a list of parsed conventional commits.
///
/// Returns:
/// - `Bump::Major` if any commit has `is_breaking == true`.
/// - `Bump::Minor` if any commit has `commit_type == "feat"`.
/// - `Bump::Patch` otherwise (e.g. `fix`, `perf`, `refactor`, `chore`, `docs`, etc., or empty).
pub fn deduce_bump(commits: &[ConventionalCommit]) -> Bump {
    if commits.iter().any(|c| c.is_breaking) {
        Bump::Major
    } else if commits.iter().any(|c| c.commit_type == "feat") {
        Bump::Minor
    } else {
        Bump::Patch
    }
}

/// Parse all commit messages and deduce the appropriate SemVer bump rationale.
pub fn parse_and_deduce_with_rationale(messages: &[impl AsRef<str>]) -> (BumpRationale, Vec<ConventionalCommit>) {
    let commits: Vec<ConventionalCommit> = messages
        .iter()
        .filter_map(|m| ConventionalCommit::parse(m.as_ref()))
        .collect();

    let rationale = deduce_rationale(&commits);
    (rationale, commits)
}

/// Parse all commit messages and deduce the appropriate SemVer bump.
///
/// Defaults to `(Bump::Patch, vec![])` if no valid Conventional Commits are found.
pub fn parse_and_deduce_bump(messages: &[impl AsRef<str>]) -> (Bump, Vec<ConventionalCommit>) {
    let (rationale, commits) = parse_and_deduce_with_rationale(messages);
    (rationale.level, commits)
}
