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
