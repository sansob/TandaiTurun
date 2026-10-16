//! The document canvas: pages on a grey background, like a PDF reader.
//! Handles zoom, page modes, scrolling to pages and anchors, text selection,
//! links and search highlights.

use crate::i18n::tr;
use crate::layout::{self, char_rects, DocFont, Layout, PageSize, Params, Prim};
use crate::theme::{PageTheme, Palette, ACCENT};
use egui::{Color32, CornerRadius, CursorIcon, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use std::path::PathBuf;
use tandaiturun_core::search::{find_all, SearchOptions};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ZoomMode {
    Fixed(f32),
    FitWidth,
    FitPage,
}

impl Default for ZoomMode {
    fn default() -> Self {
        ZoomMode::Fixed(1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PageMode {
    /// Pages one below the other (the default).
    #[default]
    Continuous,
    /// One page at a time.
    Single,
    /// Two pages side by side, scrolling.
    TwoPage,
}

impl PageMode {
    pub const ALL: [PageMode; 3] = [PageMode::Single, PageMode::Continuous, PageMode::TwoPage];

    pub fn label(self) -> &'static str {
        match self {
            PageMode::Single => "Single Page View",
            PageMode::Continuous => "Enable Scrolling",
            PageMode::TwoPage => "Two Page View",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Select,
    Hand,
}

/// The zoom presets of the zoom menu.
pub const ZOOM_STEPS: &[f32] = &[0.25, 0.33, 0.5, 0.67, 0.75, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0];

/// Settings shared by every view (from the app settings).
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub page: PageSize,
    pub font: DocFont,
    pub font_size: f32,
    pub theme: PageTheme,
    /// Load images from the internet.
    pub allow_remote: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPos {
    pub prim: usize,
    pub ch: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: TextPos,
    pub focus: TextPos,
}

impl Selection {
    fn ordered(&self) -> (TextPos, TextPos) {
        let a = (self.anchor.prim, self.anchor.ch);
        let b = (self.focus.prim, self.focus.ch);
        if a <= b {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.focus
    }
}

#[derive(Debug, Clone)]
pub struct Hit {
    pub prim: usize,
    pub chars: Range<usize>,
    pub page: usize,
    /// Text around the match, for the results list.
    pub snippet: String,
}

#[derive(Debug, Default)]
pub struct Search {
    pub query: String,
    pub opts: SearchOptions,
    pub hits: Vec<Hit>,
    pub current: Option<usize>,
    key: Option<(String, SearchOptions, u64)>,
}

pub enum ViewEvent {
    /// A link was clicked (not an in-document anchor, those are handled here).
    Link(String),
    /// "Edit here": switch to the editor at this source offset.
    EditAt(usize),
    /// The reader clicked an internet image placeholder: load them.
    AllowRemote,
}

enum ScrollTarget {
    Page(usize),
    Flow(f32),
}

pub struct DocView {
    pub zoom: ZoomMode,
    pub mode: PageMode,
    pub tool: Tool,
    pub layout: Option<Layout>,
    key: Option<(u64, Params)>,
    /// Bumped on every new layout.
    pub generation: u64,
    pub current_page: usize,
    /// Flow y at the top of the viewport.
    pub top_flow: f32,
    target: Option<ScrollTarget>,
    pub selection: Option<Selection>,
    pub search: Search,
    canvas_size: Vec2,
    viewport_size: Vec2,
    keep_ratio: Option<f32>,
    drag_select: bool,
}

impl Default for DocView {
    fn default() -> Self {
        DocView {
            zoom: ZoomMode::default(),
            mode: PageMode::default(),
            tool: Tool::default(),
            layout: None,
            key: None,
            generation: 0,
            current_page: 0,
            top_flow: 0.0,
            target: None,
            selection: None,
            search: Search::default(),
            canvas_size: Vec2::ZERO,
            viewport_size: Vec2::new(800.0, 600.0),
            keep_ratio: None,
            drag_select: false,
        }
    }
}

const PAD: f32 = 20.0;
const GAP: f32 = 14.0;

impl DocView {
    pub fn new(zoom: ZoomMode, mode: PageMode) -> DocView {
        DocView { zoom, mode, ..Default::default() }
    }

    pub fn page_count(&self) -> usize {
        self.layout.as_ref().map_or(1, |l| l.page_count())
    }

    /// The zoom actually used (fit modes depend on the window size).
    pub fn effective_zoom(&self, look: &Look) -> f32 {
        let page = look.page.size();
        let cols = if self.mode == PageMode::TwoPage { 2.0 } else { 1.0 };
        let avail = self.viewport_size - Vec2::splat(2.0 * PAD);
        let fit_w = (avail.x - (cols - 1.0) * GAP - 14.0) / (page.x * cols);
        let fit_h = avail.y / page.y;
        let z = match self.zoom {
            ZoomMode::Fixed(z) => z,
            ZoomMode::FitWidth => fit_w,
            ZoomMode::FitPage => fit_w.min(fit_h),
        };
        (z.clamp(0.1, 8.0) * 100.0).round() / 100.0
    }

    pub fn set_zoom(&mut self, zoom: ZoomMode) {
        if self.zoom != zoom {
            self.keep_ratio = Some(self.scroll_ratio());
            self.zoom = zoom;
        }
    }

    pub fn zoom_in(&mut self, look: &Look) {
        let z = self.effective_zoom(look);
        let next = ZOOM_STEPS.iter().copied().find(|s| *s > z + 0.001).unwrap_or(z);
        self.set_zoom(ZoomMode::Fixed(next));
    }

    pub fn zoom_out(&mut self, look: &Look) {
        let z = self.effective_zoom(look);
        let next = ZOOM_STEPS.iter().rev().copied().find(|s| *s < z - 0.001).unwrap_or(z);
        self.set_zoom(ZoomMode::Fixed(next));
    }

    pub fn set_mode(&mut self, mode: PageMode) {
        if self.mode != mode {
            self.mode = mode;
            self.go_to_page(self.current_page);
        }
    }

    fn scroll_ratio(&self) -> f32 {
        match &self.layout {
            Some(l) if !l.pages.is_empty() => {
                let total = l.pages.last().map_or(1.0, |p| p.end).max(1.0);
                self.top_flow / total
            }
            _ => 0.0,
        }
    }

    /// Next / previous page, or row of pages in Two Page view.
    pub fn step_page(&mut self, forward: bool) {
        let n = self.page_count();
        let step = if self.mode == PageMode::TwoPage { 2 } else { 1 };
        let row = self.current_page - self.current_page % step;
        let page = if forward { row + step } else { row.saturating_sub(step) };
        if page < n {
            self.go_to_page(page);
        }
    }

    pub fn go_to_page(&mut self, page: usize) {
        let n = self.page_count();
        self.current_page = page.min(n.saturating_sub(1));
        self.target = Some(ScrollTarget::Page(self.current_page));
    }

    pub fn go_to_flow(&mut self, y: f32) {
        self.target = Some(ScrollTarget::Flow(y));
    }

    pub fn go_to_heading(&mut self, index: usize) {
        if let Some(y) = self.layout.as_ref().and_then(|l| l.headings.get(index).copied()) {
            self.go_to_flow(y);
        }
    }

    pub fn go_to_anchor(&mut self, anchor: &str) -> bool {
        let y = self.layout.as_ref().and_then(|l| {
            l.anchors.get(anchor).or_else(|| l.anchors.get(&tandaiturun_core::outline::slugify(anchor))).copied()
        });
        if let Some(y) = y {
            self.go_to_flow(y);
            true
        } else {
            false
        }
    }

    /// Scrolls the preview to the block at a source offset.
    pub fn go_to_offset(&mut self, offset: usize) {
        if let Some(y) = self.layout.as_ref().and_then(|l| l.y_of_offset(offset)) {
            self.go_to_flow(y);
        }
    }

    /// Page rectangles in canvas coordinates.
    fn page_rects(&self, l: &Layout) -> Vec<Rect> {
        let ps = l.page_size;
        let width = self.viewport_size.x.max(1.0);
        let n = l.pages.len();
        match self.mode {
            PageMode::Continuous | PageMode::Single => (0..n)
                .map(|i| {
                    let x = ((width - ps.x) / 2.0).max(PAD);
                    let y = if self.mode == PageMode::Single { PAD } else { PAD + i as f32 * (ps.y + GAP) };
                    Rect::from_min_size(Pos2::new(x, y), ps)
                })
                .collect(),
            PageMode::TwoPage => (0..n)
                .map(|i| {
                    let row_w = 2.0 * ps.x + GAP;
                    let x0 = ((width - row_w) / 2.0).max(PAD);
                    let x = x0 + (i % 2) as f32 * (ps.x + GAP);
                    let y = PAD + (i / 2) as f32 * (ps.y + GAP);
                    Rect::from_min_size(Pos2::new(x, y), ps)
                })
                .collect(),
        }
    }

    /// Lays the document out again when the text or the settings changed.
    pub fn update_layout(&mut self, ctx: &egui::Context, text: &str, rev: u64, look: &Look, base_dir: Option<PathBuf>) {
        let params = Params {
            page: look.page,
            font: look.font,
            font_size: look.font_size,
            theme: look.theme,
            zoom: self.effective_zoom(look),
            base_dir,
            pixels_per_point: ctx.pixels_per_point(),
            allow_remote: look.allow_remote,
            fonts: crate::layout::font_count(ctx),
        };
        let stale = self.key.as_ref() != Some(&(rev, params.clone()));
        let images_ready =
            self.layout.as_ref().is_some_and(|l| l.pending_images) && ctx.cumulative_frame_nr().is_multiple_of(10);
        if stale || images_ready {
            let same_text = self.key.as_ref().is_some_and(|(r, _)| *r == rev);
            if self.keep_ratio.is_none() && self.layout.is_some() {
                self.keep_ratio = Some(self.scroll_ratio());
            }
            self.layout = Some(layout::layout(ctx, text, &params));
            self.key = Some((rev, params));
            self.generation += 1;
            if !same_text {
                self.selection = None;
            }
            if let Some(l) = &self.layout {
                if l.pending_images {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
            }
        }
    }

    /// Recomputes search hits when the query or the layout changed.
    pub fn refresh_search(&mut self) {
        let key = (self.search.query.clone(), self.search.opts, self.generation);
        if self.search.key.as_ref() == Some(&key) {
            return;
        }
        let keep_current = self.search.key.as_ref().is_some_and(|k| k.0 == key.0 && k.1 == key.1);
        self.search.key = Some(key);
        let old = self.search.current;
        self.search.hits.clear();
        let Some(l) = &self.layout else { return };
        if self.search.query.is_empty() {
            self.search.current = None;
            return;
        }
        for (i, p) in l.prims.iter().enumerate() {
            let Prim::Text(t) = p else { continue };
            if !t.selectable {
                continue;
            }
            let text = &t.galley.job.text;
            for r in find_all(text, &self.search.query, self.search.opts) {
                let chars = layout::byte_to_char(text, r.start)..layout::byte_to_char(text, r.end);
                let y = t.pos.y + char_rects(&t.galley, chars.clone()).first().map_or(0.0, |r| r.top());
                let snippet = snippet(text, r);
                self.search.hits.push(Hit { prim: i, chars, page: l.page_of(y), snippet });
            }
        }
        self.search.current = if self.search.hits.is_empty() {
            None
        } else if keep_current {
            old.map(|c| c.min(self.search.hits.len() - 1))
        } else {
            // The first hit at or after the current position.
            let top = self.top_flow;
            let first = self.search.hits.iter().position(|h| hit_y(l, h) >= top - 1.0).unwrap_or(0);
            self.target = Some(ScrollTarget::Flow(hit_y(l, &self.search.hits[first])));
            Some(first)
        };
    }

    pub fn search_step(&mut self, forward: bool) {
        let n = self.search.hits.len();
        if n == 0 {
            return;
        }
        let next = match self.search.current {
            None => 0,
            Some(c) if forward => (c + 1) % n,
            Some(c) => (c + n - 1) % n,
        };
        self.select_hit(next);
    }

    pub fn select_hit(&mut self, i: usize) {
        self.search.current = Some(i);
        if let (Some(l), Some(h)) = (&self.layout, self.search.hits.get(i)) {
            let y = hit_y(l, h);
            self.target = Some(ScrollTarget::Flow((y - l.page_size.y * 0.15).max(0.0)));
            self.current_page = h.page;
        }
    }

    /// The selected text, lines joined with newlines.
    pub fn selected_text(&self) -> Option<String> {
        let sel = self.selection?;
        let l = self.layout.as_ref()?;
        let (a, b) = sel.ordered();
        let mut out = String::new();
        for i in a.prim..=b.prim {
            let Some(Prim::Text(t)) = l.prims.get(i) else { continue };
            if !t.selectable {
                continue;
            }
            let text = &t.galley.job.text;
            let n = text.chars().count();
            let from = if i == a.prim { a.ch } else { 0 };
            let to = if i == b.prim { b.ch } else { n };
            if !out.is_empty() {
                out.push('\n');
            }
            out.extend(text.chars().skip(from).take(to.saturating_sub(from)));
        }
        Some(out)
    }

    pub fn select_all(&mut self) {
        let Some(l) = &self.layout else { return };
        let texts: Vec<usize> = l
            .prims
            .iter()
            .enumerate()
            .filter(|(_, p)| matches!(p, Prim::Text(t) if t.selectable))
            .map(|(i, _)| i)
            .collect();
        if let (Some(&first), Some(&last)) = (texts.first(), texts.last()) {
            let Prim::Text(t) = &l.prims[last] else { return };
            let n = t.galley.job.text.chars().count();
            self.selection =
                Some(Selection { anchor: TextPos { prim: first, ch: 0 }, focus: TextPos { prim: last, ch: n } });
        }
    }

    /// Draws the canvas. Returns the events the app has to act on.
    pub fn show(&mut self, ui: &mut egui::Ui, id: egui::Id, pal: &Palette) -> Vec<ViewEvent> {
        let mut events = Vec::new();
        self.viewport_size = ui.available_size();
        let Some(l) = self.layout.take() else { return events };
        let rects = self.page_rects(&l);
        let mut canvas = rects.iter().fold(Vec2::ZERO, |acc, r| acc.max(r.max.to_vec2())) + Vec2::splat(PAD);
        canvas.x = canvas.x.max(self.viewport_size.x);
        if self.mode == PageMode::Single {
            canvas.y = l.page_size.y + 2.0 * PAD;
        }
        self.canvas_size = canvas;

        // Where to scroll this frame.
        let mut offset: Option<Vec2> = None;
        let target = self.target.take();
        if let Some(ratio) = self.keep_ratio.take() {
            let total = l.pages.last().map_or(0.0, |p| p.end);
            let y = ratio * total;
            self.place_flow(&l, &rects, y, &mut offset);
        }
        match target {
            Some(ScrollTarget::Page(p)) => {
                if let Some(r) = rects.get(p) {
                    offset =
                        Some(Vec2::new(0.0, if self.mode == PageMode::Single { 0.0 } else { r.top() - PAD / 2.0 }));
                }
            }
            Some(ScrollTarget::Flow(y)) => self.place_flow(&l, &rects, y, &mut offset),
            None => {}
        }

        let mut area =
            egui::ScrollArea::both().id_salt(id).auto_shrink(false).wheel_scroll_multiplier(Vec2::splat(1.0));
        if let Some(o) = offset {
            area = area.scroll_offset(Vec2::new(o.x.max(0.0), o.y.max(0.0)));
        }
        // The canvas senses drags itself, which hides them from the area's own
        // drag-to-scroll, so the Hand tool pans by hand below.
        area = area.scroll_source(
            egui::containers::scroll_area::ScrollSource::SCROLL_BAR
                | egui::containers::scroll_area::ScrollSource::MOUSE_WHEEL,
        );
        let colors = l.params.theme.colors();
        let out = area.show_viewport(ui, |ui, viewport| {
            let (canvas_rect, resp) = ui.allocate_exact_size(canvas, Sense::click_and_drag());
            let origin = canvas_rect.min;
            let painter = ui.painter_at(ui.clip_rect());
            painter.rect_filled(ui.clip_rect(), 0.0, pal.canvas);

            // The current page: the one under the middle of the viewport.
            let probe = viewport.center().y.min(viewport.top() + viewport.height() * 0.4);
            if self.mode != PageMode::Single {
                if let Some(i) = rects.iter().position(|r| r.bottom() + GAP >= probe) {
                    self.current_page = i;
                }
            }
            // Flow y at the top of the viewport (for keeping the position on zoom).
            // Single Page shows one page, and every page sits at the same spot.
            let top = if self.mode == PageMode::Single {
                rects.get(self.current_page).map(|r| (self.current_page, r))
            } else {
                rects.iter().enumerate().find(|(_, r)| r.bottom() >= viewport.top())
            };
            if let Some((i, r)) = top {
                let span = l.pages[i];
                self.top_flow = span.start + (viewport.top() - r.top() - l.margin).clamp(0.0, span.end - span.start);
            }

            let hits_by_prim = self.hit_rects(&l);
            for (i, r) in rects.iter().enumerate() {
                if self.mode == PageMode::Single && i != self.current_page {
                    continue;
                }
                let r = if self.mode == PageMode::Single {
                    Rect::from_min_size(Pos2::new(r.left(), PAD), r.size())
                } else {
                    *r
                };
                if !r.intersects(viewport.expand(200.0)) {
                    continue;
                }
                let screen = r.translate(origin.to_vec2());
                paint_page(ui, &painter, &l, i, screen, &colors);
                paint_marks(&painter, &l, i, screen, &hits_by_prim, self.selection.as_ref(), &colors);
            }

            // Pointer: links, selection, context menu.
            let (mode, current_page) = (self.mode, self.current_page);
            let to_page = |pos: Pos2| -> Option<(usize, Pos2)> {
                let c = pos - origin.to_vec2();
                rects.iter().enumerate().find_map(|(i, r)| {
                    let r = if mode == PageMode::Single {
                        if i != current_page {
                            return None;
                        }
                        Rect::from_min_size(Pos2::new(r.left(), PAD), r.size())
                    } else {
                        *r
                    };
                    r.expand(GAP / 2.0).contains(c).then(|| {
                        let span = l.pages[i];
                        (i, Pos2::new(c.x - r.left() - l.margin, c.y - r.top() - l.margin + span.start))
                    })
                })
            };
            let hover_link = resp.hover_pos().and_then(to_page).and_then(|(p, f)| l.link_at(p, f).map(str::to_owned));
            let hover_remote = resp.hover_pos().and_then(to_page).is_some_and(|(p, f)| l.remote_placeholder_at(p, f));
            if hover_remote {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                if resp.clicked() {
                    events.push(ViewEvent::AllowRemote);
                }
            } else if let Some(url) = &hover_link {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                resp.clone().on_hover_text_at_pointer(url.as_str());
            } else if self.tool == Tool::Hand {
                ui.ctx().set_cursor_icon(if resp.dragged() { CursorIcon::Grabbing } else { CursorIcon::Grab });
            } else if resp.hover_pos().and_then(to_page).and_then(|(p, f)| l.text_at(p, f, false)).is_some() {
                ui.ctx().set_cursor_icon(CursorIcon::Text);
            }

            if resp.clicked() {
                if let Some(url) = hover_link {
                    if let Some(anchor) = url.strip_prefix('#') {
                        self.go_to_anchor(anchor);
                    } else {
                        events.push(ViewEvent::Link(url));
                    }
                } else if !ui.input(|i| i.modifiers.shift) {
                    self.selection = None;
                }
            }
            if self.tool == Tool::Hand && resp.dragged_by(egui::PointerButton::Primary) {
                ui.scroll_with_delta(resp.drag_delta());
            }
            if self.tool == Tool::Select {
                if resp.drag_started_by(egui::PointerButton::Primary) {
                    let start = resp.interact_pointer_pos().and_then(to_page).and_then(|(p, f)| l.text_at(p, f, true));
                    self.drag_select = start.is_some();
                    self.selection = start.map(|(prim, ch)| {
                        let tp = TextPos { prim, ch };
                        Selection { anchor: tp, focus: tp }
                    });
                }
                if self.drag_select && resp.dragged_by(egui::PointerButton::Primary) {
                    if let Some((prim, ch)) =
                        resp.interact_pointer_pos().and_then(to_page).and_then(|(p, f)| l.text_at(p, f, true))
                    {
                        if let Some(sel) = &mut self.selection {
                            sel.focus = TextPos { prim, ch };
                        }
                    }
                    // Scroll when dragging past the edge.
                    if let Some(p) = resp.interact_pointer_pos() {
                        let clip = ui.clip_rect();
                        let dy = if p.y > clip.bottom() - 10.0 {
                            12.0
                        } else if p.y < clip.top() + 10.0 {
                            -12.0
                        } else {
                            0.0
                        };
                        if dy != 0.0 {
                            ui.scroll_with_delta(Vec2::new(0.0, -dy));
                        }
                    }
                }
                if resp.drag_stopped() {
                    self.drag_select = false;
                }
                if resp.double_clicked() {
                    if let Some((prim, ch)) =
                        resp.interact_pointer_pos().and_then(to_page).and_then(|(p, f)| l.text_at(p, f, false))
                    {
                        if let Some(Prim::Text(t)) = l.prims.get(prim) {
                            let (a, b) = word_at(&t.galley.job.text, ch);
                            self.selection =
                                Some(Selection { anchor: TextPos { prim, ch: a }, focus: TextPos { prim, ch: b } });
                        }
                    }
                }
            }
            let click_offset = resp.interact_pointer_pos().and_then(to_page).and_then(|(_, f)| l.offset_of_y(f.y));
            if resp.secondary_clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(id.with("ctx_offset"), click_offset));
            }
            resp.context_menu(|ui| {
                let has_sel = self.selection.is_some_and(|s| !s.is_empty());
                if ui.add_enabled(has_sel, egui::Button::new(tr("Copy")).shortcut_text("Ctrl+C")).clicked() {
                    if let Some(t) = self.selected_text() {
                        ui.ctx().copy_text(t);
                    }
                    ui.close();
                }
                if ui.add(egui::Button::new(tr("Select All")).shortcut_text("Ctrl+A")).clicked() {
                    self.select_all();
                    ui.close();
                }
                ui.separator();
                if ui.button(tr("Edit Here")).clicked() {
                    let at: Option<usize> = ui.ctx().data(|d| d.get_temp(id.with("ctx_offset"))).flatten();
                    events.push(ViewEvent::EditAt(at.unwrap_or(0)));
                    ui.close();
                }
                ui.separator();
                if ui.button(tr("Select Tool")).clicked() {
                    self.tool = Tool::Select;
                    ui.close();
                }
                if ui.button(tr("Hand Tool")).clicked() {
                    self.tool = Tool::Hand;
                    ui.close();
                }
            });
        });
        // Ctrl + wheel zooms.
        if out.inner_rect.contains(ui.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::new(-1.0, -1.0))) {
            let zd = ui.input(|i| i.zoom_delta());
            if (zd - 1.0).abs() > 0.001 {
                let look_zoom = l.params.zoom;
                let z = (look_zoom * zd).clamp(0.1, 8.0);
                self.keep_ratio = Some(self.scroll_ratio());
                self.zoom = ZoomMode::Fixed((z * 100.0).round() / 100.0);
            }
        }
        self.layout = Some(l);
        events
    }

    fn place_flow(&mut self, l: &Layout, rects: &[Rect], y: f32, offset: &mut Option<Vec2>) {
        let page = l.page_of(y);
        if let Some(r) = rects.get(page) {
            let span = l.pages[page];
            self.current_page = page;
            let within = (y - span.start).max(0.0);
            let top = if self.mode == PageMode::Single { PAD } else { r.top() };
            // The start of a page shows the whole page, margin included.
            let y = if within <= 1.0 { top - PAD / 2.0 } else { top + l.margin + within - 12.0 };
            *offset = Some(Vec2::new(0.0, y.max(0.0)));
        }
    }

    fn hit_rects(&self, _l: &Layout) -> Vec<(usize, Range<usize>, bool)> {
        self.search
            .hits
            .iter()
            .enumerate()
            .map(|(i, h)| (h.prim, h.chars.clone(), self.search.current == Some(i)))
            .collect()
    }

    /// Keyboard navigation while the canvas has focus.
    pub fn handle_keys(&mut self, ctx: &egui::Context) {
        let n = self.page_count();
        let (page_down, page_up, home, end, copy, all) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::PageDown),
                i.consume_key(egui::Modifiers::NONE, egui::Key::PageUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Home),
                i.consume_key(egui::Modifiers::NONE, egui::Key::End),
                i.events.iter().any(|e| matches!(e, egui::Event::Copy)),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::A),
            )
        });
        if page_down {
            self.step_page(true);
        }
        if page_up {
            self.step_page(false);
        }
        if home {
            self.go_to_page(0);
        }
        if end {
            self.go_to_page(n.saturating_sub(1));
        }
        if self.mode == PageMode::Single {
            let (right, left) =
                ctx.input(|i| (i.key_pressed(egui::Key::ArrowRight), i.key_pressed(egui::Key::ArrowLeft)));
            if right && self.current_page + 1 < n {
                self.go_to_page(self.current_page + 1);
            }
            if left {
                self.go_to_page(self.current_page.saturating_sub(1));
            }
        }
        if copy {
            if let Some(t) = self.selected_text().filter(|t| !t.is_empty()) {
                ctx.copy_text(t);
            }
        }
        if all {
            self.select_all();
        }
    }
}

fn hit_y(l: &Layout, h: &Hit) -> f32 {
    match l.prims.get(h.prim) {
        Some(Prim::Text(t)) => t.pos.y + char_rects(&t.galley, h.chars.clone()).first().map_or(0.0, |r| r.top()),
        _ => 0.0,
    }
}

fn snippet(text: &str, r: Range<usize>) -> String {
    let mut a = r.start.saturating_sub(40);
    while !text.is_char_boundary(a) {
        a -= 1;
    }
    let mut b = (r.end + 60).min(text.len());
    while !text.is_char_boundary(b) {
        b += 1;
    }
    let mut s = String::new();
    if a > 0 {
        s.push('…');
    }
    s.push_str(&text[a..b].replace('\n', " "));
    if b < text.len() {
        s.push('…');
    }
    s
}

/// The word (as a char range) around character `ch`.
fn word_at(text: &str, ch: usize) -> (usize, usize) {
    let chars: Vec<char> = text.chars().collect();
    let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '-' || c == '\'';
    if chars.is_empty() {
        return (0, 0);
    }
    let ch = ch.min(chars.len() - 1);
    if !is_word(chars[ch]) {
        return (ch, ch + 1);
    }
    let mut a = ch;
    while a > 0 && is_word(chars[a - 1]) {
        a -= 1;
    }
    let mut b = ch;
    while b < chars.len() && is_word(chars[b]) {
        b += 1;
    }
    (a, b)
}

/// The content origin of a page: where flow y = `span.start` is drawn.
fn content_origin(l: &Layout, page: usize, page_rect: Rect) -> (Pos2, Rect) {
    let span = l.pages[page];
    let top = page_rect.top() + l.margin;
    let origin = Pos2::new(page_rect.left() + l.margin, top - span.start);
    let clip = Rect::from_min_max(
        Pos2::new(page_rect.left() + 1.0, top - 0.5),
        Pos2::new(page_rect.right() - 1.0, top + (span.end - span.start) + 0.5),
    );
    (origin, clip)
}

pub fn paint_page(
    ui: &egui::Ui,
    painter: &egui::Painter,
    l: &Layout,
    page: usize,
    rect: Rect,
    c: &crate::theme::PageColors,
) {
    // Soft shadow, then the paper.
    for (k, a) in [(1.0, 38u8), (3.0, 18), (6.0, 8)] {
        painter.rect_filled(rect.translate(Vec2::new(0.0, k * 0.6)).expand(k), 2.0, Color32::from_black_alpha(a));
    }
    painter.rect_filled(rect, 0.0, c.paper);
    let (origin, clip) = content_origin(l, page, rect);
    let painter = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
    let off = origin.to_vec2();
    for &i in &l.page_prims[page] {
        match &l.prims[i] {
            Prim::Rect { rect, fill, stroke, radius } => {
                let r = rect.translate(off);
                painter.rect_filled(r, *radius, *fill);
                if stroke.width > 0.0 {
                    painter.rect_stroke(r, *radius, *stroke, StrokeKind::Inside);
                }
            }
            Prim::Line { a, b, stroke } => {
                painter.line_segment([*a + off, *b + off], *stroke);
            }
            _ => {}
        }
    }
    for &i in &l.page_prims[page] {
        match &l.prims[i] {
            Prim::Text(t) => painter.galley(t.pos + off, t.galley.clone(), c.text),
            Prim::Image { rect, uri, .. } => paint_image(ui.ctx(), &painter, uri, rect.translate(off)),
            Prim::Placeholder { rect, label, remote } => {
                let r = rect.translate(off);
                painter.rect_filled(r, 4.0, c.code_bg);
                let border = if *remote { Stroke::new(1.0, c.link) } else { Stroke::new(1.0, c.code_border) };
                painter.rect_stroke(r, 4.0, border, StrokeKind::Inside);
                let font = egui::FontId::proportional((13.0 * l.params.zoom).max(6.0));
                let g = painter.layout(label.clone(), font, c.muted, r.width() - 16.0);
                painter.with_clip_rect(r.intersect(painter.clip_rect())).galley(
                    Pos2::new(r.left() + 8.0, r.center().y - g.size().y / 2.0),
                    g,
                    c.muted,
                );
            }
            Prim::Check { rect, checked } => {
                let r = rect.translate(off);
                let radius = CornerRadius::same((3.0 * l.params.zoom) as u8);
                if *checked {
                    painter.rect_filled(r, radius, ACCENT);
                    let w = r.width();
                    let pts = vec![
                        Pos2::new(r.left() + w * 0.22, r.top() + w * 0.52),
                        Pos2::new(r.left() + w * 0.42, r.top() + w * 0.72),
                        Pos2::new(r.left() + w * 0.78, r.top() + w * 0.3),
                    ];
                    painter.add(egui::Shape::line(pts, Stroke::new((1.8 * l.params.zoom).max(1.0), Color32::WHITE)));
                } else {
                    painter.rect_stroke(
                        r,
                        radius,
                        Stroke::new((1.3 * l.params.zoom).max(1.0), c.muted),
                        StrokeKind::Inside,
                    );
                }
            }
            _ => {}
        }
    }
}

/// Pictures, diagrams and formulas get mipmaps, so they stay smooth when the
/// page is zoomed out or drawn as a thumbnail.
pub const IMAGE_TEXTURE: egui::TextureOptions =
    egui::TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear));

fn paint_image(ctx: &egui::Context, painter: &egui::Painter, uri: &str, rect: Rect) {
    if let Ok(egui::load::TexturePoll::Ready { texture }) =
        ctx.try_load_texture(uri, IMAGE_TEXTURE, egui::SizeHint::default())
    {
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        painter.image(texture.id, rect, uv, Color32::WHITE);
    }
}

/// Search hits and the selection on top of a page.
fn paint_marks(
    painter: &egui::Painter,
    l: &Layout,
    page: usize,
    rect: Rect,
    hits: &[(usize, Range<usize>, bool)],
    sel: Option<&Selection>,
    c: &crate::theme::PageColors,
) {
    let (origin, clip) = content_origin(l, page, rect);
    let painter = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
    let on_page = &l.page_prims[page];
    for (prim, chars, current) in hits {
        if on_page.binary_search(prim).is_err() {
            continue;
        }
        if let Prim::Text(t) = &l.prims[*prim] {
            for r in char_rects(&t.galley, chars.clone()) {
                let r = r.translate(t.pos.to_vec2() + origin.to_vec2()).expand(1.0);
                painter.rect_filled(r, 2.0, if *current { c.mark_current } else { c.mark });
                if *current {
                    painter.rect_stroke(
                        r,
                        2.0,
                        Stroke::new(1.5, Color32::from_rgb(0xE0, 0x6A, 0x00)),
                        StrokeKind::Outside,
                    );
                }
            }
        }
    }
    if let Some(sel) = sel {
        let (a, b) = sel.ordered();
        for &i in on_page {
            if i < a.prim || i > b.prim {
                continue;
            }
            if let Prim::Text(t) = &l.prims[i] {
                if !t.selectable {
                    continue;
                }
                let n = t.galley.job.text.chars().count();
                let from = if i == a.prim { a.ch } else { 0 };
                let to = if i == b.prim { b.ch } else { n };
                for r in char_rects(&t.galley, from..to) {
                    painter.rect_filled(r.translate(t.pos.to_vec2() + origin.to_vec2()), 0.0, c.selection);
                }
            }
        }
    }
}

/// A small picture of a page: text rows become grey bars, everything else
/// is drawn to scale. Used for the page thumbnails.
pub fn paint_thumbnail(ui: &egui::Ui, painter: &egui::Painter, l: &Layout, page: usize, rect: Rect) {
    let c = l.params.theme.colors();
    painter.rect_filled(rect.translate(Vec2::new(0.0, 1.0)).expand(1.0), 1.0, Color32::from_black_alpha(40));
    painter.rect_filled(rect, 0.0, c.paper);
    let k = rect.width() / l.page_size.x;
    let span = l.pages[page];
    let origin = rect.min + Vec2::splat(l.margin * k) - Vec2::new(0.0, span.start * k);
    let clip = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top() + l.margin * k),
        Pos2::new(rect.right(), rect.top() + (l.margin + span.end - span.start) * k),
    );
    let painter = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
    let map = |p: Pos2| origin + p.to_vec2() * k;
    for &i in &l.page_prims[page] {
        match &l.prims[i] {
            Prim::Rect { rect, fill, .. } => {
                painter.rect_filled(Rect::from_min_max(map(rect.min), map(rect.max)), 0.0, *fill);
            }
            Prim::Line { a, b, stroke } => {
                painter.line_segment([map(*a), map(*b)], Stroke::new(1.0, stroke.color));
            }
            Prim::Text(t) => {
                for row in &t.galley.rows {
                    let (Some(first), Some(last)) = (row.glyphs.first(), row.glyphs.last()) else { continue };
                    let color = t.galley.job.sections.first().map_or(c.text, |s| s.format.color);
                    let x0 = t.pos.x + row.pos.x + first.pos.x;
                    let x1 = t.pos.x + row.pos.x + last.pos.x + last.advance_width;
                    let cy = t.pos.y + row.pos.y + row.size.y * 0.55;
                    let h = (row.size.y * 0.32).max(1.0 / k);
                    let r = Rect::from_min_max(map(Pos2::new(x0, cy - h / 2.0)), map(Pos2::new(x1, cy + h / 2.0)));
                    painter.rect_filled(r, 0.0, color.gamma_multiply(0.55));
                }
            }
            Prim::Image { rect, uri, .. } => {
                paint_image(ui.ctx(), &painter, uri, Rect::from_min_max(map(rect.min), map(rect.max)));
            }
            Prim::Placeholder { rect, .. } | Prim::Check { rect, .. } => {
                painter.rect_filled(Rect::from_min_max(map(rect.min), map(rect.max)), 0.0, c.code_bg);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_and_snippets() {
        assert_eq!(word_at("hello brave world", 8), (6, 11));
        assert_eq!(word_at("a, b", 1), (1, 2));
        let text = "x".repeat(100) + "needle" + &"y".repeat(100);
        let s = snippet(&text, 100..106);
        assert!(s.starts_with('…') && s.ends_with('…') && s.contains("needle"));
    }

    #[test]
    fn selection_order() {
        let s = Selection { anchor: TextPos { prim: 3, ch: 1 }, focus: TextPos { prim: 1, ch: 5 } };
        assert_eq!(s.ordered().0.prim, 1);
    }
}
