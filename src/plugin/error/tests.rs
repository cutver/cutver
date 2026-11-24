use super::*;

/// Pins the lean-binary advice so it cannot drift back to a remedy that cannot work.
///
/// `WasmNotSupported` is reachable only from the `#[cfg(not(feature = "plugins"))]`
/// branch of `create_wasm_runtime_driver`, so it always means the binary was built
/// without the `plugins` feature. The fix must name that feature and the
/// plugin-enabled build, never a `cutver.toml` key.
#[test]
fn wasm_not_supported_names_the_plugins_feature_and_the_enabled_build() {
    let name = PluginName::new("demo").unwrap();
    let message = PluginError::WasmNotSupported { name }.to_string();

    assert!(
        message.contains("plugins"),
        "message must name the 'plugins' feature token: {message}"
    );
    assert!(
        message.contains("--features plugins"),
        "message must show the plugin-enabled build command: {message}"
    );
    assert!(
        !message.contains("runtime = \"process\""),
        "message must not recommend a cutver.toml key that cannot help: {message}"
    );
}

/// S6 (#369): the overrun names the limit it crossed and routes the author to the channel that is
/// safe to be loud on. Its remedy is never a longer `timeout_seconds` — nothing was waiting.
#[test]
fn an_answer_over_the_limit_names_the_limit_and_the_remedy() {
    let name = PluginName::new("loud-plugin").unwrap();
    let message = PluginError::AnswerTooLarge {
        name,
        capability: "changelog.v1".to_string(),
        limit_bytes: 1024 * 1024,
    }
    .to_string();

    assert!(
        message.contains("1048576 bytes"),
        "the message names the limit in bytes: {message}"
    );
    assert!(
        message.contains("changelog.v1"),
        "the message names the capability that was refused: {message}"
    );
    assert!(
        message.contains("Where:") && message.contains("Fix:"),
        "the house style carries a capitalized Where: and Fix:: {message}"
    );
    assert!(
        message.contains("stderr"),
        "the remedy points at the channel a plugin may be loud on: {message}"
    );
    assert!(
        !message.contains("timeout_seconds"),
        "an overrun is not a deadline, so its remedy must not be a longer timeout: {message}"
    );
    assert!(
        message.contains("kept only when the call fails") && message.contains("root_dir"),
        "the remedy sends the author to stderr, which is discarded on a successful call (#369 S7): \
         leaving that out walks them into silence, so the message must say where a line survives \
         and where to write what they must see: {message}"
    );
}

/// Pins the size of the plugin error so it cannot silently break `bump`'s lint gate.
///
/// `bump::Error` embeds this enum by value (`Plugin(#[from] PluginError)` in
/// `src/bump/types.rs`), so every byte added here is added there. The repo runs
/// `clippy -D warnings`, and `clippy::result_large_err` starts firing on `bump`'s
/// `Result`-returning functions once `bump::Error` passes 128 bytes: a larger
/// variant here breaks linting in a module that did not change.
///
/// A new variant must therefore stay compact (bounded fields, no third `String`
/// in one variant) or be boxed. Raise these bounds only together with a deliberate
/// change to `bump`'s error type.
#[test]
fn plugin_error_stays_within_the_bump_result_budget() {
    let plugin = std::mem::size_of::<PluginError>();
    let bump = std::mem::size_of::<crate::bump::Error>();

    assert!(
        plugin <= 96,
        "PluginError grew to {plugin} bytes; bump::Error embeds it by value and \
         clippy::result_large_err fires on bump above 128 bytes"
    );
    assert!(
        bump <= 128,
        "bump::Error is {bump} bytes; clippy::result_large_err fires on its Result-returning \
         functions above 128 bytes"
    );
}
