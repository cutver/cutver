//! The grant-resolution tests of the process driver: an unset `permissions.env` key fails the
//! invocation, never the construction (#300 S3, S4).
//!
//! They live here rather than inside `process/tests.rs` because that file sits under the 400-line
//! ratchet in `tests/source_line_budget.rs`, whose table "can shrink but never grow".
//!
//! `use super::*` reaches the driver, the types and the `serde_json` import the parent test module
//! already has; the resolver is named explicitly because it is the authority these tests pin.

use super::*;
use crate::plugin::driver::env_grants_from_lookup;

/// S3 of #300: the driver is built on a host that cannot produce the declared grant, and the typed
/// error arrives when the plugin is invoked — the construction that only records the declaration
/// succeeds, so a command that never invokes the plugin never needs the secret.
///
/// The program does not exist: resolution runs before the spawn, so a passing test also proves no
/// child was attempted with a partial environment.
#[test]
fn test_an_unset_grant_fails_the_invocation_not_the_construction() {
    const KEY: &str = "CUTVER_TEST_UNSET_GRANT_KEY";

    let name = PluginName::new("grant-plugin").unwrap();
    let grants = DeclaredEnvGrants::with_lookup(vec![KEY.to_string()], std::sync::Arc::new(|_| None));
    let capabilities = HashSet::from([Capability::ManifestV1]);

    let driver = ProcessDriver::new(
        name.clone(),
        "cutver-nonexistent-plugin-program",
        vec![],
        grants,
        5,
        capabilities,
    );

    let invocation = PluginInvocation::new(PluginCall::ManifestRead, &serde_json::json!({})).unwrap();
    let err = driver
        .invoke(&invocation)
        .expect_err("an unset grant must fail the invocation");
    let message = err.to_string();

    match err {
        PluginError::UnsetEnvGrant {
            name: err_name,
            key: err_key,
        } => {
            assert_eq!(err_name, name);
            assert_eq!(err_key, KEY);
        }
        other => panic!("expected UnsetEnvGrant, got {other:?}"),
    }
    assert!(message.contains(KEY), "message must name the key: {message}");
    assert!(
        message.contains("[plugins.grant-plugin.permissions]"),
        "message must name the grant it failed to resolve: {message}"
    );
}

/// S4: a declared but unset grant is an error naming the plugin and the key, never an empty
/// string handed to the plugin. `env_grants_from_lookup` is the authority the process factory and
/// the WASM manifest builder both resolve their grants through, and the injected lookup stands in
/// for the process environment (#344).
#[test]
fn test_declared_but_unset_grant_names_plugin_and_key() {
    const KEY: &str = "CUTVER_TEST_UNSET_GRANT_KEY";

    let name = PluginName::new("grant-plugin").unwrap();
    let err = env_grants_from_lookup(&name, &[KEY.to_string()], |_| None).expect_err("an unset grant must fail");
    let message = err.to_string();

    match err {
        PluginError::UnsetEnvGrant {
            name: err_name,
            key: err_key,
        } => {
            assert_eq!(err_name, name);
            assert_eq!(err_key, KEY);
        }
        other => panic!("expected UnsetEnvGrant, got {other:?}"),
    }
    assert!(
        message.contains("grant-plugin"),
        "message must name the plugin: {message}"
    );
    assert!(message.contains(KEY), "message must name the key: {message}");
    assert!(
        message.contains("[plugins.grant-plugin.permissions]"),
        "message must name the grant it failed to resolve: {message}"
    );
}

/// S3: the pure resolver answers from the injected lookup alone. A present key keeps its value, and
/// a key the lookup cannot produce is an error — which is also what a non-UTF-8 value looks like,
/// because the boundary adapter is `|key| std::env::var(key).ok()` — never a silent empty string.
#[test]
fn test_injected_lookup_is_the_only_input_to_the_resolver() {
    let name = PluginName::new("lookup-plugin").unwrap();
    let declared = vec!["CUTVER_TEST_LOOKUP_PRESENT".to_string()];
    let key = declared[0].clone();

    let grants = env_grants_from_lookup(&name, &declared, |asked| {
        (asked == key).then(|| "present-value".to_string())
    })
    .expect("a key the lookup produces resolves to its value");
    assert_eq!(grants, BTreeMap::from([(key.clone(), "present-value".to_string())]));

    let err = env_grants_from_lookup(&name, &declared, |_| None).expect_err("a key the lookup cannot produce fails");
    assert!(
        matches!(&err, PluginError::UnsetEnvGrant { key: err_key, .. } if err_key == &key),
        "expected UnsetEnvGrant naming the key, got {err:?}"
    );
}

/// #300: the driver's `Debug` reaches test output and error paths, so it prints the declared keys
/// and never a resolved value — a grant is a secret until the plugin it was granted to reads it.
#[test]
fn test_the_declared_grants_debug_prints_keys_never_values() {
    let grants = DeclaredEnvGrants::with_lookup(
        vec!["CUTVER_TEST_SECRET_KEY".to_string()],
        std::sync::Arc::new(|_| Some("secret-value".to_string())),
    );

    let printed = format!("{grants:?}");
    assert!(
        printed.contains("CUTVER_TEST_SECRET_KEY"),
        "the declaration travels: {printed}"
    );
    assert!(
        !printed.contains("secret-value"),
        "a resolved value must never be printed: {printed}"
    );
}
