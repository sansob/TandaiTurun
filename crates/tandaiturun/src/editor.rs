//! The Markdown source editor: syntax colouring, the formatting commands,
//! list continuation on Enter, and find & replace in the source.

use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use egui::{Color32, FontId, Galley, Id, Stroke};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;
use tandaiturun_core::format::{self, Edit, LineKind};
use tandaiturun_core::parser_options;

/// A command of the formatting toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Bold,
    Italic,
    Strike,
    Code,
    Heading(u8),
    Paragraph,
    Bullet,
    Numbered,
    Task,
    Quote,
    Link,
    Image,
    Table,
    CodeBlock,
    Rule,
    PageBreak,
    Indent,
    Outdent,
}

pub fn apply_format(text: &str, sel: Range<usize>, f: Format) -> Edit {
    match f {
        Format::Bold => format::toggle_wrap(text, sel, "**"),
        Format::Italic => format::toggle_wrap(text, sel, "*"),
        Format::Strike => format::toggle_wrap(text, sel, "~~"),
        Format::Code => format::toggle_wrap(text, sel, "`"),
        Format::Heading(l) => format::set_line_kind(text, sel, LineKind::Heading(l)),
        Format::Paragraph => format::set_line_kind(text, sel, LineKind::Paragraph),
        Format::Bullet => format::set_line_kind(text, sel, LineKind::Bullet),
        Format::Numbered => format::set_line_kind(text, sel, LineKind::Numbered),
        Format::Task => format::set_line_kind(text, sel, LineKind::Task),
        Format::Quote => format::set_line_kind(text, sel, LineKind::Quote),
        Format::Link => format::insert_link(text, sel, false),
        Format::Image => format::insert_link(text, sel, true),
        Format::Table => format::table(text, sel),
        Format::CodeBlock => format::code_block(text, sel),
        Format::Rule => format::horizontal_rule(text, sel),
        Format::PageBreak => format::page_break(text, sel),
        Format::Indent => format::indent(text, sel, false),
        Format::Outdent => format::indent(text, sel, true),
    }
}

/// Source colours.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Class {
    Plain,
    Heading,
    Emph,
    Strong,
    Strike,
    Code,
    Link,
    Url,
    Marker,
    Quote,
    Muted,
}

fn class_format(c: Class, size: f32, dark: bool) -> TextFormat {
    let pick = |light: (u8, u8, u8), dk: (u8, u8, u8)| {
        let (r, g, b) = if dark { dk } else { light };
        Color32::from_rgb(r, g, b)
    };
    let text = pick((0x24, 0x29, 0x2E), (0xD4, 0xD4, 0xD4));
    let mut f = TextFormat {
        font_id: FontId::monospace(size),
        color: text,
        line_height: Some(size * 1.5),
        ..Default::default()
    };
    match c {
        Class::Plain => {}
        Class::Heading => f.color = pick((0x0B, 0x5C, 0xAD), (0x6C, 0xB2, 0xFF)),
        Class::Emph => f.italics = true,
        Class::Strong => f.color = pick((0x00, 0x00, 0x00), (0xFF, 0xFF, 0xFF)),
        Class::Strike => {
            f.color = pick((0x8A, 0x8A, 0x8A), (0x8A, 0x8A, 0x8A));
            f.strikethrough = Stroke::new(1.0, f.color);
        }
        Class::Code => f.color = pick((0xB3, 0x1D, 0x28), (0xFF, 0x9E, 0x80)),
        Class::Link => f.color = pick((0x14, 0x73, 0xE6), (0x6C, 0xB2, 0xFF)),
        Class::Url => f.color = pick((0x6A, 0x73, 0x7D), (0x8B, 0x94, 0x9E)),
        Class::Marker => f.color = crate::theme::BRAND,
        Class::Quote => f.color = pick((0x2E, 0x7D, 0x32), (0x81, 0xC7, 0x84)),
        Class::Muted => f.color = pick((0x8A, 0x8A, 0x8A), (0x7A, 0x7A, 0x7A)),
    }
    f
}

/// Colours the Markdown source.
pub fn highlight(text: &str, size: f32, dark: bool, marks: &[Range<usize>], current: Option<usize>) -> LayoutJob {
    let mut cls = vec![Class::Plain; text.len()];
    let set = |r: Range<usize>, c: Class, cls: &mut Vec<Class>| {
        let end = r.end.min(cls.len());
        for x in &mut cls[r.start.min(end)..end] {
            *x = c;
        }
    };
    for (ev, r) in Parser::new_ext(text, parser_options()).into_offset_iter() {
        match ev {
            Event::Start(Tag::Heading { .. }) => set(r, Class::Heading, &mut cls),
            Event::Start(Tag::Emphasis) => set(r, Class::Emph, &mut cls),
            Event::Start(Tag::Strong) => set(r, Class::Strong, &mut cls),
            Event::Start(Tag::Strikethrough) => set(r, Class::Strike, &mut cls),
            Event::Start(Tag::BlockQuote(_)) => {
                let mut at = r.start;
                for line in text[r.clone()].split_inclusive('\n') {
                    let lead = line.len() - line.trim_start().len();
                    if line.trim_start().starts_with('>') {
                        set(at + lead..at + lead + 1, Class::Quote, &mut cls);
                    }
                    at += line.len();
                }
            }
            Event::Start(Tag::Item) => {
                let s = &text[r.clone()];
                let n = s.find(' ').unwrap_or(0).min(s.len());
                set(r.start..r.start + n, Class::Marker, &mut cls);
            }
            Event::TaskListMarker(_) => set(r, Class::Marker, &mut cls),
            Event::Start(Tag::Link { .. }) | Event::Start(Tag::Image { .. }) => {
                let s = &text[r.clone()];
                set(r.clone(), Class::Link, &mut cls);
                if let Some(i) = s.rfind("](") {
                    set(r.start + i + 1..r.end, Class::Url, &mut cls);
                }
            }
            Event::Code(_) => set(r, Class::Code, &mut cls),
            Event::Start(Tag::CodeBlock(_)) => {
                set(r.clone(), Class::Code, &mut cls);
                // The fences themselves are muted.
                let s = &text[r.clone()];
                if s.starts_with("```") || s.starts_with("~~~") {
                    let first = s.find('\n').unwrap_or(s.len());
                    set(r.start..r.start + first, Class::Muted, &mut cls);
                    let body = s.trim_end_matches('\n');
                    if let Some(last) = body.rfind('\n') {
                        if body[last + 1..].trim_start().starts_with(['`', '~']) {
                            set(r.start + last + 1..r.start + body.len(), Class::Muted, &mut cls);
                        }
                    }
                }
            }
            Event::Html(_) | Event::InlineHtml(_) => set(r, Class::Muted, &mut cls),
            Event::Start(Tag::MetadataBlock(_)) => set(r, Class::Muted, &mut cls),
            Event::Rule => set(r, Class::Marker, &mut cls),
            Event::FootnoteReference(_) => set(r, Class::Link, &mut cls),
            Event::End(TagEnd::Table) => {}
            _ => {}
        }
    }
    let mut job = LayoutJob::default();
    let mark_bg = if dark { Color32::from_rgb(0x6B, 0x55, 0x00) } else { Color32::from_rgb(0xFF, 0xE5, 0x8F) };
    let cur_bg = Color32::from_rgb(0xFF, 0x96, 0x32);
    // Cut points: class changes and search mark edges.
    let mut cuts: Vec<usize> = vec![0, text.len()];
    for (i, w) in cls.windows(2).enumerate() {
        if w[0] != w[1] {
            cuts.push(i + 1);
        }
    }
    for m in marks {
        cuts.push(m.start.min(text.len()));
        cuts.push(m.end.min(text.len()));
    }
    cuts.sort_unstable();
    cuts.dedup();
    let mut mi = 0;
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a >= b || !text.is_char_boundary(a) || !text.is_char_boundary(b) {
            continue;
        }
        let mut f = class_format(cls[a], size, dark);
        while mi < marks.len() && marks[mi].end <= a {
            mi += 1;
        }
        if let Some(m) = marks.get(mi) {
            if m.start <= a && a < m.end {
                f.background = if current == Some(mi) { cur_bg } else { mark_bg };
            }
        }
        job.append(&text[a..b], 0.0, f);
    }
    if text.is_empty() {
        job.append("", 0.0, class_format(Class::Plain, size, dark));
    }
    job
}

#[derive(Default)]
pub struct Editor {
    cache: Option<(u64, Arc<Galley>)>,
    /// The selection (byte offsets) as of the last frame.
    pub cursor: Range<usize>,
    /// A selection to apply on the next frame (after a command changed the text).
    pending: Option<Range<usize>>,
    scroll_to_cursor: bool,
    pub focus_next: bool,
    /// Source search marks and the current one.
    pub marks: Vec<Range<usize>>,
    pub current_mark: Option<usize>,
}

pub struct EditorOutput {
    pub changed: bool,
    pub cursor_moved: bool,
}

fn char_to_byte(text: &str, ch: usize) -> usize {
    text.char_indices().nth(ch).map_or(text.len(), |(b, _)| b)
}

/// `r` limited to the text and moved onto character boundaries: the
/// selection can be a frame old, from before the text changed under it.
pub fn snap(text: &str, r: Range<usize>) -> Range<usize> {
    let floor = |mut i: usize| {
        i = i.min(text.len());
        while !text.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    let (a, b) = (floor(r.start), floor(r.end));
    a.min(b)..b.max(a)
}

fn byte_to_char(text: &str, b: usize) -> usize {
    text[..snap(text, b..b).start].chars().count()
}

impl Editor {
    pub fn id(uid: u64) -> Id {
        Id::new(("editor", uid))
    }

    /// Applies an edit made outside the widget (toolbar, menu).
    pub fn apply(&mut self, text: &mut String, edit: Edit) {
        *text = edit.text;
        self.cursor = snap(text, edit.selection.clone());
        self.pending = Some(edit.selection);
        self.focus_next = true;
        self.scroll_to_cursor = true;
    }

    pub fn format(&mut self, text: &mut String, f: Format) {
        let e = apply_format(text, self.selection(text), f);
        self.apply(text, e);
    }

    /// The current selection, valid for `text`.
    pub fn selection(&self, text: &str) -> Range<usize> {
        snap(text, self.cursor.clone())
    }

    /// Forgets the selection and search marks (the text was replaced).
    pub fn reset(&mut self, text: &str) {
        self.cursor = snap(text, self.cursor.clone());
        self.pending = None;
        self.marks.clear();
        self.current_mark = None;
    }

    /// Selects a range and scrolls it into view.
    pub fn select(&mut self, range: Range<usize>) {
        self.pending = Some(range);
        self.scroll_to_cursor = true;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, uid: u64, text: &mut String, size: f32, dark: bool) -> EditorOutput {
        let id = Self::id(uid);
        let ctx = ui.ctx().clone();
        let focused = ctx.memory(|m| m.has_focus(id));
        let mut changed = false;

        // Keys the editor handles itself, before the TextEdit sees them.
        if focused {
            let sel = self.selection(text);
            let mut edit: Option<Edit> = None;
            ctx.input_mut(|i| {
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                    // Typing replaces the selection first, Enter included.
                    let mut t = text.clone();
                    t.replace_range(sel.clone(), "");
                    edit = Some(format::continue_list(&t, sel.start).unwrap_or_else(|| {
                        t.insert(sel.start, '\n');
                        Edit { text: t, selection: sel.start + 1..sel.start + 1 }
                    }));
                } else if i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab) {
                    edit = Some(format::indent(text, sel.clone(), true));
                } else if i.key_pressed(egui::Key::Tab) && !i.modifiers.any() {
                    let multi_line = text[sel.clone()].contains('\n');
                    let in_list =
                        !matches!(format::kind_at(text, sel.start), LineKind::Paragraph | LineKind::Heading(_));
                    if multi_line || in_list {
                        i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
                        edit = Some(format::indent(text, sel.clone(), false));
                    }
                }
                let cmd = egui::Modifiers::COMMAND;
                let fmt = |i: &mut egui::InputState, key| i.consume_key(cmd, key);
                // egui-winit turns Ctrl+X / Ctrl+C into Cut / Copy events even
                // with Shift held, so Ctrl+Shift+X / C arrive as those.
                let shifted = |i: &mut egui::InputState, ev: egui::Event| {
                    let on = i.modifiers.command && i.modifiers.shift;
                    let at = i.events.iter().position(|e| *e == ev).filter(|_| on);
                    at.map(|p| i.events.remove(p)).is_some()
                };
                let f = if fmt(i, egui::Key::B) {
                    Some(Format::Bold)
                } else if fmt(i, egui::Key::I) {
                    Some(Format::Italic)
                } else if fmt(i, egui::Key::K) {
                    Some(Format::Link)
                } else if i.consume_key(cmd | egui::Modifiers::SHIFT, egui::Key::X) || shifted(i, egui::Event::Cut) {
                    Some(Format::Strike)
                } else if i.consume_key(cmd | egui::Modifiers::SHIFT, egui::Key::C) || shifted(i, egui::Event::Copy) {
                    Some(Format::Code)
                } else {
                    [
                        egui::Key::Num1,
                        egui::Key::Num2,
                        egui::Key::Num3,
                        egui::Key::Num4,
                        egui::Key::Num5,
                        egui::Key::Num6,
                    ]
                    .iter()
                    .position(|k| i.consume_key(cmd | egui::Modifiers::ALT, *k))
                    .map(|l| Format::Heading(l as u8 + 1))
                };
                if let Some(f) = f {
                    edit = Some(apply_format(text, sel.clone(), f));
                }
            });
            if let Some(e) = edit {
                self.apply(text, e);
                changed = true;
            }
        }

        if let Some(sel) = self.pending.take() {
            let mut state = egui::text_edit::TextEditState::load(&ctx, id).unwrap_or_default();
            let (a, b) = (byte_to_char(text, sel.start), byte_to_char(text, sel.end));
            state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))));
            state.store(&ctx, id);
            self.cursor = sel;
        }

        let marks = self.marks.clone();
        let current = self.current_mark;
        let cache = &mut self.cache;
        let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap: f32| -> Arc<Galley> {
            let s = buf.as_str();
            let mut h = std::collections::hash_map::DefaultHasher::new();
            s.hash(&mut h);
            wrap.to_bits().hash(&mut h);
            size.to_bits().hash(&mut h);
            dark.hash(&mut h);
            marks.hash(&mut h);
            current.hash(&mut h);
            let key = h.finish();
            if let Some((k, g)) = cache.as_ref() {
                if *k == key {
                    return g.clone();
                }
            }
            let mut job = highlight(s, size, dark, &marks, current);
            job.wrap.max_width = wrap;
            let g = ui.fonts_mut(|f| f.layout_job(job));
            *cache = Some((key, g.clone()));
            g
        };
        // Before the widget runs, so a key sent along with the request (menu
        // Undo/Redo) already lands in a focused editor this frame.
        if self.focus_next {
            ui.memory_mut(|m| m.request_focus(id));
            self.focus_next = false;
        }
        let out = egui::TextEdit::multiline(text)
            .id(id)
            .font(FontId::monospace(size))
            .lock_focus(true)
            .desired_width(f32::INFINITY)
            .desired_rows(40)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin { left: 20, right: 14, top: 4, bottom: 40 })
            .layouter(&mut layouter)
            .show(ui);
        changed |= out.response.changed();
        let mut cursor_moved = false;
        if let Some(r) = out.cursor_range {
            let range = r.as_sorted_char_range();
            let sel = char_to_byte(text, range.start.0)..char_to_byte(text, range.end.0);
            if sel != self.cursor {
                cursor_moved = true;
            }
            self.cursor = sel;
            if self.scroll_to_cursor {
                let rect = out.galley.pos_from_cursor(r.primary).translate(out.galley_pos.to_vec2());
                ui.scroll_to_rect(rect.expand(40.0), Some(egui::Align::Center));
                self.scroll_to_cursor = false;
            }
        }
        EditorOutput { changed, cursor_moved }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_covers_all_text_once() {
        let md =
            "# Title\n\nSome *em* and **strong** `code` [link](http://x).\n\n- item\n> quote\n\n```rs\nfn a(){}\n```\n";
        let marks = vec![std::ops::Range { start: 5, end: 9 }];
        let job = highlight(md, 14.0, false, &marks, Some(0));
        assert_eq!(job.text, md);
        let mut end = 0;
        for s in &job.sections {
            assert_eq!(s.byte_range.start.0, end);
            end = s.byte_range.end.0;
        }
        assert_eq!(end, md.len());
        // The search mark has a background.
        let at = job.sections.iter().find(|s| s.byte_range.start.0 == 5).unwrap();
        assert_ne!(at.format.background, Color32::TRANSPARENT);
    }

    #[test]
    fn format_commands_dispatch() {
        assert_eq!(apply_format("a", 0..1, Format::Bold).text, "**a**");
        assert_eq!(apply_format("a", 0..1, Format::Heading(2)).text, "## a");
        assert_eq!(apply_format("a", 0..1, Format::Task).text, "- [ ] a");
    }
}
