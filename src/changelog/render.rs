mod body;
mod template;

#[cfg(test)]
mod tests;

pub(crate) use body::{
    PACKAGE_CATEGORY_HEADING_LEVEL, SECTION_HEADING_LEVEL, render_categorised, render_item, render_new_contributors,
};
pub use body::{render_body, render_body_with_context, render_body_with_plugin};
pub use template::{create_environment, interpolate_string, render_source, render_template};
// The section labels and their order are `SECTIONS`' alone. The release context reads them through
// this one re-export instead of carrying a second copy of the list.
pub(crate) use template::{BREAKING_SECTION_HEADING, section_for};

#[allow(unused_imports)]
pub use template::normalize_legacy_tokens;
