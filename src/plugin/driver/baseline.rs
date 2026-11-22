//! The named system environment a process plugin's child receives before its declared grants
//! (#369 S1).
//!
//! `env_clear()` on its own is the right boundary for *authority* and the wrong one for *authoring*:
//! a plugin written in Python, Go or bash inherits no `PATH`, no `HOME`, no `TMPDIR` and, on Windows,
//! no `SystemRoot` — which stops a managed engine from starting at all. The child therefore runs with
//! this baseline plus the keys the plugin declared in `permissions.env`.
//!
//! The list is one list for every platform: a key this host does not set is simply absent, so a Unix
//! host ignores `SystemRoot` and a Windows host ignores `TMPDIR`. Naming one list keeps the reference
//! page and the code in agreement, and keeps the rule reviewable in one glance.

use std::collections::BTreeMap;

/// The system keys every process plugin's child receives, granted or not.
///
/// Two rules make the list safe to grant unconditionally:
///
/// - it carries **no secret**: no `*_TOKEN`, no `*_KEY`, no password or credential shape. A token in
///   CI stays out of the child unless the plugin declared it, which is the boundary `env_clear` was
///   there for (Pillar VI.2). `tests/process_driver_env_baseline.rs` fails when a name carries one of
///   the secret shapes it names, so the list cannot drift silently in that direction.
/// - it grants no capability the child did not already have: the child is an arbitrary program the
///   author wrote, and the filesystem and the network were never isolated from it. `PATH` and `HOME`
///   change how convenient it is to start, not what it can reach.
pub const SYSTEM_ENV_BASELINE: &[&str] = &[
    // Program lookup and the working directory of the user it runs as.
    "PATH",
    "HOME",
    "USERPROFILE",
    // Temporary directories: a managed engine and a standard library both need one.
    "TMPDIR",
    "TEMP",
    "TMP",
    // The locale a child decodes and encodes with.
    "LANG",
    "LC_ALL",
    // Windows: without SystemRoot a managed engine does not start, and the shell cannot resolve.
    "SystemRoot",
    "SYSTEMDRIVE",
    "APPDATA",
    "LOCALAPPDATA",
];

/// The baseline resolved against this process's environment, ready to hand to a child.
///
/// A key the host does not set — or sets to the empty string — is **omitted**, never carried as an
/// empty value: an empty `PATH`, `LANG` or `HOME` reads as a real answer and hides a broken
/// environment, while an absent key is exactly what the host says. Fail-closed belongs to the
/// declared grants — a declared key the host cannot produce is
/// [`crate::plugin::error::PluginError::UnsetEnvGrant`] — and the baseline must never turn a missing
/// system variable into a failed invocation.
pub fn system_env_baseline() -> BTreeMap<String, String> {
    baseline_from_lookup(|key| std::env::var(key).ok())
}

/// [`system_env_baseline`] over an injected lookup, so a test drives it without mutating the process
/// environment, whose mutators are unsafe under parallel tests (#344).
fn baseline_from_lookup(lookup: impl Fn(&str) -> Option<String>) -> BTreeMap<String, String> {
    SYSTEM_ENV_BASELINE
        .iter()
        .filter_map(|key| lookup(key).map(|value| ((*key).to_string(), value)))
        .filter(|(_, value)| !value.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_the_host_does_not_set_is_absent_from_the_map() {
        let baseline = baseline_from_lookup(|key| (key != "TMPDIR").then(|| format!("{key}-value")));

        assert!(
            !baseline.contains_key("TMPDIR"),
            "an unset system key is omitted, never carried as an empty or invented value"
        );
        assert_eq!(
            baseline.len(),
            SYSTEM_ENV_BASELINE.len() - 1,
            "every other baseline key is carried: the omission is per key"
        );
    }

    #[test]
    fn a_key_the_host_sets_to_the_empty_string_is_absent_from_the_map() {
        let baseline = baseline_from_lookup(|key| {
            Some(if key == "LANG" {
                String::new()
            } else {
                format!("{key}-value")
            })
        });

        assert!(
            !baseline.contains_key("LANG"),
            "a host that exports `LANG=` says it has no locale; hand that child an empty LANG and it \
             reads as a real answer instead of a missing one"
        );
    }

    #[test]
    fn the_baseline_carries_every_declared_key_when_the_host_sets_them_all() {
        let baseline = baseline_from_lookup(|key| Some(format!("{key}-value")));

        for key in SYSTEM_ENV_BASELINE {
            assert_eq!(
                baseline.get(*key).map(String::as_str),
                Some(format!("{key}-value").as_str()),
                "'{key}' is in the named baseline, so a host that sets it hands it over"
            );
        }
    }

    #[test]
    fn the_named_baseline_has_no_duplicate_key() {
        let mut names: Vec<String> = SYSTEM_ENV_BASELINE.iter().map(|key| key.to_ascii_uppercase()).collect();
        names.sort();
        let declared = names.len();
        names.dedup();

        assert_eq!(
            names.len(),
            declared,
            "a key named twice would be listed twice in the reference page and implies an intent \
             the code does not have"
        );
    }
}
