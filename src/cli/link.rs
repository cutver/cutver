use std::collections::BTreeMap;
use std::fmt::Display;

/// Configuration for hyperlink and footnote rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkMode {
    /// Whether OSC 8 terminal hyperlinks should be emitted.
    /// Only true when interactive TTY, colors enabled, and not plain.
    pub hyperlinks: bool,
    /// Whether footnotes are enabled/collected.
    pub footnotes: bool,
}

impl LinkMode {
    /// Determine link mode from color/TTY enablement and plain flag.
    pub const fn new(colors_enabled: bool, plain: bool) -> Self {
        if plain || !colors_enabled {
            Self {
                hyperlinks: false,
                footnotes: true,
            }
        } else {
            Self {
                hyperlinks: true,
                footnotes: true,
            }
        }
    }
}

/// Formats an OSC 8 terminal hyperlink:
/// `\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\`
pub fn format_osc8_link(text: impl Display, url: &str) -> String {
    format!("\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\")
}

/// Helper to format a link honoring LinkMode.
///
/// If `hyperlinks` is enabled, formats with OSC 8.
/// Otherwise, returns the plain text unchanged (or with url if plain fallback requested).
pub fn format_link(text: &str, url: &str, mode: LinkMode) -> String {
    if mode.hyperlinks {
        format_osc8_link(text, url)
    } else {
        text.to_string()
    }
}

/// Registry of accessible footnote links for terminal and screen reader users (NVDA/JAWS/Orca).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FootnoteCollector {
    urls: Vec<String>,
    indices: BTreeMap<String, usize>,
}

impl FootnoteCollector {
    /// Create a new empty footnote collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a URL or get existing footnote reference index (1-based).
    pub fn add(&mut self, url: &str) -> usize {
        if let Some(&idx) = self.indices.get(url) {
            return idx;
        }
        let idx = self.urls.len() + 1;
        self.urls.push(url.to_string());
        self.indices.insert(url.to_string(), idx);
        idx
    }

    /// Render formatted link for the given text and URL based on LinkMode.
    pub fn link(&mut self, text: &str, url: &str, mode: LinkMode) -> String {
        let idx = self.add(url);
        if mode.hyperlinks {
            let osc8 = format_osc8_link(text, url);
            format!("{osc8}[^{idx}]")
        } else {
            format!("{text}[^{idx}]")
        }
    }

    /// Returns whether any footnotes have been recorded.
    pub fn is_empty(&self) -> bool {
        self.urls.is_empty()
    }

    /// Returns the number of collected footnotes.
    pub fn len(&self) -> usize {
        self.urls.len()
    }

    /// Renders the collected footnotes block.
    ///
    /// Output format:
    /// ```text
    /// [^1]: https://example.com/1
    /// [^2]: https://example.com/2
    /// ```
    pub fn render_footnotes(&self) -> String {
        if self.urls.is_empty() {
            return String::new();
        }
        let lines: Vec<String> = self
            .urls
            .iter()
            .enumerate()
            .map(|(i, url)| format!("[^{}]: {url}", i + 1))
            .collect();
        lines.join("\n")
    }
}

/// Transform markdown links `[text](url)` in a line to OSC 8 hyperlinks and/or accessible footnotes.
pub fn transform_markdown_links(line: &str, mode: LinkMode, collector: &mut FootnoteCollector) -> String {
    let mut result = String::with_capacity(line.len());
    let mut remaining = line;

    while let Some(start_bracket) = remaining.find('[') {
        let before = &remaining[..start_bracket];
        result.push_str(before);
        let after_bracket = &remaining[start_bracket + 1..];

        let Some(end_bracket) = after_bracket.find(']') else {
            result.push('[');
            remaining = after_bracket;
            continue;
        };

        let link_text = &after_bracket[..end_bracket];
        let after_text = &after_bracket[end_bracket + 1..];

        if !after_text.starts_with('(') {
            result.push('[');
            result.push_str(link_text);
            result.push(']');
            remaining = after_text;
            continue;
        }

        let after_paren = &after_text[1..];
        let Some(end_paren) = after_paren.find(')') else {
            result.push('[');
            result.push_str(link_text);
            result.push_str("](");
            remaining = after_paren;
            continue;
        };

        let url = &after_paren[..end_paren];
        let replacement = collector.link(link_text, url, mode);
        result.push_str(&replacement);
        remaining = &after_paren[end_paren + 1..];
    }

    result.push_str(remaining);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_osc8_link() {
        let rendered = format_osc8_link("#123", "https://github.com/Row0902/cutver/pull/123");
        assert_eq!(
            rendered,
            "\x1b]8;;https://github.com/Row0902/cutver/pull/123\x1b\\#123\x1b]8;;\x1b\\"
        );
    }

    #[test]
    fn test_format_link_with_mode() {
        let url = "https://github.com/Row0902/cutver";
        let mode_hyper = LinkMode::new(true, false);
        assert_eq!(
            format_link("cutver", url, mode_hyper),
            "\x1b]8;;https://github.com/Row0902/cutver\x1b\\cutver\x1b]8;;\x1b\\"
        );
        let mode_plain = LinkMode::new(true, true);
        assert_eq!(format_link("cutver", url, mode_plain), "cutver");
        let mode_no_color = LinkMode::new(false, false);
        assert_eq!(format_link("cutver", url, mode_no_color), "cutver");
    }

    #[test]
    fn test_footnote_collector_deduplicates_urls() {
        let mut collector = FootnoteCollector::new();
        let idx1 = collector.add("https://github.com/Row0902/cutver/pull/1");
        let idx2 = collector.add("https://github.com/Row0902/cutver/pull/2");
        let idx3 = collector.add("https://github.com/Row0902/cutver/pull/1");

        assert_eq!(idx1, 1);
        assert_eq!(idx2, 2);
        assert_eq!(idx3, 1);
        assert_eq!(collector.len(), 2);
    }

    #[test]
    fn test_footnote_collector_renders_hyperlink_with_mode() {
        let mut collector = FootnoteCollector::new();
        let mode_hyperlinks = LinkMode::new(true, false);
        let link_out = collector.link("#10", "https://github.com/Row0902/cutver/pull/10", mode_hyperlinks);

        assert_eq!(
            link_out,
            "\x1b]8;;https://github.com/Row0902/cutver/pull/10\x1b\\#10\x1b]8;;\x1b\\[^1]"
        );

        let footnotes = collector.render_footnotes();
        assert_eq!(footnotes, "[^1]: https://github.com/Row0902/cutver/pull/10");
    }

    #[test]
    fn test_footnote_collector_renders_plain_with_mode() {
        let mut collector = FootnoteCollector::new();
        let mode_plain = LinkMode::new(true, true);
        let link_out = collector.link("#10", "https://github.com/Row0902/cutver/pull/10", mode_plain);

        assert_eq!(link_out, "#10[^1]");

        let mode_no_color = LinkMode::new(false, false);
        let link_out2 = collector.link("#20", "https://github.com/Row0902/cutver/pull/20", mode_no_color);
        assert_eq!(link_out2, "#20[^2]");

        let footnotes = collector.render_footnotes();
        assert_eq!(
            footnotes,
            "[^1]: https://github.com/Row0902/cutver/pull/10\n[^2]: https://github.com/Row0902/cutver/pull/20"
        );
    }

    #[test]
    fn test_transform_markdown_links() {
        let mut collector = FootnoteCollector::new();
        let mode = LinkMode::new(true, false);
        let line = "- **cli**: in [#123](https://github.com/Row0902/cutver/pull/123) ([abc](https://github.com/Row0902/cutver/commit/abc))";
        let transformed = transform_markdown_links(line, mode, &mut collector);
        assert!(transformed.contains("\x1b]8;;https://github.com/Row0902/cutver/pull/123\x1b\\#123\x1b]8;;\x1b\\[^1]"));
        assert!(
            transformed.contains("\x1b]8;;https://github.com/Row0902/cutver/commit/abc\x1b\\abc\x1b]8;;\x1b\\[^2]")
        );
        assert_eq!(collector.len(), 2);
    }
}
