//! Page layout: Markdown blocks → positioned text, boxes and images in one
//! continuous column, cut into pages at allowed break points.
//!
//! Coordinates are in screen points at the current zoom. The flow's origin is
//! the top-left corner of the first page's content area (inside the margins);
//! a page shows the slice `start..end` of the flow.

use crate::markdown::{self, Block, Inline, ListItem, SpanStyle};
use crate::theme::{PageColors, PageTheme};
use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Color32, FontFamily, FontId, Galley, Pos2, Rect, Stroke, Vec2};
use pulldown_cmark::Alignment;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DocFont {
    #[default]
    Sans,
    Serif,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PageSize {
    #[default]
    A4,
    Letter,
}

impl PageSize {
    pub const ALL: [PageSize; 2] = [PageSize::A4, PageSize::Letter];

    /// Size at 100 % in points (CSS pixels, 96 per inch).
    pub fn size(self) -> Vec2 {
        match self {
            PageSize::A4 => Vec2::new(794.0, 1123.0),
            PageSize::Letter => Vec2::new(816.0, 1056.0),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PageSize::A4 => "A4",
            PageSize::Letter => "Letter",
        }
    }

    /// Size in PDF points (72 per inch).
    pub fn pdf_points(self) -> (f32, f32) {
        match self {
            PageSize::A4 => (595.28, 841.89),
            PageSize::Letter => (612.0, 792.0),
        }
    }
}

/// Page margin at 100 %: 0.75 inch.
pub const MARGIN: f32 = 72.0;

/// Everything the layout depends on (and is cached by).
#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    pub page: PageSize,
    pub font: DocFont,
    /// Body text size at 100 %, in points.
    pub font_size: f32,
    pub theme: PageTheme,
    pub zoom: f32,
    pub base_dir: Option<PathBuf>,
    pub pixels_per_point: f32,
    /// Load images from the internet (the reader allowed it).
    pub allow_remote: bool,
    /// How many fonts egui has: system fonts are added as documents need them.
    pub fonts: usize,
}

/// The number of fonts installed in `ctx`, for [`Params::fonts`].
pub fn font_count(ctx: &egui::Context) -> usize {
    ctx.fonts(|f| f.definitions().font_data.len())
}

#[derive(Clone)]
pub struct TextPrim {
    pub pos: Pos2,
    pub galley: Arc<Galley>,
    /// Byte ranges of `galley.job.text` that are links.
    pub links: Vec<(Range<usize>, String)>,
    /// List markers are left out of selection and search.
    pub selectable: bool,
}

impl TextPrim {
    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.pos, self.galley.size())
    }
}

#[derive(Clone)]
pub enum Prim {
    Text(TextPrim),
    Rect {
        rect: Rect,
        fill: Color32,
        stroke: Stroke,
        radius: f32,
    },
    Line {
        a: Pos2,
        b: Pos2,
        stroke: Stroke,
    },
    Image {
        rect: Rect,
        uri: String,
        link: Option<String>,
    },
    /// An image that isn't shown (missing, remote, still loading).
    Placeholder {
        rect: Rect,
        label: String,
        /// An internet image waiting for permission: clicking it allows them.
        remote: bool,
    },
    Check {
        rect: Rect,
        checked: bool,
    },
}

impl Prim {
    /// Vertical extent in the flow.
    pub fn y_range(&self) -> (f32, f32) {
        let r = match self {
            Prim::Text(t) => t.rect(),
            Prim::Rect { rect, .. }
            | Prim::Image { rect, .. }
            | Prim::Placeholder { rect, .. }
            | Prim::Check { rect, .. } => *rect,
            Prim::Line { a, b, .. } => Rect::from_two_pos(*a, *b),
        };
        (r.top(), r.bottom())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageSpan {
    pub start: f32,
    pub end: f32,
}

pub struct Layout {
    pub params: Params,
    pub prims: Vec<Prim>,
    pub pages: Vec<PageSpan>,
    /// Prim indices drawn on each page.
    pub page_prims: Vec<Vec<usize>>,
    /// Page size at this zoom.
    pub page_size: Vec2,
    pub margin: f32,
    /// Flow y of every heading, in document order (matches the outline).
    pub headings: Vec<f32>,
    /// Flow y of every anchor (`#heading`, `#fn-label`).
    pub anchors: HashMap<String, f32>,
    /// (source byte offset, flow y) of each top-level block, in order.
    pub blocks: Vec<(usize, f32)>,
    /// Some images are still loading: lay out again when they are ready.
    pub pending_images: bool,
    /// Internet images not shown because they weren't allowed.
    pub remote_blocked: usize,
}

impl Layout {
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// The page showing flow position `y`.
    pub fn page_of(&self, y: f32) -> usize {
        match self.pages.binary_search_by(|p| p.start.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Less)) {
            Ok(i) => i,
            Err(0) => 0,
            Err(i) => (i - 1).min(self.pages.len().saturating_sub(1)),
        }
    }

    /// Flow y of the block containing source offset `offset`.
    pub fn y_of_offset(&self, offset: usize) -> Option<f32> {
        let i = self.blocks.partition_point(|(o, _)| *o <= offset);
        self.blocks.get(i.saturating_sub(1)).map(|(_, y)| *y)
    }

    /// The source offset of the top-level block at flow `y`.
    pub fn offset_of_y(&self, y: f32) -> Option<usize> {
        let i = self.blocks.partition_point(|(_, by)| *by <= y + 1.0);
        self.blocks.get(i.saturating_sub(1)).map(|(o, _)| *o)
    }

    /// The index of the heading the reader is in at flow `y`.
    pub fn heading_at(&self, y: f32) -> Option<usize> {
        let i = self.headings.partition_point(|hy| *hy <= y + 2.0);
        i.checked_sub(1)
    }

    /// The text prim and character at a flow position. With `nearest`, a
    /// position beside or between lines picks the closest line.
    pub fn text_at(&self, page: usize, flow: Pos2, nearest: bool) -> Option<(usize, usize)> {
        let ids = self.page_prims.get(page)?;
        let mut best: Option<(f32, usize)> = None;
        for &i in ids {
            let Prim::Text(t) = &self.prims[i] else { continue };
            if !t.selectable {
                continue;
            }
            let r = t.rect();
            if r.expand(2.0).contains(flow) {
                let c = t.galley.cursor_from_pos(flow - t.pos);
                return Some((i, c.index.0));
            }
            if nearest {
                let dy = if flow.y < r.top() {
                    r.top() - flow.y
                } else if flow.y > r.bottom() {
                    flow.y - r.bottom()
                } else {
                    0.0
                };
                let dx = if flow.x < r.left() {
                    r.left() - flow.x
                } else if flow.x > r.right() {
                    flow.x - r.right()
                } else {
                    0.0
                };
                let d = dy * 4.0 + dx;
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, i));
                }
            }
        }
        let (_, i) = best?;
        let Prim::Text(t) = &self.prims[i] else { return None };
        let local = flow - t.pos;
        let clamped = Vec2::new(local.x, local.y.clamp(0.0, (t.galley.size().y - 1.0).max(0.0)));
        Some((i, t.galley.cursor_from_pos(clamped).index.0))
    }

    /// Whether a flow position is over an internet image waiting for permission.
    pub fn remote_placeholder_at(&self, page: usize, flow: Pos2) -> bool {
        self.page_prims.get(page).is_some_and(|ids| {
            ids.iter()
                .any(|&i| matches!(&self.prims[i], Prim::Placeholder { rect, remote: true, .. } if rect.contains(flow)))
        })
    }

    /// The link under a flow position, if any.
    pub fn link_at(&self, page: usize, flow: Pos2) -> Option<&str> {
        for &i in self.page_prims.get(page)? {
            match &self.prims[i] {
                Prim::Text(t) if !t.links.is_empty() && t.rect().contains(flow) => {
                    let local = flow - t.pos;
                    let c = t.galley.cursor_from_pos(local).index.0;
                    // Only when the pointer is over a glyph, not past the end of a row.
                    let over_glyph = char_rects(&t.galley, c..c + 1).iter().any(|r| r.contains(local.to_pos2()));
                    if !over_glyph {
                        continue;
                    }
                    let byte = char_to_byte(&t.galley.job.text, c);
                    if let Some((_, url)) = t.links.iter().find(|(r, _)| r.contains(&byte)) {
                        return Some(url);
                    }
                }
                Prim::Image { rect, link: Some(url), .. } if rect.contains(flow) => return Some(url),
                _ => {}
            }
        }
        None
    }
}

pub fn char_to_byte(text: &str, ch: usize) -> usize {
    text.char_indices().nth(ch).map_or(text.len(), |(b, _)| b)
}

pub fn byte_to_char(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].chars().count()
}

/// Rectangles (galley coordinates) covering a character range, one per row.
pub fn char_rects(galley: &Galley, range: Range<usize>) -> Vec<Rect> {
    let mut out = Vec::new();
    let mut ci = 0;
    for row in &galley.rows {
        let n = row.glyphs.len();
        let (a, b) = (range.start.max(ci), range.end.min(ci + n));
        if a < b {
            let g0 = &row.glyphs[a - ci];
            let g1 = &row.glyphs[b - 1 - ci];
            let x0 = row.pos.x + g0.pos.x;
            let x1 = row.pos.x + g1.pos.x + g1.advance_width;
            out.push(Rect::from_min_max(Pos2::new(x0, row.pos.y), Pos2::new(x1, row.pos.y + row.size.y)));
        }
        ci += row.char_count_including_newline().0;
        if ci >= range.end {
            break;
        }
    }
    out
}

/// Turns a Markdown image URL into something egui's loaders can open.
/// Remote images are not fetched: a document shouldn't phone home when read.
pub enum ImageSource {
    Local(String),
    Remote,
    Missing,
}

pub fn resolve_image(url: &str, base_dir: Option<&Path>) -> ImageSource {
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("//") {
        return ImageSource::Remote;
    }
    let path_part = url.strip_prefix("file://").unwrap_or(url);
    let path_part = path_part.split(['?', '#']).next().unwrap_or("");
    let decoded = percent_decode(path_part);
    let p = Path::new(&decoded);
    let full = if p.is_absolute() {
        p.to_path_buf()
    } else {
        match base_dir {
            Some(d) => d.join(p),
            None => return ImageSource::Missing,
        }
    };
    if full.is_file() {
        let s = full.to_string_lossy().replace('\\', "/");
        let s = if s.starts_with('/') { format!("file://{s}") } else { format!("file:///{s}") };
        ImageSource::Local(s)
    } else {
        ImageSource::Missing
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A laid-out run of inline content: the job, its links and its formulas.
type InlineJob = (LayoutJob, Vec<(Range<usize>, String)>, Vec<MathSpot>);

/// STIX Two Math has smaller letters than Inter at the same size.
const MATH_SCALE: f32 = 1.12;

/// A formula in a text galley: (char index of its placeholder, formula, text size).
type MathSpot = (usize, std::sync::Arc<crate::math::Formula>, f32);

struct Ctx {
    tight: bool,
    color: Color32,
    size_factor: f32,
    list_depth: usize,
}

struct Builder<'a> {
    egui: &'a egui::Context,
    p: &'a Params,
    c: PageColors,
    z: f32,
    s: f32,
    content_h: f32,
    prims: Vec<Prim>,
    breaks: Vec<f32>,
    forced: Vec<f32>,
    y: f32,
    pending: f32,
    fresh: bool,
    keep_next: bool,
    kept_at: Option<f32>,
    headings: Vec<f32>,
    heading_anchors: Vec<String>,
    anchors: HashMap<String, f32>,
    blocks: Vec<(usize, f32)>,
    top_mark: Option<usize>,
    pending_images: bool,
    remote_blocked: usize,
}

impl Builder<'_> {
    fn family(&self, bold: bool, italic: bool) -> FontFamily {
        let serif = self.p.font == crate::layout::DocFont::Serif;
        let name = match (serif, bold, italic) {
            (false, false, false) => return FontFamily::Proportional,
            (false, true, false) => "bold",
            (false, false, true) => "italic",
            (false, true, true) => "bolditalic",
            (true, false, false) => "serif",
            (true, true, false) => "serif-bold",
            (true, false, true) => "serif-italic",
            (true, true, true) => "serif-bolditalic",
        };
        FontFamily::Name(name.into())
    }

    fn format(&self, st: SpanStyle, size: f32, color: Color32, link: bool, bold_all: bool) -> TextFormat {
        let mut size = size;
        let mut valign = Align::BOTTOM;
        if st.sup || st.sub {
            size *= 0.75;
            valign = if st.sup { Align::TOP } else { Align::BOTTOM };
        }
        let family = if st.code { FontFamily::Monospace } else { self.family(st.bold || bold_all, st.italic) };
        let font_size = if st.code { size * 0.88 } else { size };
        let color = if link { self.c.link } else { color };
        TextFormat {
            font_id: FontId::new(font_size, family),
            line_height: Some(size * 1.55),
            color,
            background: if st.code {
                self.c.code_bg
            } else if st.mark {
                self.c.mark
            } else {
                Color32::TRANSPARENT
            },
            strikethrough: if st.strike { Stroke::new(1.0 * self.z.max(0.5), color) } else { Stroke::NONE },
            underline: if link {
                Stroke::new((0.8 * self.z).max(0.5), self.c.link.gamma_multiply(0.5))
            } else {
                Stroke::NONE
            },
            valign,
            ..Default::default()
        }
    }

    fn inline_job(
        &self,
        inlines: &[Inline],
        size: f32,
        color: Color32,
        bold_all: bool,
        width: f32,
    ) -> (LayoutJob, Vec<(Range<usize>, String)>) {
        let (job, links, _) = self.inline_job_math(inlines, size, color, bold_all, width);
        (job, links)
    }

    /// Like `inline_job`, plus the formulas: (character index of their placeholder, formula, text size).
    fn inline_job_math(&self, inlines: &[Inline], size: f32, color: Color32, bold_all: bool, width: f32) -> InlineJob {
        let mut job = LayoutJob::default();
        let mut links: Vec<(Range<usize>, String)> = Vec::new();
        let mut maths: Vec<MathSpot> = Vec::new();
        for i in inlines {
            match i {
                Inline::Span(sp) => {
                    let start = job.text.len();
                    let fmt = self.format(sp.style, size, color, sp.link.is_some(), bold_all);
                    job.append(&sp.text, 0.0, fmt);
                    if let Some(url) = &sp.link {
                        match links.last_mut() {
                            Some((r, u)) if u == url && r.end == start => r.end = job.text.len(),
                            _ => links.push((start..job.text.len(), url.clone())),
                        }
                    }
                }
                Inline::Break => {
                    job.append("\n", 0.0, self.format(SpanStyle::default(), size, color, false, bold_all));
                }
                Inline::Image { alt, .. } => {
                    let fmt =
                        self.format(SpanStyle { italic: true, ..Default::default() }, size, self.c.muted, false, false);
                    job.append(&format!("[{alt}]"), 0.0, fmt);
                }
                Inline::Math { tex, display } => match crate::math::formula(self.egui, tex, *display, color) {
                    Ok(fm) => {
                        // A no-break space, with leading space up to the formula's width,
                        // holds its place in the line.
                        let mut fmt = self.format(SpanStyle::default(), size, color, false, false);
                        let space = self.egui.fonts_mut(|f| f.glyph_width(&fmt.font_id, '\u{a0}'));
                        let lead = (fm.width * size * MATH_SCALE - space).max(0.0);
                        let tall = (fm.height + fm.depth) * size * MATH_SCALE;
                        fmt.line_height = Some((size * 1.55).max(tall * 1.2 + size * 0.2));
                        maths.push((job.text.chars().count(), fm, size * MATH_SCALE));
                        job.append("\u{a0}", lead, fmt);
                    }
                    Err(_) => {
                        let fence = if *display { "$$" } else { "$" };
                        let st = SpanStyle { code: true, ..Default::default() };
                        job.append(
                            &format!("{fence}{tex}{fence}"),
                            0.0,
                            self.format(st, size, self.c.muted, false, false),
                        );
                    }
                },
            }
        }
        job.wrap.max_width = width.max(10.0);
        (job, links, maths)
    }

    /// Images for the formulas of a galley placed at `pos`, sitting on their baseline.
    fn place_math(&mut self, pos: Pos2, galley: &Galley, maths: Vec<MathSpot>) {
        let spots: Vec<usize> = maths.iter().map(|m| m.0).collect();
        for (ci, fm, size) in maths {
            let mut seen = 0;
            for row in &galley.rows {
                let n = row.glyphs.len();
                if ci < seen + n {
                    let k = ci - seen;
                    let g = &row.glyphs[k];
                    // The gap starts where the previous glyph ends (the extra
                    // spacing may sit before or after the placeholder glyph).
                    let gap_start = if k > 0 {
                        let p = &row.glyphs[k - 1];
                        p.pos.x + p.advance_width
                    } else {
                        0.0
                    };
                    let gap_end = g.pos.x + g.advance_width;
                    let x = pos.x + row.pos.x + gap_start + ((gap_end - gap_start) - fm.width * size).max(0.0) / 2.0;
                    // The text baseline of the row: egui puts the taller placeholder's
                    // own baseline higher up, so take it from another glyph.
                    let other = (0..n).find(|&j| !spots.contains(&(seen + j)) && !row.glyphs[j].chr.is_whitespace());
                    let baseline = match other {
                        Some(j) => pos.y + row.pos.y + row.glyphs[j].pos.y,
                        None => {
                            let total = (fm.height + fm.depth) * size;
                            pos.y + row.pos.y + (row.size.y - total) / 2.0 + fm.height * size
                        }
                    };
                    let _ = g.pos.y;
                    let rect = Rect::from_min_size(
                        Pos2::new(x, baseline - fm.height * size),
                        Vec2::new(fm.width * size, (fm.height + fm.depth) * size),
                    );
                    self.prims.push(Prim::Image { rect, uri: fm.uri.clone(), link: None });
                    break;
                }
                seen += row.char_count_including_newline().0;
            }
        }
    }

    fn galley(&self, job: LayoutJob) -> Arc<Galley> {
        self.egui.fonts_mut(|f| f.layout_job(job))
    }

    fn add_break(&mut self, y: f32) {
        if self.keep_next {
            // The first break after a heading (and any other at the same
            // place, e.g. a list and its first item) is dropped.
            self.keep_next = false;
            self.kept_at = Some(y);
            return;
        }
        if self.kept_at.is_some_and(|k| (k - y).abs() < 0.5) {
            return;
        }
        self.kept_at = None;
        self.breaks.push(y);
    }

    /// Starts a block: collapses the margin with the previous block's and
    /// marks the block top as a place where a page may end.
    fn begin(&mut self, top: f32) {
        if self.fresh {
            self.fresh = false;
        } else if self.y > 0.0 {
            self.y += self.pending.max(top);
        }
        self.pending = 0.0;
        if let Some(offset) = self.top_mark.take() {
            self.blocks.push((offset, self.y));
        }
        self.add_break(self.y);
    }

    /// Places a text galley; a page may end after any of its rows.
    fn place_text(&mut self, x: f32, galley: Arc<Galley>, links: Vec<(Range<usize>, String)>, breakable: bool) -> f32 {
        let y0 = self.y;
        if breakable {
            let rows: Vec<f32> = galley.rows.iter().map(|r| y0 + r.pos.y + r.size.y).collect();
            for (k, b) in rows.iter().enumerate() {
                if k + 1 < rows.len() {
                    self.add_break(*b);
                }
            }
        }
        let h = galley.size().y;
        self.prims.push(Prim::Text(TextPrim { pos: Pos2::new(x, y0), galley, links, selectable: true }));
        h
    }

    fn blocks(&mut self, blocks: &[Block], x: f32, w: f32, cx: &mut Ctx) {
        for b in blocks {
            self.block(b, x, w, cx);
        }
    }

    fn block(&mut self, b: &Block, x: f32, w: f32, cx: &mut Ctx) {
        let s = self.s * cx.size_factor;
        match b {
            Block::Paragraph(inl) => {
                if let [Inline::Image { url, alt, link }] = inl.as_slice() {
                    self.image(url, alt, link.clone(), x, w);
                    return;
                }
                let visible: Vec<&Inline> =
                    inl.iter().filter(|i| !matches!(i, Inline::Span(sp) if sp.text.trim().is_empty())).collect();
                if let [Inline::Math { tex, display: true }] = visible.as_slice() {
                    if let Ok(fm) = crate::math::formula(self.egui, tex, true, cx.color) {
                        self.begin(0.9 * s);
                        let mut size = Vec2::new(fm.width, fm.height + fm.depth) * s * MATH_SCALE;
                        if size.x > w {
                            size *= w / size.x;
                        }
                        let rect = Rect::from_min_size(Pos2::new(x + (w - size.x) / 2.0, self.y), size);
                        self.prims.push(Prim::Image { rect, uri: fm.uri.clone(), link: None });
                        self.y += size.y;
                        self.pending = 0.9 * s;
                        return;
                    }
                }
                self.begin(0.8 * s);
                let (job, links, maths) = self.inline_job_math(inl, s, cx.color, false, w);
                let g = self.galley(job);
                self.place_math(Pos2::new(x, self.y), &g, maths);
                self.y += self.place_text(x, g, links, true);
                self.pending = if cx.tight { 0.25 * s } else { 0.8 * s };
            }
            Block::Heading { level, inlines } => {
                let (f, top) = match level {
                    1 => (2.0, 1.4),
                    2 => (1.5, 1.3),
                    3 => (1.25, 1.2),
                    4 => (1.05, 1.1),
                    5 => (0.95, 1.0),
                    _ => (0.9, 1.0),
                };
                self.begin(top * s);
                let idx = self.headings.len();
                self.headings.push(self.y);
                if let Some(a) = self.heading_anchors.get(idx) {
                    self.anchors.insert(a.clone(), self.y);
                }
                let color = if *level >= 6 { self.c.muted } else { self.c.heading };
                let (mut job, links, maths) = self.inline_job_math(inlines, s * f, color, true, w);
                for sec in &mut job.sections {
                    let lh = sec.format.line_height.unwrap_or(0.0);
                    sec.format.line_height = Some(if lh > s * f * 1.56 { lh } else { s * f * 1.3 });
                }
                let g = self.galley(job);
                self.place_math(Pos2::new(x, self.y), &g, maths);
                self.y += self.place_text(x, g, links, true);
                if *level <= 2 {
                    self.y += 0.3 * s;
                    let stroke = Stroke::new(1.0 * self.z.max(0.5), self.c.rule);
                    self.prims.push(Prim::Line { a: Pos2::new(x, self.y), b: Pos2::new(x + w, self.y), stroke });
                    self.y += 1.0;
                }
                self.pending = 0.55 * s;
                // Never end a page right after a heading.
                self.keep_next = true;
            }
            Block::Code { lang, text } if lang.eq_ignore_ascii_case("mermaid") => {
                match crate::mermaid::diagram(self.egui, text, self.c.dark, self.c.paper) {
                    Ok(d) => {
                        self.begin(0.8 * s);
                        let mut size = d.size * self.z;
                        if size.x > w {
                            size *= w / size.x;
                        }
                        let max_h = self.content_h * 0.95;
                        if size.y > max_h {
                            size *= max_h / size.y;
                        }
                        // Centred, like a figure.
                        let rect = Rect::from_min_size(Pos2::new(x + (w - size.x) / 2.0, self.y), size);
                        self.prims.push(Prim::Image { rect, uri: d.uri.clone(), link: None });
                        self.y += size.y;
                        self.pending = 0.8 * s;
                    }
                    Err(e) => {
                        let note = format!("⚠ Mermaid: {e}");
                        self.block(&Block::Html(note), x, w, cx);
                        self.block(&Block::Code { lang: String::new(), text: text.clone() }, x, w, cx);
                    }
                }
            }
            Block::Code { lang, text } => {
                self.begin(0.8 * s);
                let pad = 0.75 * s;
                let size = 0.86 * s;
                let mut job = if lang.is_empty() {
                    LayoutJob::simple(text.clone(), FontId::monospace(size), self.c.text, f32::INFINITY)
                } else {
                    let theme = if self.c.dark {
                        egui_extras::syntax_highlighting::CodeTheme::dark(size)
                    } else {
                        egui_extras::syntax_highlighting::CodeTheme::light(size)
                    };
                    let style = self.egui.global_style();
                    egui_extras::syntax_highlighting::highlight(self.egui, &style, &theme, text, lang)
                };
                for sec in &mut job.sections {
                    sec.format.font_id = FontId::monospace(size);
                    sec.format.line_height = Some(size * 1.5);
                    sec.format.background = Color32::TRANSPARENT;
                    if sec.format.color == Color32::PLACEHOLDER {
                        sec.format.color = self.c.text;
                    }
                }
                job.wrap.max_width = (w - 2.0 * pad).max(10.0);
                job.wrap.break_anywhere = false;
                let g = self.galley(job);
                let h = g.size().y + 2.0 * pad;
                let top = self.y;
                let rect = Rect::from_min_size(Pos2::new(x, top), Vec2::new(w, h));
                self.prims.push(Prim::Rect {
                    rect,
                    fill: self.c.code_bg,
                    stroke: Stroke::new(1.0, self.c.code_border),
                    radius: 4.0 * self.z,
                });
                self.y += pad;
                self.place_text(x + pad, g, vec![], true);
                self.y = top + h;
                self.pending = 0.8 * s;
            }
            Block::Quote(children) => {
                self.begin(0.8 * s);
                let top = self.y;
                let inset = 1.1 * s;
                let saved = cx.color;
                cx.color = self.c.muted;
                self.fresh = true;
                self.blocks(children, x + inset, w - inset, cx);
                cx.color = saved;
                self.fresh = false;
                let bar = Rect::from_min_max(Pos2::new(x, top), Pos2::new(x + 3.5 * self.z, self.y));
                self.prims.push(Prim::Rect {
                    rect: bar,
                    fill: self.c.quote_bar,
                    stroke: Stroke::NONE,
                    radius: 1.5 * self.z,
                });
                self.pending = 0.8 * s;
            }
            Block::List { start, tight, items } => {
                self.begin(0.8 * s);
                let saved_tight = cx.tight;
                cx.tight = *tight;
                cx.list_depth += 1;
                let last_number = start.map(|n| n + items.len().saturating_sub(1) as u64).unwrap_or(0);
                let indent = if start.is_some() {
                    (format!("{last_number}.").chars().count() as f32 * 0.6 + 1.0) * s
                } else {
                    1.5 * s
                };
                for (k, item) in items.iter().enumerate() {
                    if k > 0 {
                        self.y += if *tight { 0.25 * s } else { self.pending.max(0.5 * s) };
                        self.pending = 0.0;
                        self.add_break(self.y);
                    }
                    let marker = start.map(|n| format!("{}.", n + k as u64));
                    self.list_item(item, marker, x, w, indent, cx);
                }
                cx.list_depth -= 1;
                cx.tight = saved_tight;
                self.pending = if cx.list_depth > 0 { 0.25 * s } else { 0.8 * s };
            }
            Block::Table { aligns, head, rows } => self.table(aligns, head, rows, x, w, cx),
            Block::Rule => {
                self.begin(1.2 * s);
                let stroke = Stroke::new((1.5 * self.z).max(1.0), self.c.rule);
                self.prims.push(Prim::Line { a: Pos2::new(x, self.y), b: Pos2::new(x + w, self.y), stroke });
                self.y += 2.0;
                self.pending = 1.2 * s;
            }
            Block::PageBreak => {
                if self.y > 0.0 {
                    self.forced.push(self.y);
                    self.breaks.push(self.y);
                }
                self.pending = 0.0;
                self.fresh = true;
            }
            Block::Footnote { label, number, blocks } => {
                self.begin(0.6 * s);
                self.anchors.insert(format!("fn-{label}"), self.y);
                let saved = (cx.size_factor, cx.color);
                cx.size_factor *= 0.85;
                cx.color = self.c.muted;
                let fs = self.s * cx.size_factor;
                let item = ListItem { task: None, blocks: blocks.clone() };
                let marker = format!("{number}.");
                let fmt = self.format(SpanStyle::default(), fs, cx.color, false, false);
                let mut job = LayoutJob::single_section(marker.clone(), fmt);
                job.wrap.max_width = f32::INFINITY;
                let indent = (self.galley(job).size().x + 0.6 * fs).max(2.2 * fs);
                self.fresh = true;
                self.list_item(&item, Some(marker), x, w, indent, cx);
                (cx.size_factor, cx.color) = saved;
                self.pending = 0.4 * s;
            }
            Block::Html(text) => {
                self.begin(0.8 * s);
                let span = markdown::Span { text: text.clone(), style: SpanStyle::default(), link: None };
                let (job, _) = self.inline_job(&[Inline::Span(span)], s * 0.95, self.c.muted, false, w);
                let g = self.galley(job);
                self.y += self.place_text(x, g, vec![], true);
                self.pending = 0.8 * s;
            }
        }
    }

    fn list_item(&mut self, item: &ListItem, number: Option<String>, x: f32, w: f32, indent: f32, cx: &mut Ctx) {
        let s = self.s * cx.size_factor;
        let top = self.y;
        // The marker sits on the first line of the item's first paragraph.
        match (item.task, number) {
            (Some(done), _) => {
                let side = 0.95 * s;
                let line_h = s * 1.55;
                let rect = Rect::from_min_size(
                    Pos2::new(x + indent - side - 0.45 * s, top + (line_h - side) / 2.0),
                    Vec2::splat(side),
                );
                self.prims.push(Prim::Check { rect, checked: done });
            }
            (None, marker) => {
                let text = marker.unwrap_or_else(|| ["•", "◦", "▪"][(cx.list_depth.max(1) - 1) % 3].to_string());
                let fmt = self.format(SpanStyle::default(), s, cx.color, false, false);
                let mut job = LayoutJob::single_section(text, fmt);
                job.wrap.max_width = f32::INFINITY;
                let g = self.galley(job);
                let mx = x + indent - g.size().x - 0.45 * s;
                self.prims.push(Prim::Text(TextPrim {
                    pos: Pos2::new(mx, top),
                    galley: g,
                    links: vec![],
                    selectable: false,
                }));
            }
        }
        self.fresh = true;
        if item.blocks.is_empty() {
            self.y += s * 1.55;
        }
        self.blocks(&item.blocks, x + indent, w - indent, cx);
        self.fresh = false;
    }

    fn table(
        &mut self,
        aligns: &[Alignment],
        head: &[Vec<Inline>],
        rows: &[Vec<Vec<Inline>>],
        x: f32,
        w: f32,
        cx: &Ctx,
    ) {
        let s = self.s * cx.size_factor;
        self.begin(0.8 * s);
        let ncols = head.len().max(rows.iter().map(|r| r.len()).max().unwrap_or(0)).max(1);
        let empty: Vec<Inline> = Vec::new();
        fn cell<'a>(r: &'a [Vec<Inline>], c: usize, empty: &'a [Inline]) -> &'a [Inline] {
            r.get(c).map_or(empty, |v| &v[..])
        }
        let all_rows = std::iter::once((head, true)).chain(rows.iter().map(|r| (&r[..], false)));
        // A table with more columns than fit even at their narrowest is set in
        // smaller type (down to half size), rather than running off the page.
        let mut k = 1.0f32;
        let (size, padx, pady, widths) = loop {
            let (size, padx, pady) = (0.94 * s * k, 0.7 * s * k, 0.4 * s * k);
            // Natural (unwrapped) column widths.
            let mut natural = vec![0.0f32; ncols];
            for (r, is_head) in all_rows.clone() {
                for (c, nat) in natural.iter_mut().enumerate() {
                    let (job, _) = self.inline_job(cell(r, c, &empty), size, cx.color, is_head, f32::INFINITY);
                    *nat = nat.max(self.galley(job).size().x + 2.0 * padx);
                }
            }
            let total: f32 = natural.iter().sum();
            if total <= w {
                break (size, padx, pady, natural);
            }
            let min = 3.0 * s * k;
            let min_total: f32 = natural.iter().map(|n| min.min(*n)).sum();
            if min_total > w && k > 0.5 {
                k = (k * w / min_total).max(0.5);
                continue;
            }
            let widths: Vec<f32> = natural.iter().map(|n| (n * w / total).max(min.min(*n))).collect();
            break (size, padx, pady, widths);
        };
        let table_w: f32 = widths.iter().sum();
        let border = Stroke::new(1.0, self.c.rule);
        for (ri, (r, is_head)) in all_rows.enumerate() {
            let mut galleys = Vec::with_capacity(ncols);
            for (c, cw) in widths.iter().enumerate() {
                let (job, links, maths) =
                    self.inline_job_math(cell(r, c, &empty), size, cx.color, is_head, cw - 2.0 * padx);
                galleys.push((self.galley(job), links, maths));
            }
            let row_h = galleys.iter().map(|(g, _, _)| g.size().y).fold(0.0, f32::max) + 2.0 * pady;
            if ri > 0 {
                self.add_break(self.y);
            }
            let fill = if is_head {
                self.c.table_head
            } else if ri % 2 == 0 {
                self.c.code_bg.gamma_multiply(0.6)
            } else {
                Color32::TRANSPARENT
            };
            if fill != Color32::TRANSPARENT {
                let rect = Rect::from_min_size(Pos2::new(x, self.y), Vec2::new(table_w, row_h));
                self.prims.push(Prim::Rect { rect, fill, stroke: Stroke::NONE, radius: 0.0 });
            }
            let mut cx_x = x;
            for (c, (g, links, maths)) in galleys.into_iter().enumerate() {
                let cw = widths[c];
                let gx = match aligns.get(c) {
                    Some(Alignment::Center) => cx_x + (cw - g.size().x) / 2.0,
                    Some(Alignment::Right) => cx_x + cw - padx - g.size().x,
                    _ => cx_x + padx,
                };
                let rect = Rect::from_min_size(Pos2::new(cx_x, self.y), Vec2::new(cw, row_h));
                self.prims.push(Prim::Rect { rect, fill: Color32::TRANSPARENT, stroke: border, radius: 0.0 });
                // Cells are centred vertically in their row.
                let gy = self.y + (row_h - g.size().y) / 2.0;
                self.place_math(Pos2::new(gx, gy), &g, maths);
                self.prims.push(Prim::Text(TextPrim { pos: Pos2::new(gx, gy), galley: g, links, selectable: true }));
                cx_x += cw;
            }
            self.y += row_h;
        }
        self.pending = 0.8 * s;
    }

    fn image(&mut self, url: &str, alt: &str, link: Option<String>, x: f32, w: f32) {
        let s = self.s;
        self.begin(0.8 * s);
        let placeholder = |b: &mut Self, label: String, h: f32, remote: bool| {
            let rect = Rect::from_min_size(Pos2::new(x, b.y), Vec2::new(w.min(420.0 * b.z), h));
            b.prims.push(Prim::Placeholder { rect, label, remote });
            b.y += h;
        };
        let source = match resolve_image(url, self.p.base_dir.as_deref()) {
            ImageSource::Remote if self.p.allow_remote => match crate::remote::image(self.egui, url) {
                crate::remote::State::Ready { uri, .. } => ImageSource::Local(uri),
                crate::remote::State::Loading => {
                    self.pending_images = true;
                    placeholder(self, format!("⏳ {alt}"), 2.6 * s, false);
                    self.pending = 0.8 * s;
                    return;
                }
                crate::remote::State::Failed(e) => {
                    placeholder(self, format!("🌐 {alt} · {e}"), 2.6 * s, false);
                    self.pending = 0.8 * s;
                    return;
                }
            },
            other => other,
        };
        match source {
            ImageSource::Local(uri) => {
                let poll = self.egui.try_load_texture(&uri, crate::view::IMAGE_TEXTURE, egui::SizeHint::default());
                match poll {
                    Ok(egui::load::TexturePoll::Ready { texture }) => {
                        let mut size = texture.size * self.z;
                        let max_h = self.content_h * 0.95;
                        if size.x > w {
                            size *= w / size.x;
                        }
                        if size.y > max_h {
                            size *= max_h / size.y;
                        }
                        let rect = Rect::from_min_size(Pos2::new(x, self.y), size);
                        self.prims.push(Prim::Image { rect, uri, link });
                        self.y += size.y;
                    }
                    Ok(egui::load::TexturePoll::Pending { .. }) => {
                        self.pending_images = true;
                        placeholder(self, format!("⏳ {alt}"), 120.0 * self.z, false);
                    }
                    Err(_) => placeholder(self, format!("🖼 {alt} ({url})"), 2.6 * s, false),
                }
            }
            ImageSource::Remote => {
                self.remote_blocked += 1;
                let host = url.split("://").nth(1).unwrap_or(url).split('/').next().unwrap_or(url);
                let what = if alt.is_empty() { host.to_string() } else { format!("{alt} · {host}") };
                let label = format!("🌐 {what}  ·  {}", crate::i18n::tr("Click to show internet images"));
                placeholder(self, label, 2.6 * s, true);
            }
            ImageSource::Missing => placeholder(self, format!("🖼 {alt} ({url})"), 2.6 * s, false),
        }
        self.pending = 0.8 * s;
    }
}

/// Cuts the flow into pages: each page ends at the last allowed break that
/// fits, or at a forced page break.
pub fn paginate(total: f32, page_h: f32, breaks: &[f32], forced: &[f32]) -> Vec<PageSpan> {
    let eps = 0.5;
    let mut pages = Vec::new();
    let mut start = 0.0f32;
    while start < total - eps {
        let limit = start + page_h;
        let forced_here = forced
            .iter()
            .copied()
            .filter(|f| *f > start + eps && *f <= limit + eps)
            .fold(None, |a: Option<f32>, f| Some(a.map_or(f, |a| a.min(f))));
        let end = if let Some(f) = forced_here {
            f
        } else if limit >= total {
            total
        } else {
            let i = breaks.partition_point(|b| *b <= limit);
            match breaks[..i].iter().rev().find(|b| **b > start + eps) {
                Some(b) if *b - start > page_h * 0.25 => *b,
                // Only a tiny piece would fit: cut the block at the page edge.
                _ => limit,
            }
        };
        pages.push(PageSpan { start, end });
        start = end;
        if pages.len() > 100_000 {
            break;
        }
    }
    if pages.is_empty() {
        pages.push(PageSpan { start: 0.0, end: 0.0 });
    }
    pages
}

/// Lays out a whole document.
pub fn layout(ctx: &egui::Context, markdown: &str, params: &Params) -> Layout {
    let top = markdown::parse(markdown);
    let anchors: Vec<String> = tandaiturun_core::outline::outline(markdown).into_iter().map(|h| h.anchor).collect();
    layout_blocks(ctx, &top, anchors, params)
}

pub fn layout_blocks(
    ctx: &egui::Context,
    top: &[markdown::TopBlock],
    heading_anchors: Vec<String>,
    p: &Params,
) -> Layout {
    let z = p.zoom;
    let page_size = p.page.size() * z;
    let margin = MARGIN * z;
    let content_w = page_size.x - 2.0 * margin;
    let content_h = page_size.y - 2.0 * margin;
    let mut b = Builder {
        egui: ctx,
        p,
        c: p.theme.colors(),
        z,
        s: p.font_size * z,
        content_h,
        prims: Vec::new(),
        breaks: Vec::new(),
        forced: Vec::new(),
        y: 0.0,
        pending: 0.0,
        fresh: false,
        keep_next: false,
        kept_at: None,
        headings: Vec::new(),
        heading_anchors,
        anchors: HashMap::new(),
        blocks: Vec::new(),
        top_mark: None,
        pending_images: false,
        remote_blocked: 0,
    };
    let mut cx = Ctx { tight: false, color: b.c.text, size_factor: 1.0, list_depth: 0 };
    for t in top {
        b.top_mark = Some(t.offset);
        b.block(&t.block, 0.0, content_w, &mut cx);
        if let Some(offset) = b.top_mark.take() {
            b.blocks.push((offset, b.y));
        }
    }
    let total = b.y;
    let mut breaks = b.breaks;
    breaks.sort_by(|a, b| a.total_cmp(b));
    breaks.dedup();
    let pages = paginate(total, content_h, &breaks, &b.forced);
    let mut page_prims = vec![Vec::new(); pages.len()];
    for (i, prim) in b.prims.iter().enumerate() {
        let (t, bot) = prim.y_range();
        let first = pages.partition_point(|p| p.end <= t + 0.01).min(pages.len() - 1);
        for (k, pg) in pages.iter().enumerate().skip(first) {
            if pg.start >= bot - 0.01 && !(pg.start == pg.end && k == 0) {
                break;
            }
            page_prims[k].push(i);
        }
    }
    Layout {
        params: p.clone(),
        prims: b.prims,
        pages,
        page_prims,
        page_size,
        margin,
        headings: b.headings,
        anchors: b.anchors,
        blocks: b.blocks,
        pending_images: b.pending_images,
        remote_blocked: b.remote_blocked,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn pagination_respects_breaks_and_forced_breaks() {
        // 3 blocks of 400 on 1000-high pages: the third goes to page 2.
        let p = paginate(1200.0, 1000.0, &[0.0, 400.0, 800.0], &[]);
        assert_eq!(p, vec![PageSpan { start: 0.0, end: 800.0 }, PageSpan { start: 800.0, end: 1200.0 }]);
        // A forced break wins.
        let p = paginate(1200.0, 1000.0, &[0.0, 400.0, 800.0], &[400.0]);
        assert_eq!(p[0].end, 400.0);
        // One block taller than a page is cut at the page edge.
        let p = paginate(2500.0, 1000.0, &[0.0], &[]);
        assert_eq!(p.len(), 3);
        // Empty document: one blank page.
        assert_eq!(paginate(0.0, 1000.0, &[], &[]).len(), 1);
    }

    #[test]
    fn local_images_resolve_relative_to_the_document() {
        let dir = std::env::temp_dir().join(format!("tt-img-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("img dir")).unwrap();
        std::fs::write(dir.join("img dir/a.png"), b"x").unwrap();
        assert!(
            matches!(resolve_image("img%20dir/a.png", Some(&dir)), ImageSource::Local(u) if u.starts_with("file://") && u.ends_with("img dir/a.png"))
        );
        assert!(matches!(resolve_image("nope.png", Some(&dir)), ImageSource::Missing));
        assert!(matches!(resolve_image("https://x/y.png", Some(&dir)), ImageSource::Remote));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Runs `f` with a laid-out context, like a frame of the app.
    pub(crate) fn with_ctx(f: impl FnOnce(&egui::Context)) {
        let ctx = egui::Context::default();
        ctx.set_fonts(crate::fonts::definitions());
        let mut f = Some(f);
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            if let Some(f) = f.take() {
                f(ui.ctx());
            }
        });
        out.textures_delta.clear();
    }

    pub(crate) fn params(zoom: f32) -> Params {
        Params {
            page: PageSize::A4,
            font: DocFont::Sans,
            font_size: 15.0,
            theme: PageTheme::Paper,
            zoom,
            base_dir: None,
            pixels_per_point: 1.0,
            allow_remote: false,
            fonts: 0,
        }
    }

    #[test]
    fn long_documents_get_several_pages_and_anchors() {
        let mut md = String::from("# Start\n\n");
        for i in 0..60 {
            md.push_str(&format!("## Section {i}\n\nSome text for section {i}, long enough to wrap around the line at least once when it is laid out on an A4 page.\n\n"));
        }
        with_ctx(|ctx| {
            let l = layout(ctx, &md, &params(1.0));
            assert!(l.page_count() > 3, "{} pages", l.page_count());
            assert_eq!(l.headings.len(), 61);
            assert!(l.anchors.contains_key("section-59"));
            // Pages follow each other without gaps or overlaps.
            for w in l.pages.windows(2) {
                assert_eq!(w[0].end, w[1].start);
            }
            // No page ends right after a heading.
            for (i, p) in l.pages.iter().enumerate().take(l.page_count() - 1) {
                for hy in &l.headings {
                    let last_y = p.end - hy;
                    assert!(!(0.5..50.0).contains(&last_y), "page {i} ends {last_y} after a heading");
                }
            }
            // The same document at 200 % has about the same pagination.
            let l2 = layout(ctx, &md, &params(2.0));
            assert!((l2.page_count() as i32 - l.page_count() as i32).abs() <= 1);
        });
    }

    #[test]
    fn page_breaks_force_new_pages() {
        with_ctx(|ctx| {
            let l = layout(ctx, "one\n\n<!-- pagebreak -->\n\ntwo\n\n<!-- pagebreak -->\n\nthree", &params(1.0));
            assert_eq!(l.page_count(), 3);
            assert_eq!(layout(ctx, "", &params(1.0)).page_count(), 1);
        });
    }

    #[test]
    fn text_hits_and_links() {
        with_ctx(|ctx| {
            let l = layout(ctx, "Go to [the docs](https://example.com) now.", &params(1.0));
            let Prim::Text(t) = &l.prims[0] else { panic!("text first") };
            assert_eq!(t.links.len(), 1);
            let r = char_rects(&t.galley, 6..14)[0];
            let at = t.pos + r.center().to_vec2();
            assert_eq!(l.link_at(0, at), Some("https://example.com"));
            assert_eq!(l.link_at(0, t.pos + Vec2::new(1.0, 2.0)), None);
            let (prim, ch) = l.text_at(0, at, false).unwrap();
            assert_eq!(prim, 0);
            assert!((6..14).contains(&ch));
        });
    }

    #[test]
    fn char_byte_conversion() {
        assert_eq!(char_to_byte("aé b", 2), 3);
        assert_eq!(byte_to_char("aé b", 3), 2);
        assert_eq!(char_to_byte("ab", 9), 2);
    }
}
