//! Numbers for the Document Properties dialog.

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    /// Words of the rendered text (markup, link targets and front matter excluded).
    pub words: usize,
    /// Characters of the rendered text, spaces included.
    pub characters: usize,
    pub characters_no_spaces: usize,
    /// Lines of the source.
    pub lines: usize,
    pub headings: usize,
    pub links: usize,
    pub images: usize,
    pub code_blocks: usize,
    pub tables: usize,
}

impl Stats {
    /// Reading time in minutes at 220 words per minute, at least 1 for any text.
    pub fn reading_minutes(&self) -> usize {
        if self.words == 0 {
            0
        } else {
            self.words.div_ceil(220)
        }
    }
}

pub fn stats(markdown: &str) -> Stats {
    let mut s = Stats { lines: if markdown.is_empty() { 0 } else { markdown.lines().count() }, ..Default::default() };
    let mut in_meta = false;
    let mut text = String::new();
    for event in Parser::new_ext(markdown, crate::parser_options()) {
        match event {
            Event::Start(Tag::MetadataBlock(_)) => in_meta = true,
            Event::End(TagEnd::MetadataBlock(_)) => in_meta = false,
            Event::Start(Tag::Heading { .. }) => s.headings += 1,
            Event::Start(Tag::Link { .. }) => s.links += 1,
            Event::Start(Tag::Image { .. }) => s.images += 1,
            Event::Start(Tag::CodeBlock(_)) => s.code_blocks += 1,
            Event::Start(Tag::Table(_)) => s.tables += 1,
            Event::Text(t) | Event::Code(t) if !in_meta => {
                text.push_str(&t);
            }
            Event::SoftBreak | Event::HardBreak => text.push(' '),
            Event::End(
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::Item
                | TagEnd::TableCell
                | TagEnd::CodeBlock
                | TagEnd::BlockQuote(_)
                | TagEnd::Image,
            ) => text.push(' '),
            _ => {}
        }
    }
    s.words = text.split_whitespace().count();
    s.characters = text.split_whitespace().collect::<Vec<_>>().join(" ").chars().count();
    s.characters_no_spaces = text.chars().filter(|c| !c.is_whitespace()).count();
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts() {
        let md = "---\ntitle: x y z\n---\n# Halo dunia\n\nIni **tebal** dan [tautan](https://example.com).\n\n```\ncode here\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n![alt](x.png)\n";
        let s = stats(md);
        assert_eq!(s.headings, 1);
        assert_eq!(s.links, 1);
        assert_eq!(s.images, 1);
        assert_eq!(s.code_blocks, 1);
        assert_eq!(s.tables, 1);
        // halo dunia ini tebal dan tautan. code here a b 1 2 alt
        assert_eq!(s.words, 13);
        assert_eq!(stats("**te**bal").words, 1);
        assert_eq!(s.reading_minutes(), 1);
        assert_eq!(stats("").reading_minutes(), 0);
    }
}
