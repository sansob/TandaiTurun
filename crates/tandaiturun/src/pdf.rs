//! Export to PDF, drawn from the same page layout as the screen.
//!
//! Text is real text (subset TrueType fonts with a ToUnicode map, so it can
//! be selected, searched and copied), links stay clickable, and the headings
//! become the PDF's bookmarks.

use crate::fonts;
use crate::layout::{self, char_rects, Layout, Params, Prim};
use crate::theme::PageTheme;
use crate::view::Look;
use egui::{Color32, FontFamily, Galley, Pos2, Rect as ERect};
use pdf_writer::types::{ActionType, AnnotationType, CidFontType, FontFlags, SystemInfo};
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref, Str, TextStr};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

struct FontUse {
    name: &'static str,
    face: ttf_parser::Face<'static>,
    remap: subsetter::GlyphRemapper,
    /// New glyph id → the text it stands for.
    text: BTreeMap<u16, char>,
    res: String,
}

struct Ids(i32);

impl Ids {
    fn next(&mut self) -> Ref {
        self.0 += 1;
        Ref::new(self.0)
    }
}

fn rgb(c: Color32, paper: Color32) -> (f32, f32, f32) {
    // Blend translucent colours over the paper: PDF fills here are opaque.
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    let a = a as f32 / 255.0;
    let [pr, pg, pb, _] = paper.to_srgba_unmultiplied();
    let mix = |x: u8, p: u8| (x as f32 * a + p as f32 * (1.0 - a)) / 255.0;
    (mix(r, pr), mix(g, pg), mix(b, pb))
}

fn deflate(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(data, 6)
}

/// A font tag like `ABCDEF+` from the font name and its glyphs.
fn subset_tag(name: &str, glyphs: usize) -> String {
    let mut h: u32 = 2166136261;
    for b in name.bytes().chain(glyphs.to_le_bytes()) {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    (0..6).map(|i| (b'A' + ((h >> (i * 5)) % 26) as u8) as char).collect()
}

pub struct Exporter {
    fonts: Vec<FontUse>,
    by_name: HashMap<&'static str, usize>,
}

impl Exporter {
    fn font_for(&mut self, family: &FontFamily, ch: char) -> Option<(usize, u16)> {
        for name in fonts::chain(family) {
            let idx = match self.by_name.get(name) {
                Some(i) => *i,
                None => {
                    let face = ttf_parser::Face::parse(fonts::bytes(name), 0).ok()?;
                    let res = format!("F{}", self.fonts.len() + 1);
                    let mut remap = subsetter::GlyphRemapper::new();
                    remap.remap(0);
                    self.fonts.push(FontUse { name, face, remap, text: BTreeMap::new(), res });
                    self.by_name.insert(name, self.fonts.len() - 1);
                    self.fonts.len() - 1
                }
            };
            let f = &mut self.fonts[idx];
            if let Some(gid) = f.face.glyph_index(ch) {
                let new = f.remap.remap(gid.0);
                f.text.entry(new).or_insert(ch);
                return Some((idx, new));
            }
        }
        None
    }
}

/// Glyphs drawn with one font, size, colour and baseline: (font, size, colour, baseline, [(glyph, x)]).
type Run = (usize, f32, Color32, f32, Vec<(u16, f32)>);

/// Draws one text galley. `to_pdf` maps layout points to PDF points.
fn text_ops(
    ex: &mut Exporter,
    c: &mut Content,
    pos: Pos2,
    galley: &Galley,
    k: f32,
    to_pdf: &dyn Fn(Pos2) -> (f32, f32),
    paper: Color32,
) {
    let job = &galley.job;
    let section_at = |byte: usize| {
        let i = job.sections.partition_point(|s| s.byte_range.end.0 <= byte);
        &job.sections[i.min(job.sections.len().saturating_sub(1))]
    };
    let byte_of: Vec<usize> = job.text.char_indices().map(|(b, _)| b).collect();
    // Backgrounds and decorations first, then the glyphs on top.
    let mut ci = 0usize;
    let mut runs: Vec<Run> = Vec::new();
    for row in &galley.rows {
        let origin = pos + row.pos.to_vec2();
        for (gi, g) in row.glyphs.iter().enumerate() {
            let Some(&byte) = byte_of.get(ci + gi) else { break };
            let sec = section_at(byte);
            let f = &sec.format;
            let x0 = origin.x + g.pos.x;
            let x1 = x0 + g.advance_width;
            if f.background != Color32::TRANSPARENT {
                let r = ERect::from_min_max(Pos2::new(x0, origin.y), Pos2::new(x1, origin.y + row.size.y));
                let (a, b) = (to_pdf(r.left_bottom()), to_pdf(r.right_top()));
                let (cr, cg, cb) = rgb(f.background, paper);
                c.set_fill_rgb(cr, cg, cb);
                c.rect(a.0, a.1, b.0 - a.0, b.1 - a.1);
                c.fill_nonzero();
            }
            let size = f.font_id.size;
            for (stroke, dy) in [(f.underline, 0.15 * size), (f.strikethrough, -0.3 * size)] {
                if stroke.width > 0.0 {
                    let y = origin.y + g.pos.y + dy;
                    let (a, b) = (to_pdf(Pos2::new(x0, y)), to_pdf(Pos2::new(x1, y)));
                    let (cr, cg, cb) = rgb(stroke.color, paper);
                    c.set_stroke_rgb(cr, cg, cb);
                    c.set_line_width((stroke.width * k).max(0.3));
                    c.move_to(a.0, a.1);
                    c.line_to(b.0, b.1);
                    c.stroke();
                }
            }
            if g.chr.is_whitespace() && g.chr != ' ' {
                continue;
            }
            let Some((font, gid)) = ex.font_for(&f.font_id.family, g.chr) else { continue };
            let baseline = origin.y + g.pos.y;
            let same = runs.last().is_some_and(|r| r.0 == font && r.1 == size && r.2 == f.color && r.3 == baseline);
            if !same {
                runs.push((font, size, f.color, baseline, Vec::new()));
            }
            runs.last_mut().expect("run").4.push((gid, x0));
        }
        ci += row.char_count_including_newline().0;
    }
    for (font, size, color, baseline, glyphs) in runs {
        let Some(&(_, first_x)) = glyphs.first() else { continue };
        let fu = &ex.fonts[font];
        let upem = fu.face.units_per_em() as f32;
        let (cr, cg, cb) = rgb(color, paper);
        let pt = size * k;
        let (x, y) = to_pdf(Pos2::new(first_x, baseline));
        c.set_fill_rgb(cr, cg, cb);
        c.begin_text();
        c.set_font(Name(fu.res.as_bytes()), pt);
        c.set_text_matrix([1.0, 0.0, 0.0, 1.0, x, y]);
        {
            let mut shown = c.show_positioned();
            let mut items = shown.items();
            // Where the PDF viewer will put the next glyph, in layout points.
            let mut pen = first_x;
            for (gid, gx) in glyphs {
                let diff = gx - pen;
                if diff.abs() > 0.01 {
                    items.adjust(-diff / size * 1000.0);
                }
                items.show(Str(&gid.to_be_bytes()));
                let adv = advance_of(fu, gid) / upem * size;
                pen = gx + adv;
            }
        }
        c.end_text();
    }
}

/// The advance width of a remapped glyph, in font units.
fn advance_of(fu: &FontUse, new_gid: u16) -> f32 {
    // GlyphRemapper only maps old → new; find the old id from the kept text.
    fu.text
        .get(&new_gid)
        .and_then(|ch| fu.face.glyph_index(*ch))
        .and_then(|g| fu.face.glyph_hor_advance(g))
        .unwrap_or(0) as f32
}

struct ImageRes {
    name: String,
    id: Ref,
}

/// Bytes of an internet image the app already downloaded.
pub type Fetch<'a> = &'a dyn Fn(&str) -> Option<Vec<u8>>;

fn load_image(uri: &str, fetch: Fetch<'_>) -> Option<image::RgbaImage> {
    if uri.starts_with("http://") || uri.starts_with("https://") {
        let bytes = crate::remote::bytes(uri).map(|b| b.to_vec()).or_else(|| fetch(uri))?;
        if bytes.trim_ascii_start().starts_with(b"<") {
            return crate::mermaid::rasterize_svg(&bytes, 3.0);
        }
        return image::load_from_memory(&bytes).ok().map(|i| i.into_rgba8());
    }
    if let Some(png) = crate::mermaid::png_for(uri).or_else(|| crate::math::png_for(uri)) {
        return image::load_from_memory(&png).ok().map(|i| i.into_rgba8());
    }
    let path = uri.strip_prefix("file://")?;
    let path = if cfg!(windows) { path.trim_start_matches('/') } else { path };
    if path.to_ascii_lowercase().ends_with(".svg") || path.to_ascii_lowercase().ends_with(".svgz") {
        return crate::mermaid::rasterize_svg(&std::fs::read(path).ok()?, 3.0);
    }
    image::open(path).ok().map(|i| i.into_rgba8())
}

/// Renders the document to PDF bytes.
pub fn export(ctx: &egui::Context, markdown: &str, look: &Look, base_dir: Option<PathBuf>, title: &str) -> Vec<u8> {
    let params = Params {
        page: look.page,
        font: look.font,
        font_size: look.font_size,
        // Printing is always on white paper.
        theme: PageTheme::Paper,
        zoom: 1.0,
        base_dir: base_dir.clone(),
        pixels_per_point: ctx.pixels_per_point(),
        allow_remote: look.allow_remote,
    };
    let l = layout::layout(ctx, markdown, &params);
    let fetch = |uri: &str| match ctx.try_load_bytes(uri) {
        Ok(egui::load::BytesPoll::Ready { bytes, .. }) => Some(bytes.to_vec()),
        _ => None,
    };
    render(&l, markdown, title, &fetch)
}

pub fn render(l: &Layout, markdown: &str, title: &str, fetch: Fetch<'_>) -> Vec<u8> {
    let colors = l.params.theme.colors();
    let paper = colors.paper;
    let (pw, ph) = l.params.page.pdf_points();
    let k = pw / l.page_size.x;
    let mut ids = Ids(0);
    let catalog = ids.next();
    let tree = ids.next();
    let info = ids.next();
    let mut pdf = Pdf::new();
    let mut ex = Exporter { fonts: Vec::new(), by_name: HashMap::new() };
    let mut images: HashMap<String, ImageRes> = HashMap::new();
    let mut image_data: Vec<(Ref, image::RgbaImage)> = Vec::new();
    let page_ids: Vec<Ref> = (0..l.pages.len()).map(|_| ids.next()).collect();
    let mut page_contents = Vec::new();
    let mut page_annots: Vec<Vec<Ref>> = vec![Vec::new(); l.pages.len()];
    let mut annots: Vec<(Ref, Rect, String)> = Vec::new();

    for (pi, span) in l.pages.iter().enumerate() {
        let mut c = Content::new();
        let origin_y = l.margin - span.start;
        let to_pdf = |p: Pos2| ((p.x + l.margin) * k, ph - (p.y + origin_y) * k);
        // Page background (only needed for coloured paper).
        if paper != Color32::WHITE {
            let (r, g, b) = rgb(paper, paper);
            c.set_fill_rgb(r, g, b);
            c.rect(0.0, 0.0, pw, ph);
            c.fill_nonzero();
        }
        c.save_state();
        let top = ph - l.margin * k;
        let h = (span.end - span.start + 1.0) * k;
        c.rect(0.0, top - h, pw, h + 0.5);
        c.clip_nonzero();
        c.end_path();
        for &i in &l.page_prims[pi] {
            match &l.prims[i] {
                Prim::Rect { rect, fill, stroke, .. } => {
                    let (a, b) = (to_pdf(rect.left_bottom()), to_pdf(rect.right_top()));
                    if *fill != Color32::TRANSPARENT {
                        let (r, g, bl) = rgb(*fill, paper);
                        c.set_fill_rgb(r, g, bl);
                        c.rect(a.0, a.1, b.0 - a.0, b.1 - a.1);
                        c.fill_nonzero();
                    }
                    if stroke.width > 0.0 {
                        let (r, g, bl) = rgb(stroke.color, paper);
                        c.set_stroke_rgb(r, g, bl);
                        c.set_line_width((stroke.width * k).max(0.3));
                        c.rect(a.0, a.1, b.0 - a.0, b.1 - a.1);
                        c.stroke();
                    }
                }
                Prim::Line { a, b, stroke } => {
                    let (a, b) = (to_pdf(*a), to_pdf(*b));
                    let (r, g, bl) = rgb(stroke.color, paper);
                    c.set_stroke_rgb(r, g, bl);
                    c.set_line_width((stroke.width * k).max(0.3));
                    c.move_to(a.0, a.1);
                    c.line_to(b.0, b.1);
                    c.stroke();
                }
                Prim::Placeholder { rect, .. } => {
                    let (a, b) = (to_pdf(rect.left_bottom()), to_pdf(rect.right_top()));
                    let (r, g, bl) = rgb(colors.code_border, paper);
                    c.set_stroke_rgb(r, g, bl);
                    c.set_line_width(0.75);
                    c.rect(a.0, a.1, b.0 - a.0, b.1 - a.1);
                    c.stroke();
                }
                Prim::Check { rect, checked } => {
                    let (a, b) = (to_pdf(rect.left_bottom()), to_pdf(rect.right_top()));
                    let (w, hh) = (b.0 - a.0, b.1 - a.1);
                    if *checked {
                        let (r, g, bl) = rgb(crate::theme::ACCENT, paper);
                        c.set_fill_rgb(r, g, bl);
                        c.rect(a.0, a.1, w, hh);
                        c.fill_nonzero();
                        c.set_stroke_rgb(1.0, 1.0, 1.0);
                        c.set_line_width(1.2);
                        c.move_to(a.0 + w * 0.22, a.1 + hh * 0.48);
                        c.line_to(a.0 + w * 0.42, a.1 + hh * 0.28);
                        c.line_to(a.0 + w * 0.78, a.1 + hh * 0.7);
                        c.stroke();
                    } else {
                        let (r, g, bl) = rgb(colors.muted, paper);
                        c.set_stroke_rgb(r, g, bl);
                        c.set_line_width(0.9);
                        c.rect(a.0, a.1, w, hh);
                        c.stroke();
                    }
                }
                Prim::Image { rect, uri, link } => {
                    if !images.contains_key(uri) {
                        if let Some(img) = load_image(uri, fetch) {
                            let id = ids.next();
                            images.insert(uri.clone(), ImageRes { name: format!("Im{}", images.len() + 1), id });
                            image_data.push((id, img));
                        }
                    }
                    if let Some(res) = images.get(uri) {
                        let (a, b) = (to_pdf(rect.left_bottom()), to_pdf(rect.right_top()));
                        c.save_state();
                        c.transform([b.0 - a.0, 0.0, 0.0, b.1 - a.1, a.0, a.1]);
                        c.x_object(Name(res.name.as_bytes()));
                        c.restore_state();
                    }
                    if let Some(url) = link {
                        let (a, b) = (to_pdf(rect.left_bottom()), to_pdf(rect.right_top()));
                        let id = ids.next();
                        page_annots[pi].push(id);
                        annots.push((id, Rect::new(a.0, a.1, b.0, b.1), url.clone()));
                    }
                }
                Prim::Text(_) => {}
            }
        }
        for &i in &l.page_prims[pi] {
            if let Prim::Text(t) = &l.prims[i] {
                text_ops(&mut ex, &mut c, t.pos, &t.galley, k, &to_pdf, paper);
                for (range, url) in &t.links {
                    let text = &t.galley.job.text;
                    let chars = layout::byte_to_char(text, range.start)..layout::byte_to_char(text, range.end);
                    for r in char_rects(&t.galley, chars) {
                        let r = r.translate(t.pos.to_vec2());
                        if r.bottom() <= span.start || r.top() >= span.end {
                            continue;
                        }
                        let (a, b) = (to_pdf(r.left_bottom()), to_pdf(r.right_top()));
                        let id = ids.next();
                        page_annots[pi].push(id);
                        annots.push((id, Rect::new(a.0, a.1, b.0, b.1), url.clone()));
                    }
                }
            }
        }
        c.restore_state();
        page_contents.push(c.finish());
    }

    // Pages.
    let mut font_refs: Vec<(Ref, Ref, Ref, Ref, Ref)> = Vec::new();
    for _ in &ex.fonts {
        font_refs.push((ids.next(), ids.next(), ids.next(), ids.next(), ids.next()));
    }
    for (pi, content) in page_contents.into_iter().enumerate() {
        let content_id = ids.next();
        pdf.stream(content_id, &deflate(&content)).filter(Filter::FlateDecode);
        let mut page = pdf.page(page_ids[pi]);
        page.media_box(Rect::new(0.0, 0.0, pw, ph));
        page.parent(tree);
        page.contents(content_id);
        if !page_annots[pi].is_empty() {
            page.annotations(page_annots[pi].iter().copied());
        }
        let mut res = page.resources();
        {
            let mut f = res.fonts();
            for (fu, refs) in ex.fonts.iter().zip(&font_refs) {
                f.pair(Name(fu.res.as_bytes()), refs.0);
            }
        }
        {
            let mut x = res.x_objects();
            for r in images.values() {
                x.pair(Name(r.name.as_bytes()), r.id);
            }
        }
        res.finish();
        page.finish();
    }
    pdf.pages(tree).kids(page_ids.iter().copied()).count(page_ids.len() as i32);

    // Links: web links open the URL, `#anchor` jumps inside the document.
    for (id, rect, url) in annots {
        let mut a = pdf.annotation(id);
        a.subtype(AnnotationType::Link);
        a.rect(rect);
        a.border(0.0, 0.0, 0.0, None);
        if let Some(anchor) = url.strip_prefix('#') {
            if let Some(&y) = l.anchors.get(anchor) {
                let p = l.page_of(y);
                let top = ph - (y - l.pages[p].start + l.margin) * k;
                a.action().action_type(ActionType::GoTo).destination().page(page_ids[p]).xyz(0.0, top + 6.0, None);
            }
        } else {
            a.action().action_type(ActionType::Uri).uri(Str(url.as_bytes()));
        }
        a.finish();
    }

    // Images: RGB samples plus an alpha soft mask.
    for (id, img) in image_data {
        let (w, h) = img.dimensions();
        let mut rgb_data = Vec::with_capacity((w * h * 3) as usize);
        let mut alpha = Vec::with_capacity((w * h) as usize);
        for p in img.pixels() {
            rgb_data.extend_from_slice(&p.0[..3]);
            alpha.push(p.0[3]);
        }
        let has_alpha = alpha.iter().any(|a| *a != 255);
        let mask_id = has_alpha.then(|| ids.next());
        let data = deflate(&rgb_data);
        let mut x = pdf.image_xobject(id, &data);
        x.filter(Filter::FlateDecode);
        x.width(w as i32);
        x.height(h as i32);
        x.color_space().device_rgb();
        x.bits_per_component(8);
        if let Some(m) = mask_id {
            x.s_mask(m);
        }
        x.finish();
        if let Some(m) = mask_id {
            let data = deflate(&alpha);
            let mut x = pdf.image_xobject(m, &data);
            x.filter(Filter::FlateDecode);
            x.width(w as i32);
            x.height(h as i32);
            x.color_space().device_gray();
            x.bits_per_component(8);
            x.finish();
        }
    }

    // Fonts: subset TrueType, CID-keyed, Identity-H, with a ToUnicode map.
    for (fu, (type0, cid, desc, file, cmap)) in ex.fonts.iter().zip(font_refs) {
        let data = fonts::bytes(fu.name);
        let Ok(sub) = subsetter::subset(data, 0, &fu.remap) else { continue };
        let base = format!("{}+{}", subset_tag(fu.name, fu.text.len()), fu.name.replace(' ', ""));
        let base = Name(base.as_bytes());
        let upem = fu.face.units_per_em() as f32;
        let to1000 = |v: f32| v / upem * 1000.0;
        pdf.type0_font(type0)
            .base_font(base)
            .encoding_predefined(Name(b"Identity-H"))
            .descendant_font(cid)
            .to_unicode(cmap);
        let info_sys = SystemInfo { registry: Str(b"Adobe"), ordering: Str(b"Identity"), supplement: 0 };
        {
            let mut cf = pdf.cid_font(cid);
            cf.subtype(CidFontType::Type2);
            cf.base_font(base);
            cf.system_info(info_sys);
            cf.font_descriptor(desc);
            cf.default_width(0.0);
            cf.cid_to_gid_map_predefined(Name(b"Identity"));
            let n = fu.remap.num_gids();
            let widths: Vec<f32> = (0..n).map(|g| to1000(advance_of(fu, g))).collect();
            cf.widths().consecutive(0, widths);
            cf.finish();
        }
        let bb = fu.face.global_bounding_box();
        let italic = fu.face.italic_angle();
        let mut flags = FontFlags::NON_SYMBOLIC;
        if fu.name.contains("Serif") {
            flags |= FontFlags::SERIF;
        }
        if fu.name.contains("Mono") {
            flags |= FontFlags::FIXED_PITCH;
        }
        if italic != 0.0 {
            flags |= FontFlags::ITALIC;
        }
        pdf.font_descriptor(desc)
            .name(base)
            .flags(flags)
            .bbox(Rect::new(
                to1000(bb.x_min as f32),
                to1000(bb.y_min as f32),
                to1000(bb.x_max as f32),
                to1000(bb.y_max as f32),
            ))
            .italic_angle(italic)
            .ascent(to1000(fu.face.ascender() as f32))
            .descent(to1000(fu.face.descender() as f32))
            .cap_height(to1000(fu.face.capital_height().unwrap_or(fu.face.ascender()) as f32))
            .stem_v(80.0)
            .font_file2(file);
        pdf.stream(file, &deflate(&sub)).filter(Filter::FlateDecode);
        let mut uc = pdf_writer::types::UnicodeCmap::new(Name(b"Custom"), info_sys);
        for (g, ch) in &fu.text {
            uc.pair(*g, *ch);
        }
        pdf.cmap(cmap, &uc.finish());
    }

    // Bookmarks from the headings.
    let headings = tandaiturun_core::outline::outline(markdown);
    let mut outline_root = None;
    if !headings.is_empty() && headings.len() == l.headings.len() {
        let root = ids.next();
        let item_ids: Vec<Ref> = headings.iter().map(|_| ids.next()).collect();
        // Parent of each heading: the closest earlier heading of a lower level.
        let parents: Vec<Option<usize>> =
            (0..headings.len()).map(|i| (0..i).rev().find(|&j| headings[j].level < headings[i].level)).collect();
        let children = |p: Option<usize>| -> Vec<usize> { (0..headings.len()).filter(|&i| parents[i] == p).collect() };
        for (i, h) in headings.iter().enumerate() {
            let siblings = children(parents[i]);
            let pos = siblings.iter().position(|&s| s == i).unwrap_or(0);
            let kids = children(Some(i));
            let y = l.headings[i];
            let p = l.page_of(y);
            let top = ph - (y - l.pages[p].start + l.margin) * k;
            let mut it = pdf.outline_item(item_ids[i]);
            it.title(TextStr(&h.text));
            it.parent(parents[i].map_or(root, |p| item_ids[p]));
            if pos > 0 {
                it.prev(item_ids[siblings[pos - 1]]);
            }
            if let Some(&n) = siblings.get(pos + 1) {
                it.next(item_ids[n]);
            }
            if let (Some(&f), Some(&la)) = (kids.first(), kids.last()) {
                it.first(item_ids[f]);
                it.last(item_ids[la]);
                it.count(kids.len() as i32);
            }
            it.dest().page(page_ids[p]).xyz(0.0, top + 6.0, None);
            it.finish();
        }
        let top_level = children(None);
        let mut o = pdf.outline(root);
        if let (Some(&f), Some(&la)) = (top_level.first(), top_level.last()) {
            o.first(item_ids[f]);
            o.last(item_ids[la]);
        }
        o.count(top_level.len() as i32);
        o.finish();
        outline_root = Some(root);
    }

    let mut cat = pdf.catalog(catalog);
    cat.pages(tree);
    if let Some(r) = outline_root {
        cat.outlines(r);
        cat.pair(Name(b"PageMode"), Name(b"UseOutlines"));
    }
    cat.finish();
    pdf.document_info(info)
        .title(TextStr(title))
        .creator(TextStr("TandaiTurun"))
        .producer(TextStr(concat!("TandaiTurun ", env!("CARGO_PKG_VERSION"))));
    pdf.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_six_capitals() {
        let t = subset_tag("Inter", 42);
        assert_eq!(t.len(), 6);
        assert!(t.chars().all(|c| c.is_ascii_uppercase()));
    }

    #[test]
    fn renders_a_pdf_with_text_fonts_links_and_bookmarks() {
        let md = "# Title\n\nHello **PDF** with [a link](https://example.com) and `code`.\n\n## Second\n\nÜmlaut, emoji ✔.\n";
        crate::layout::tests::with_ctx(|ctx| {
            let l = crate::layout::layout(ctx, md, &crate::layout::tests::params(1.0));
            let bytes = render(&l, md, "Title", &|_| None);
            let s = String::from_utf8_lossy(&bytes);
            assert!(bytes.starts_with(b"%PDF-"));
            assert!(s.contains("/FontFile2"));
            assert!(s.contains("/ToUnicode"));
            assert!(s.contains("Identity-H"));
            assert!(s.contains("(https://example.com)"));
            assert!(s.contains("/Outlines"));
            assert!(s.contains("/Count 1"), "one top-level bookmark");
            assert!(bytes.len() < 200_000, "fonts are subset: {} bytes", bytes.len());
        });
    }

    #[test]
    fn colours_blend_over_paper() {
        let (r, g, b) = rgb(Color32::from_rgba_unmultiplied(0, 0, 0, 128), Color32::WHITE);
        assert!((r - 0.498).abs() < 0.01 && r == g && g == b);
    }
}
