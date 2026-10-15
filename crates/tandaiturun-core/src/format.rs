//! The editing commands behind the formatting toolbar and shortcuts.
//!
//! Every command takes the text and the selection (byte offsets, `start <=
//! end`) and returns the new text with the selection to show afterwards, so
//! the editor widget only has to apply the result.

use std::ops::Range;

/// The result of an edit: the new text and the new selection (byte offsets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub text: String,
    pub selection: Range<usize>,
}

fn clamp(text: &str, sel: Range<usize>) -> Range<usize> {
    let fix = |mut i: usize| {
        i = i.min(text.len());
        while !text.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    let (a, b) = (fix(sel.start), fix(sel.end));
    a.min(b)..a.max(b)
}

/// Bold, italic, strikethrough, inline code: wraps the selection in `marker`,
/// or removes the marker when the selection is already wrapped. With nothing
/// selected the markers are inserted with the cursor between them.
pub fn toggle_wrap(text: &str, sel: Range<usize>, marker: &str) -> Edit {
    let sel = clamp(text, sel);
    let m = marker.len();
    let inner = &text[sel.clone()];
    // The selection includes the markers: "**bold**".
    if inner.len() >= 2 * m && inner.starts_with(marker) && inner.ends_with(marker) {
        let mut out = String::with_capacity(text.len());
        out.push_str(&text[..sel.start]);
        out.push_str(&inner[m..inner.len() - m]);
        out.push_str(&text[sel.end..]);
        return Edit { text: out, selection: sel.start..sel.end - 2 * m };
    }
    // The markers sit right around the selection: **[bold]**.
    if sel.start >= m && text[..sel.start].ends_with(marker) && text[sel.end..].starts_with(marker) {
        // `*` inside `**`: only unwrap when it isn't part of a longer run.
        let longer = |c: Option<char>| marker.len() == 1 && c == marker.chars().next();
        if !(longer(text[..sel.start - m].chars().next_back()) || longer(text[sel.end + m..].chars().next())) {
            let mut out = String::with_capacity(text.len());
            out.push_str(&text[..sel.start - m]);
            out.push_str(inner);
            out.push_str(&text[sel.end + m..]);
            return Edit { text: out, selection: sel.start - m..sel.end - m };
        }
    }
    // Keep surrounding spaces of the selection outside the markers.
    let lead = inner.len() - inner.trim_start().len();
    let trail = if inner.trim().is_empty() { 0 } else { inner.len() - inner.trim_end().len() };
    let core = sel.start + lead..sel.end - trail;
    let mut out = String::with_capacity(text.len() + 2 * m);
    out.push_str(&text[..core.start]);
    out.push_str(marker);
    out.push_str(&text[core.clone()]);
    out.push_str(marker);
    out.push_str(&text[core.end..]);
    Edit { text: out, selection: core.start + m..core.end + m }
}

/// The byte range of the whole lines touched by the selection.
pub fn line_span(text: &str, sel: &Range<usize>) -> Range<usize> {
    let start = text[..sel.start].rfind('\n').map_or(0, |i| i + 1);
    // A selection that ends right after a newline doesn't include the next line.
    let end_probe = if sel.end > sel.start && text[..sel.end].ends_with('\n') { sel.end - 1 } else { sel.end };
    let end = text[end_probe..].find('\n').map_or(text.len(), |i| end_probe + i);
    start..end.max(start)
}

/// The kind of block a line can be turned into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Paragraph,
    Heading(u8),
    Bullet,
    Numbered,
    Task,
    Quote,
}

fn split_prefix(line: &str) -> (&str, &str, LineKind) {
    let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let (indent, rest) = line.split_at(indent_len);
    let hashes = rest.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) && rest[hashes..].starts_with(' ') {
        return (indent, &rest[hashes + 1..], LineKind::Heading(hashes as u8));
    }
    for (p, kind) in [("- [ ] ", LineKind::Task), ("- [x] ", LineKind::Task), ("- [X] ", LineKind::Task)] {
        if let Some(r) = rest.strip_prefix(p) {
            return (indent, r, kind);
        }
    }
    for p in ["- ", "* ", "+ "] {
        if let Some(r) = rest.strip_prefix(p) {
            return (indent, r, LineKind::Bullet);
        }
    }
    let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && (rest[digits..].starts_with(". ") || rest[digits..].starts_with(") ")) {
        return (indent, &rest[digits + 2..], LineKind::Numbered);
    }
    if let Some(r) = rest.strip_prefix("> ").or_else(|| rest.strip_prefix('>')) {
        return (indent, r, LineKind::Quote);
    }
    (indent, rest, LineKind::Paragraph)
}

/// The kind of the line the cursor is on.
pub fn kind_at(text: &str, at: usize) -> LineKind {
    let span = line_span(text, &clamp(text, at..at));
    split_prefix(&text[span]).2
}

/// Turns every selected line into `kind`; when all of them already are, they
/// go back to plain paragraphs (so the toolbar buttons toggle).
pub fn set_line_kind(text: &str, sel: Range<usize>, kind: LineKind) -> Edit {
    let sel = clamp(text, sel);
    let span = line_span(text, &sel);
    let lines: Vec<&str> = text[span.clone()].split('\n').collect();
    let all_already = lines.iter().filter(|l| !l.trim().is_empty()).all(|l| split_prefix(l).2 == kind)
        && lines.iter().any(|l| !l.trim().is_empty());
    let target = if all_already { LineKind::Paragraph } else { kind };
    let mut n = 0;
    let new_lines: Vec<String> = lines
        .iter()
        .map(|line| {
            if line.trim().is_empty() && lines.len() > 1 {
                return line.to_string();
            }
            let (indent, body, old) = split_prefix(line);
            let body = if old == LineKind::Task && target == LineKind::Task { &line[indent.len()..] } else { body };
            n += 1;
            let prefix = match target {
                LineKind::Paragraph => String::new(),
                LineKind::Heading(l) => format!("{} ", "#".repeat(l.clamp(1, 6) as usize)),
                LineKind::Bullet => "- ".into(),
                LineKind::Numbered => format!("{n}. "),
                LineKind::Task if old == LineKind::Task => String::new(),
                LineKind::Task => "- [ ] ".into(),
                LineKind::Quote => "> ".into(),
            };
            format!("{indent}{prefix}{body}")
        })
        .collect();
    let replacement = new_lines.join("\n");
    let mut out = String::with_capacity(text.len() + 16);
    out.push_str(&text[..span.start]);
    out.push_str(&replacement);
    out.push_str(&text[span.end..]);
    let new_end = span.start + replacement.len();
    // A collapsed cursor stays at the end of its (single) line.
    let selection = if sel.is_empty() { new_end..new_end } else { span.start..new_end };
    Edit { text: out, selection }
}

/// Turns the selection into a link and selects the URL placeholder.
pub fn insert_link(text: &str, sel: Range<usize>, image: bool) -> Edit {
    let sel = clamp(text, sel);
    let label = &text[sel.clone()];
    let looks_like_url = label.starts_with("http://") || label.starts_with("https://");
    let bang = if image { "!" } else { "" };
    let (inserted, url_range) = if looks_like_url {
        let s = format!("{bang}[]({label})");
        let at = bang.len() + 1;
        (s, at..at)
    } else {
        let label = if label.is_empty() && image { "alt text" } else { label };
        let s = format!("{bang}[{label}](https://)");
        let at = bang.len() + label.len() + 3;
        (s, at..at + "https://".len())
    };
    let mut out = String::with_capacity(text.len() + inserted.len());
    out.push_str(&text[..sel.start]);
    out.push_str(&inserted);
    out.push_str(&text[sel.end..]);
    Edit { text: out, selection: sel.start + url_range.start..sel.start + url_range.end }
}

/// Inserts a block (table, code fence, rule…) on its own lines at the cursor,
/// with blank lines around it, and selects `select` inside it if given.
pub fn insert_block(text: &str, sel: Range<usize>, block: &str, select: Option<&str>) -> Edit {
    let sel = clamp(text, sel);
    let before = &text[..sel.start];
    let after = &text[sel.end..];
    let lead = if before.is_empty() || before.ends_with("\n\n") {
        ""
    } else if before.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let trail = if after.starts_with("\n\n") {
        ""
    } else if after.starts_with('\n') || after.is_empty() {
        "\n"
    } else {
        "\n\n"
    };
    let start = sel.start + lead.len();
    let mut out = String::with_capacity(text.len() + block.len() + 4);
    out.push_str(before);
    out.push_str(lead);
    out.push_str(block);
    out.push_str(trail);
    out.push_str(after);
    let selection = match select.and_then(|s| block.find(s).map(|i| start + i..start + i + s.len())) {
        Some(r) => r,
        None => {
            let e = start + block.len();
            e..e
        }
    };
    Edit { text: out, selection }
}

/// A fenced code block around the selection (or an empty one).
pub fn code_block(text: &str, sel: Range<usize>) -> Edit {
    let sel = clamp(text, sel);
    let body = text[sel.clone()].trim_end_matches('\n').to_string();
    let block = format!("```\n{body}\n```");
    let mut e = insert_block(text, sel, &block, None);
    // Put the cursor right after the opening fence, where the language goes.
    let fence = e.text[..e.selection.end].rfind("```\n").map(|i| i + 3).unwrap_or(e.selection.end);
    let open = e.text[..fence].rfind("```").map(|i| i + 3).unwrap_or(fence);
    e.selection = open..open;
    e
}

/// A 3×2 table template with the first header cell selected.
pub fn table(text: &str, sel: Range<usize>) -> Edit {
    let block = "| Column 1 | Column 2 | Column 3 |\n| -------- | -------- | -------- |\n| Cell     | Cell     | Cell     |\n| Cell     | Cell     | Cell     |";
    insert_block(text, sel, block, Some("Column 1"))
}

pub fn horizontal_rule(text: &str, sel: Range<usize>) -> Edit {
    insert_block(text, sel.end..sel.end, "---", None)
}

pub fn page_break(text: &str, sel: Range<usize>) -> Edit {
    insert_block(text, sel.end..sel.end, crate::PAGE_BREAK, None)
}

/// What Enter does in a list: continues it (`- `, `2. `, `- [ ] `, `> `), or
/// ends it when the current item is empty. `None` means a plain newline.
pub fn continue_list(text: &str, cursor: usize) -> Option<Edit> {
    let cursor = clamp(text, cursor..cursor).start;
    let span = line_span(text, &(cursor..cursor));
    if cursor != span.end {
        return None;
    }
    let line = &text[span.clone()];
    let (indent, body, kind) = split_prefix(line);
    let prefix = match kind {
        LineKind::Bullet => line[indent.len()..indent.len() + 2].to_string(),
        LineKind::Task => "- [ ] ".to_string(),
        LineKind::Quote => "> ".to_string(),
        LineKind::Numbered => {
            let digits: String = line[indent.len()..].chars().take_while(|c| c.is_ascii_digit()).collect();
            let sep = &line[indent.len() + digits.len()..indent.len() + digits.len() + 1];
            format!("{}{sep} ", digits.parse::<u64>().unwrap_or(0) + 1)
        }
        _ => return None,
    };
    if body.trim().is_empty() {
        // An empty item ends the list: remove its marker.
        let mut out = String::with_capacity(text.len());
        out.push_str(&text[..span.start]);
        out.push_str(&text[span.end..]);
        return Some(Edit { text: out, selection: span.start..span.start });
    }
    let insert = format!("\n{indent}{prefix}");
    let mut out = String::with_capacity(text.len() + insert.len());
    out.push_str(&text[..cursor]);
    out.push_str(&insert);
    out.push_str(&text[cursor..]);
    let at = cursor + insert.len();
    Some(Edit { text: out, selection: at..at })
}

/// Indents (or with `outdent`, unindents) the selected lines by two spaces.
pub fn indent(text: &str, sel: Range<usize>, outdent: bool) -> Edit {
    let sel = clamp(text, sel);
    let span = line_span(text, &sel);
    let mut delta_first: isize = 0;
    let lines: Vec<String> = text[span.clone()]
        .split('\n')
        .enumerate()
        .map(|(i, l)| {
            let new = if outdent {
                let n = l.len() - l.trim_start_matches(' ').len();
                if let Some(rest) = l.strip_prefix('\t') {
                    rest.to_string()
                } else {
                    l[n.min(2)..].to_string()
                }
            } else {
                format!("  {l}")
            };
            if i == 0 {
                delta_first = new.len() as isize - l.len() as isize;
            }
            new
        })
        .collect();
    let replacement = lines.join("\n");
    let mut out = String::with_capacity(text.len() + 8);
    out.push_str(&text[..span.start]);
    out.push_str(&replacement);
    out.push_str(&text[span.end..]);
    let new_end = span.start + replacement.len();
    let selection = if sel.is_empty() {
        let at = (sel.start as isize + delta_first).max(span.start as isize) as usize;
        at..at
    } else {
        span.start..new_end
    };
    Edit { text: out, selection }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(text: &str, sel: Range<usize>, f: impl Fn(&str, Range<usize>) -> Edit) -> (String, String) {
        let e = f(text, sel);
        let s = e.text[e.selection.clone()].to_string();
        (e.text, s)
    }

    #[test]
    fn wrap_and_unwrap() {
        assert_eq!(
            apply("say hello now", 4..9, |t, s| toggle_wrap(t, s, "**")),
            ("say **hello** now".into(), "hello".into())
        );
        assert_eq!(
            apply("say **hello** now", 6..11, |t, s| toggle_wrap(t, s, "**")),
            ("say hello now".into(), "hello".into())
        );
        assert_eq!(
            apply("say **hello** now", 4..13, |t, s| toggle_wrap(t, s, "**")),
            ("say hello now".into(), "hello".into())
        );
        // The trailing space of a double-clicked word stays outside.
        assert_eq!(toggle_wrap("hello world", 0..6, "*").text, "*hello* world");
        // Italic inside bold is added, not mistaken for an unwrap.
        assert_eq!(toggle_wrap("**hello**", 2..7, "*").text, "***hello***");
        let e = toggle_wrap("ab", 1..1, "`");
        assert_eq!((e.text.as_str(), e.selection), ("a``b", 2..2));
    }

    #[test]
    fn line_kinds_toggle() {
        let t = "one\ntwo\nthree";
        assert_eq!(set_line_kind(t, 0..7, LineKind::Bullet).text, "- one\n- two\nthree");
        assert_eq!(set_line_kind("- one\n- two", 0..11, LineKind::Bullet).text, "one\ntwo");
        assert_eq!(set_line_kind("- one\n- two", 0..11, LineKind::Numbered).text, "1. one\n2. two");
        assert_eq!(set_line_kind("## Title", 3..3, LineKind::Heading(1)).text, "# Title");
        assert_eq!(set_line_kind("# Title", 3..3, LineKind::Heading(1)).text, "Title");
        assert_eq!(set_line_kind("  - [x] done", 0..0, LineKind::Bullet).text, "  - done");
        assert_eq!(set_line_kind("todo", 0..0, LineKind::Task).text, "- [ ] todo");
        assert_eq!(set_line_kind("a\n\nb", 0..4, LineKind::Quote).text, "> a\n\n> b");
        assert_eq!(kind_at("x\n> quoted", 5), LineKind::Quote);
    }

    #[test]
    fn links_and_blocks() {
        assert_eq!(
            apply("see docs", 4..8, |t, s| insert_link(t, s, false)),
            ("see [docs](https://)".into(), "https://".into())
        );
        let e = insert_link("https://x.org", 0..13, false);
        assert_eq!((e.text.as_str(), e.selection), ("[](https://x.org)", 1..1));
        assert_eq!(insert_link("", 0..0, true).text, "![alt text](https://)");
        assert_eq!(horizontal_rule("para", 4..4).text, "para\n\n---\n");
        let (t, s) = apply("a\n\nb", 1..1, table);
        assert!(t.starts_with("a\n\n| Column 1 |"), "{t}");
        assert!(t.ends_with("|\n\nb"), "{t}");
        assert_eq!(s, "Column 1");
        let e = code_block("x\nlet a = 1;", 2..12);
        assert_eq!(e.text, "x\n\n```\nlet a = 1;\n```\n");
        assert_eq!(&e.text[..e.selection.start], "x\n\n```");
    }

    #[test]
    fn enter_continues_lists() {
        let e = continue_list("- one", 5).unwrap();
        assert_eq!((e.text.as_str(), e.selection), ("- one\n- ", 8..8));
        assert_eq!(continue_list("  9) nine", 9).unwrap().text, "  9) nine\n  10) ");
        assert_eq!(continue_list("- [x] done", 10).unwrap().text, "- [x] done\n- [ ] ");
        assert_eq!(continue_list("- one\n- ", 8).unwrap().text, "- one\n");
        assert!(continue_list("plain", 5).is_none());
        assert!(continue_list("- one", 2).is_none());
    }

    #[test]
    fn indenting() {
        assert_eq!(indent("- a\n- b", 0..7, false).text, "  - a\n  - b");
        assert_eq!(indent("  - a\n- b", 0..9, true).text, "- a\n- b");
        let e = indent("- a", 3..3, false);
        assert_eq!(e.selection, 5..5);
    }

    #[test]
    fn selections_out_of_bounds_or_mid_char_are_clamped() {
        assert_eq!(toggle_wrap("é", 1..99, "*").text, "*é*");
    }
}
