mod body;
mod template;

#[cfg(test)]
mod tests;

pub use body::{render_body, render_body_with_context, render_body_with_plugin};
pub use template::{create_environment, interpolate_string, render_template};

#[allow(unused_imports)]
pub use template::normalize_legacy_tokens;
