//! Glob matching utilities for manifest pattern resolution.

/// Matches a glob pattern against a normalized path using regex.
///
/// Supported tokens:
/// - `**`: matches arbitrary directories across `/`
/// - `*`: matches any characters except `/`
/// - `?`: matches any single character except `/`
/// - Literal text: matches exactly
pub(crate) fn glob_matches(pattern: &str, path: &str) -> bool {
    // Normalize backslashes to forward slashes for cross-platform glob matching
    let norm_path = path.replace('\\', "/");
    let norm_pattern = pattern.replace('\\', "/");

    let regex_pattern = glob_to_regex(&norm_pattern);
    match regex::Regex::new(&regex_pattern) {
        Ok(re) => re.is_match(&norm_path),
        Err(_) => false,
    }
}

fn glob_to_regex(pattern: &str) -> String {
    let mut regex = String::from("^");
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' => {
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    // Match '**'
                    i += 2;
                    if i < chars.len() && chars[i] == '/' {
                        // "**/": matches zero or more directories
                        regex.push_str("(?:.*/)?");
                        i += 1;
                    } else {
                        // "**": matches anything
                        regex.push_str(".*");
                    }
                } else {
                    // Single '*': matches non-slash characters
                    regex.push_str("[^/]*");
                    i += 1;
                }
            }
            '?' => {
                regex.push_str("[^/]");
                i += 1;
            }
            c => {
                regex.push_str(&regex::escape(&c.to_string()));
                i += 1;
            }
        }
    }
    regex.push('$');
    regex
}
