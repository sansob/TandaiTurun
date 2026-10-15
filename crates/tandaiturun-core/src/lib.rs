//! The parts of TandaiTurun that don't need a window: reading and writing
//! Markdown files, the document outline, search, the editing commands behind
//! the formatting toolbar, document statistics and HTML export.
//!
//! Everything here is plain Rust on top of [`pulldown_cmark`], so it can be
//! tested without a display and reused by other tools.

pub mod document;
pub mod format;
pub mod html;
pub mod outline;
pub mod search;
pub mod stats;

pub use pulldown_cmark;

use pulldown_cmark::Options;

/// The Markdown dialect used everywhere in TandaiTurun: CommonMark plus the
/// GitHub extensions people expect (tables, task lists, strikethrough,
/// footnotes) and YAML front matter, which is hidden from the rendered page.
pub fn parser_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_GFM
        | Options::ENABLE_MATH
}

/// The HTML comment that starts a new page in the reader and in print.
pub const PAGE_BREAK: &str = "<!-- pagebreak -->";

/// Whether a raw HTML snippet from the parser is a page-break marker.
pub fn is_page_break(html: &str) -> bool {
    let t = html.trim();
    t.eq_ignore_ascii_case(PAGE_BREAK) || t.eq_ignore_ascii_case("<!-- page-break -->") || t == "\\pagebreak"
}
