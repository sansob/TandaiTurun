//! Markdown → a small block tree that the page layout works from.
//!
//! The tree keeps only what the reader draws: block structure, inline spans
//! with their styles and links, images, and the source offset of every
//! top-level block (used to keep the editor and the preview in step).

use pulldown_cmark::{Alignment, CodeBlockKind, Event, Parser, Tag, TagEnd};
use tandaiturun_core::{is_page_break, outline::level_number, parser_options};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpanStyle {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    pub sup: bool,
    pub sub: bool,
    pub mark: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub style: SpanStyle,
    pub link: Option<String>,
}

/// Inline content: text spans, line breaks and images.
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Span(Span),
    Break,
    Image {
        url: String,
        alt: String,
        link: Option<String>,
    },
    /// LaTeX math: `$…$` (inline) or `$$…$$` (display).
    Math {
        tex: String,
        display: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Code {
        lang: String,
        text: String,
    },
    Quote(Vec<Block>),
    List {
        start: Option<u64>,
        tight: bool,
        items: Vec<ListItem>,
    },
    Table {
        aligns: Vec<Alignment>,
        head: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    Rule,
    PageBreak,
    Footnote {
        /// The label from the source, for the `#fn-…` anchor.
        label: String,
        /// What's shown: its position among the references.
        number: usize,
        blocks: Vec<Block>,
    },
    /// Raw HTML we can't render: shown as muted text.
    Html(String),
}

/// A top-level block and where it starts in the source.
#[derive(Debug, Clone, PartialEq)]
pub struct TopBlock {
    pub offset: usize,
    pub block: Block,
}

struct Frame {
    kind: FrameKind,
    blocks: Vec<Block>,
    inlines: Vec<Inline>,
}

enum FrameKind {
    Root,
    Paragraph,
    Heading(u8),
    Quote,
    List { start: Option<u64>, items: Vec<ListItem>, tight: bool },
    Item { task: Option<bool> },
    Footnote(String),
    Table { aligns: Vec<Alignment>, head: Vec<Vec<Inline>>, rows: Vec<Vec<Vec<Inline>>>, in_head: bool },
    Row(Vec<Vec<Inline>>),
    Cell,
}

/// Extracts `src="…"` and `alt="…"` of every `<img>` in an HTML snippet.
fn html_images(html: &str) -> Vec<(String, String)> {
    let lower = html.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = lower[at..].find("<img") {
        let start = at + i;
        let end = lower[start..].find('>').map_or(html.len(), |e| start + e);
        let tag = &html[start..end];
        let attr = |name: &str| -> Option<String> {
            let tl = tag.to_ascii_lowercase();
            let k = tl.find(&format!("{name}="))? + name.len() + 1;
            let rest = &tag[k..];
            let q = rest.chars().next()?;
            if q == '"' || q == '\'' {
                rest[1..].find(q).map(|e| rest[1..1 + e].to_string())
            } else {
                Some(rest.split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("").to_string())
            }
        };
        if let Some(src) = attr("src") {
            out.push((src, attr("alt").unwrap_or_default()));
        }
        at = end;
    }
    out
}

/// Text of an HTML snippet without its tags and comments.
fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix("<!--") {
            rest = r.find("-->").map_or("", |e| &r[e + 3..]);
        } else if rest.starts_with('<') {
            rest = rest.find('>').map_or("", |e| &rest[e + 1..]);
        } else {
            let e = rest.find('<').unwrap_or(rest.len());
            out.push_str(&rest[..e]);
            rest = &rest[e..];
        }
    }
    out.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
}

/// Parses Markdown into top-level blocks (front matter is skipped).
pub fn parse(markdown: &str) -> Vec<TopBlock> {
    let mut out: Vec<TopBlock> = Vec::new();
    let mut stack: Vec<Frame> = vec![Frame { kind: FrameKind::Root, blocks: Vec::new(), inlines: Vec::new() }];
    let mut style = SpanStyle::default();
    let mut links: Vec<String> = Vec::new();
    let mut image: Option<(String, String)> = None;
    let mut code: Option<(String, String)> = None;
    let mut in_meta = false;
    let mut top_offset = 0usize;
    // Footnotes are shown numbered in the order they're first referenced,
    // like on GitHub, whatever their labels; unreferenced ones go last.
    let mut notes: Vec<String> = Vec::new();
    for e in Parser::new_ext(markdown, parser_options()) {
        if let Event::FootnoteReference(l) | Event::Start(Tag::FootnoteDefinition(l)) = e {
            if !notes.iter().any(|n| *n == *l) {
                notes.push(l.to_string());
            }
        }
    }
    let note_number = |l: &str| notes.iter().position(|n| n == l).map_or(0, |i| i + 1);

    // Blocks that complete at the root go straight to `out` with their offset.
    macro_rules! push_block {
        ($stack:expr, $b:expr) => {{
            let b = $b;
            if $stack.len() == 1 {
                out.push(TopBlock { offset: top_offset, block: b });
            } else {
                $stack.last_mut().expect("frame").blocks.push(b);
            }
        }};
    }

    for (event, range) in Parser::new_ext(markdown, parser_options()).into_offset_iter() {
        if stack.len() == 1 && matches!(event, Event::Start(_) | Event::Rule | Event::Html(_)) {
            top_offset = range.start;
        }
        if in_meta {
            if let Event::End(TagEnd::MetadataBlock(_)) = event {
                in_meta = false;
            }
            continue;
        }
        if let Some((_, alt)) = image.as_mut() {
            match event {
                Event::Text(t) | Event::Code(t) => alt.push_str(&t),
                Event::End(TagEnd::Image) => {
                    let (url, alt) = image.take().expect("image");
                    let f = stack.last_mut().expect("frame");
                    f.inlines.push(Inline::Image { url, alt, link: links.last().cloned() });
                }
                _ => {}
            }
            continue;
        }
        if let Some((_, text)) = code.as_mut() {
            match event {
                Event::Text(t) => text.push_str(&t),
                Event::End(TagEnd::CodeBlock) => {
                    let (lang, mut text) = code.take().expect("code");
                    if text.ends_with('\n') {
                        text.pop();
                    }
                    push_block!(stack, Block::Code { lang, text });
                }
                _ => {}
            }
            continue;
        }
        let span = |text: &str, style: SpanStyle, links: &[String]| {
            Inline::Span(Span { text: text.to_string(), style, link: links.last().cloned() })
        };
        match event {
            Event::Start(tag) => match tag {
                Tag::MetadataBlock(_) => in_meta = true,
                Tag::Paragraph => stack.push(Frame { kind: FrameKind::Paragraph, blocks: vec![], inlines: vec![] }),
                Tag::Heading { level, .. } => {
                    stack.push(Frame { kind: FrameKind::Heading(level_number(level)), blocks: vec![], inlines: vec![] })
                }
                Tag::BlockQuote(_) => stack.push(Frame { kind: FrameKind::Quote, blocks: vec![], inlines: vec![] }),
                Tag::CodeBlock(kind) => {
                    let lang = match kind {
                        CodeBlockKind::Fenced(l) => l.split([' ', ',', '{']).next().unwrap_or("").to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    code = Some((lang, String::new()));
                }
                Tag::List(start) => stack.push(Frame {
                    kind: FrameKind::List { start, items: vec![], tight: true },
                    blocks: vec![],
                    inlines: vec![],
                }),
                Tag::Item => {
                    stack.push(Frame { kind: FrameKind::Item { task: None }, blocks: vec![], inlines: vec![] })
                }
                Tag::FootnoteDefinition(label) => {
                    stack.push(Frame { kind: FrameKind::Footnote(label.to_string()), blocks: vec![], inlines: vec![] })
                }
                Tag::Table(aligns) => stack.push(Frame {
                    kind: FrameKind::Table { aligns, head: vec![], rows: vec![], in_head: false },
                    blocks: vec![],
                    inlines: vec![],
                }),
                Tag::TableHead => {
                    if let Some(Frame { kind: FrameKind::Table { in_head, .. }, .. }) = stack.last_mut() {
                        *in_head = true;
                    }
                    stack.push(Frame { kind: FrameKind::Row(vec![]), blocks: vec![], inlines: vec![] });
                }
                Tag::TableRow => stack.push(Frame { kind: FrameKind::Row(vec![]), blocks: vec![], inlines: vec![] }),
                Tag::TableCell => stack.push(Frame { kind: FrameKind::Cell, blocks: vec![], inlines: vec![] }),
                Tag::Emphasis => style.italic = true,
                Tag::Strong => style.bold = true,
                Tag::Strikethrough => style.strike = true,
                Tag::Superscript => style.sup = true,
                Tag::Subscript => style.sub = true,
                Tag::Link { dest_url, .. } => links.push(dest_url.to_string()),
                Tag::Image { dest_url, .. } => image = Some((dest_url.to_string(), String::new())),
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Emphasis => style.italic = false,
                TagEnd::Strong => style.bold = false,
                TagEnd::Strikethrough => style.strike = false,
                TagEnd::Superscript => style.sup = false,
                TagEnd::Subscript => style.sub = false,
                TagEnd::Link => {
                    links.pop();
                }
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::BlockQuote(_)
                | TagEnd::Item
                | TagEnd::FootnoteDefinition
                | TagEnd::List(_)
                | TagEnd::Table
                | TagEnd::TableHead
                | TagEnd::TableRow
                | TagEnd::TableCell => {
                    let f = stack.pop().expect("frame");
                    match f.kind {
                        FrameKind::Paragraph => {
                            // In a tight list item the text arrives without a paragraph.
                            for b in split_images(f.inlines) {
                                push_block!(stack, b);
                            }
                        }
                        FrameKind::Heading(level) => push_block!(stack, Block::Heading { level, inlines: f.inlines }),
                        FrameKind::Quote => push_block!(stack, Block::Quote(f.blocks)),
                        FrameKind::Item { task } => {
                            let mut blocks = f.blocks;
                            if !f.inlines.is_empty() {
                                // Tight item text: put it first, before any nested list.
                                let mut first = split_images(f.inlines);
                                first.append(&mut blocks);
                                blocks = first;
                            }
                            if let Some(Frame { kind: FrameKind::List { items, .. }, .. }) = stack.last_mut() {
                                items.push(ListItem { task, blocks });
                            }
                        }
                        FrameKind::List { start, items, tight } => {
                            push_block!(stack, Block::List { start, tight, items })
                        }
                        FrameKind::Footnote(label) => {
                            let number = note_number(&label);
                            push_block!(stack, Block::Footnote { label, number, blocks: f.blocks })
                        }
                        FrameKind::Cell => {
                            if let Some(Frame { kind: FrameKind::Row(cells), .. }) = stack.last_mut() {
                                cells.push(f.inlines);
                            }
                        }
                        FrameKind::Row(cells) => {
                            if let Some(Frame { kind: FrameKind::Table { head, rows, in_head, .. }, .. }) =
                                stack.last_mut()
                            {
                                if *in_head {
                                    *head = cells;
                                    *in_head = false;
                                } else {
                                    rows.push(cells);
                                }
                            }
                        }
                        FrameKind::Table { aligns, head, rows, .. } => {
                            push_block!(stack, Block::Table { aligns, head, rows })
                        }
                        FrameKind::Root => unreachable!("root is never popped"),
                    }
                }
                _ => {}
            },
            Event::Text(t) => {
                let f = stack.last_mut().expect("frame");
                f.inlines.push(span(&t, style, &links));
            }
            Event::Code(t) => {
                let f = stack.last_mut().expect("frame");
                f.inlines.push(span(&t, SpanStyle { code: true, ..style }, &links));
            }
            Event::InlineMath(t) => {
                stack.last_mut().expect("frame").inlines.push(Inline::Math { tex: t.to_string(), display: false });
            }
            Event::DisplayMath(t) => {
                stack.last_mut().expect("frame").inlines.push(Inline::Math { tex: t.to_string(), display: true });
            }
            Event::SoftBreak => {
                let f = stack.last_mut().expect("frame");
                f.inlines.push(span(" ", style, &links));
            }
            Event::HardBreak => stack.last_mut().expect("frame").inlines.push(Inline::Break),
            Event::Rule => push_block!(stack, Block::Rule),
            Event::TaskListMarker(done) => {
                for f in stack.iter_mut().rev() {
                    if let FrameKind::Item { task } = &mut f.kind {
                        *task = Some(done);
                        break;
                    }
                }
            }
            Event::FootnoteReference(label) => {
                let f = stack.last_mut().expect("frame");
                f.inlines.push(Inline::Span(Span {
                    text: note_number(&label).to_string(),
                    style: SpanStyle { sup: true, ..style },
                    link: Some(format!("#fn-{label}")),
                }));
            }
            Event::Html(h) => {
                if is_page_break(&h) {
                    push_block!(stack, Block::PageBreak);
                } else {
                    let f = stack.last_mut().expect("frame");
                    f.inlines.push(Inline::Span(Span { text: h.to_string(), style, link: None }));
                    flush_html(&mut stack);
                }
            }
            Event::InlineHtml(h) => {
                let lower = h.to_ascii_lowercase();
                let f = stack.last_mut().expect("frame");
                if lower.starts_with("<br") {
                    f.inlines.push(Inline::Break);
                } else if lower.starts_with("<mark") {
                    style.mark = true;
                } else if lower.starts_with("</mark") {
                    style.mark = false;
                } else if lower.starts_with("<sup") {
                    style.sup = true;
                } else if lower.starts_with("</sup") {
                    style.sup = false;
                } else if lower.starts_with("<sub") {
                    style.sub = true;
                } else if lower.starts_with("</sub") {
                    style.sub = false;
                } else if lower.starts_with("<img") {
                    for (url, alt) in html_images(&h) {
                        f.inlines.push(Inline::Image { url, alt, link: links.last().cloned() });
                    }
                }
            }
        }
        if stack.len() == 1 && !stack[0].blocks.is_empty() {
            out.extend(stack[0].blocks.drain(..).map(|block| TopBlock { offset: top_offset, block }));
        }
    }
    fix_list_tightness(markdown, &mut out);
    out
}

/// Turns the raw HTML collected in the current frame into image and text blocks.
fn flush_html(stack: &mut [Frame]) {
    let f = stack.last_mut().expect("frame");
    let html: String = f
        .inlines
        .drain(..)
        .filter_map(|i| match i {
            Inline::Span(s) => Some(s.text),
            _ => None,
        })
        .collect();
    for (url, alt) in html_images(&html) {
        f.blocks.push(Block::Paragraph(vec![Inline::Image { url, alt, link: None }]));
    }
    let text = strip_tags(&html);
    let text = text.trim();
    if !text.is_empty() {
        f.blocks.push(Block::Html(text.to_string()));
    }
}

/// A paragraph's images become their own blocks; the text around them stays
/// in paragraphs.
fn split_images(inlines: Vec<Inline>) -> Vec<Block> {
    let mut out = Vec::new();
    let mut cur = Vec::new();
    for i in inlines {
        match i {
            Inline::Image { .. } => {
                if cur.iter().any(|c: &Inline| {
                    matches!(c, Inline::Span(s) if !s.text.trim().is_empty()) || matches!(c, Inline::Math { .. })
                }) {
                    out.push(Block::Paragraph(std::mem::take(&mut cur)));
                } else {
                    cur.clear();
                }
                out.push(Block::Paragraph(vec![i]));
            }
            other => cur.push(other),
        }
    }
    if cur.iter().any(|c| {
        matches!(c, Inline::Span(s) if !s.text.trim().is_empty()) || matches!(c, Inline::Break | Inline::Math { .. })
    }) {
        out.push(Block::Paragraph(cur));
    }
    out
}

/// pulldown-cmark doesn't report list tightness as an event: a list is loose
/// when its items are separated by blank lines. Close enough for spacing.
fn fix_list_tightness(_markdown: &str, blocks: &mut [TopBlock]) {
    fn walk(b: &mut Block) {
        match b {
            Block::List { tight, items, .. } => {
                *tight =
                    items.iter().all(|i| i.blocks.iter().filter(|b| matches!(b, Block::Paragraph(_))).count() <= 1);
                for i in items {
                    i.blocks.iter_mut().for_each(walk);
                }
            }
            Block::Quote(bs) | Block::Footnote { blocks: bs, .. } => bs.iter_mut().for_each(walk),
            _ => {}
        }
    }
    blocks.iter_mut().for_each(|t| walk(&mut t.block));
}

/// Plain text of inline content.
#[cfg(test)]
pub fn inline_text(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(|i| match i {
            Inline::Span(s) => s.text.as_str(),
            Inline::Break => " ",
            Inline::Image { alt, .. } => alt.as_str(),
            Inline::Math { tex, .. } => tex.as_str(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocks(md: &str) -> Vec<Block> {
        parse(md).into_iter().map(|t| t.block).collect()
    }

    #[test]
    fn basic_structure_and_offsets() {
        let md = "---\ntitle: x\n---\n# Title\n\nSome **bold** and [link](https://a.b).\n\n- one\n- [x] two\n\n```rust\nfn a() {}\n```\n";
        let top = parse(md);
        assert_eq!(top.len(), 4, "{top:#?}");
        assert_eq!(top[0].offset, md.find("# Title").unwrap());
        assert!(matches!(&top[0].block, Block::Heading { level: 1, .. }));
        let Block::Paragraph(inl) = &top[1].block else { panic!() };
        assert_eq!(inline_text(inl), "Some bold and link.");
        assert!(inl.iter().any(|i| matches!(i, Inline::Span(s) if s.style.bold && s.text == "bold")));
        assert!(inl.iter().any(|i| matches!(i, Inline::Span(s) if s.link.as_deref() == Some("https://a.b"))));
        let Block::List { items, tight, .. } = &top[2].block else { panic!() };
        assert!(*tight);
        assert_eq!(items[1].task, Some(true));
        assert_eq!(top[3].block, Block::Code { lang: "rust".into(), text: "fn a() {}".into() });
    }

    #[test]
    fn nested_lists_quotes_tables() {
        let b = blocks("> quote\n>\n> - a\n>   - b\n\n| h1 | h2 |\n|:--|--:|\n| x | **y** |\n");
        let Block::Quote(inner) = &b[0] else { panic!("{b:#?}") };
        assert!(matches!(inner[1], Block::List { .. }));
        let Block::Table { aligns, head, rows } = &b[1] else { panic!() };
        assert_eq!(aligns, &vec![Alignment::Left, Alignment::Right]);
        assert_eq!(inline_text(&head[0]), "h1");
        assert_eq!(inline_text(&rows[0][1]), "y");
    }

    #[test]
    fn images_page_breaks_and_html() {
        let b = blocks("Text ![alt](a.png) more\n\n<!-- pagebreak -->\n\n<p align=\"center\"><img src=\"logo.png\" alt=\"Logo\"></p>\n\n***\n");
        assert!(matches!(&b[0], Block::Paragraph(i) if inline_text(i) == "Text "));
        assert!(matches!(&b[1], Block::Paragraph(i) if matches!(&i[0], Inline::Image { url, .. } if url == "a.png")));
        assert!(matches!(&b[2], Block::Paragraph(i) if inline_text(i) == " more"));
        assert_eq!(b[3], Block::PageBreak);
        assert!(
            matches!(&b[4], Block::Paragraph(i) if matches!(&i[0], Inline::Image { url, alt, .. } if url == "logo.png" && alt == "Logo"))
        );
        assert_eq!(b[5], Block::Rule);
    }

    #[test]
    fn math() {
        let b = blocks("Euler: $e^{i\\pi} + 1 = 0$.\n\n$$\\int_0^1 x\\,dx$$\n");
        let Block::Paragraph(i) = &b[0] else { panic!("{b:#?}") };
        assert!(i.iter().any(|x| matches!(x, Inline::Math { display: false, tex } if tex == "e^{i\\pi} + 1 = 0")));
        let Block::Paragraph(d) = &b[1] else { panic!("{b:#?}") };
        assert!(matches!(&d[..], [Inline::Math { display: true, .. }]));
        // A lone dollar amount is not math.
        let b = blocks("It costs $5.\n");
        let Block::Paragraph(i) = &b[0] else { panic!() };
        assert!(!i.iter().any(|x| matches!(x, Inline::Math { .. })));
    }

    #[test]
    fn footnotes() {
        let b = blocks("Hi[^1].\n\n[^1]: Note.\n");
        let Block::Paragraph(i) = &b[0] else { panic!() };
        assert!(i.iter().any(|x| matches!(x, Inline::Span(s) if s.link.as_deref() == Some("#fn-1"))));
        assert!(matches!(&b[1], Block::Footnote { label, .. } if label == "1"));
    }

    #[test]
    fn footnotes_are_numbered_by_first_reference() {
        let b = blocks("A[^z] b[^a] c[^z].\n\n[^a]: First def.\n\n[^z]: Second def.\n");
        let Block::Paragraph(i) = &b[0] else { panic!() };
        let refs: Vec<&str> = i
            .iter()
            .filter_map(|x| match x {
                Inline::Span(s) if s.style.sup => Some(s.text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(refs, ["1", "2", "1"]);
        assert!(matches!(&b[1], Block::Footnote { label, number: 2, .. } if label == "a"));
        assert!(matches!(&b[2], Block::Footnote { label, number: 1, .. } if label == "z"));
    }
}
