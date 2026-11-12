mod body;
mod template;

#[cfg(test)]
mod tests;

pub(crate) use body::{
    PACKAGE_CATEGORY_HEADING_LEVEL, SECTION_HEADING_LEVEL, render_categorised, render_item, render_new_contributors,
};
pub use body::{render_body, render_body_with_context, render_body_with_plugin};
pub use template::{create_environment, interpolate_string, render_source, render_template};

#[allow(unused_imports)]
pub use template::normalize_legacy_tokens;
