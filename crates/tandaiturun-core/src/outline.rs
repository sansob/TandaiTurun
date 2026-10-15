//! The document outline (the "Bookmarks" panel) and heading anchors.

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

/// One heading of the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    /// 1 to 6.
    pub level: u8,
    /// The heading text without formatting.
    pub text: String,
    /// The anchor used by `[link](#anchor)`: the `{#id}` attribute or a GitHub-style slug.
    pub anchor: String,
    /// Byte offset of the heading in the source.
    pub offset: usize,
    /// 0-based line of the heading in the source.
    pub line: usize,
}

pub fn level_number(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// All headings in document order, with unique anchors.
pub fn outline(markdown: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    let mut current: Option<(u8, Option<String>, usize, String)> = None;
    let mut slugs = Slugger::default();
    for (event, range) in Parser::new_ext(markdown, crate::parser_options()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, id, .. }) => {
                current = Some((level_number(level), id.map(|s| s.to_string()), range.start, String::new()));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, id, offset, text)) = current.take() {
                    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                    let anchor = match id {
                        Some(id) => slugs.claim(id),
                        None => slugs.slug(&text),
                    };
                    let line = markdown[..offset].matches('\n').count();
                    out.push(Heading { level, text, anchor, offset, line });
                }
            }
            Event::Text(t) | Event::Code(t) => {
                if let Some(c) = current.as_mut() {
                    c.3.push_str(&t);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(c) = current.as_mut() {
                    c.3.push(' ');
                }
            }
            _ => {}
        }
    }
    out
}

/// GitHub-style heading slugs, made unique with `-1`, `-2`… like GitHub does.
#[derive(Default)]
pub struct Slugger {
    seen: std::collections::HashMap<String, usize>,
}

impl Slugger {
    pub fn slug(&mut self, text: &str) -> String {
        self.claim(slugify(text))
    }

    fn claim(&mut self, base: String) -> String {
        let n = self.seen.entry(base.clone()).or_insert(0);
        let out = if *n == 0 { base.clone() } else { format!("{base}-{n}") };
        *n += 1;
        if out != base {
            self.seen.entry(out.clone()).or_insert(1);
        }
        out
    }
}

/// Lower case, spaces to `-`, punctuation dropped (letters of every script kept).
pub fn slugify(text: &str) -> String {
    text.trim()
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                Some(c.to_lowercase().collect::<String>())
            } else if c.is_whitespace() {
                Some("-".to_string())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_with_levels_lines_and_anchors() {
        let md =
            "# Judul *utama*\n\ntext\n\n## Bagian `satu`\n\nSetext\n------\n\n## Bagian satu\n### Custom {#mine}\n";
        let h = outline(md);
        let got: Vec<_> = h.iter().map(|h| (h.level, h.text.as_str(), h.anchor.as_str(), h.line)).collect();
        assert_eq!(
            got,
            vec![
                (1, "Judul utama", "judul-utama", 0),
                (2, "Bagian satu", "bagian-satu", 4),
                (2, "Setext", "setext", 6),
                (2, "Bagian satu", "bagian-satu-1", 9),
                (3, "Custom", "mine", 10),
            ]
        );
    }

    #[test]
    fn code_blocks_are_not_headings() {
        assert!(outline("```\n# not a heading\n```\n").is_empty());
    }

    #[test]
    fn slugs() {
        assert_eq!(slugify("Hello, World! 2.0"), "hello-world-20");
        assert_eq!(slugify("Apa Kabar?"), "apa-kabar");
        assert_eq!(slugify("Ünïcödé — ok"), "ünïcödé--ok");
    }
}
