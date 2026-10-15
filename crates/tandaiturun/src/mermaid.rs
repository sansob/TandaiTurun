//! Mermaid diagrams from ```` ```mermaid ```` code blocks, drawn without a
//! browser: `mermaid-rs-renderer` turns the source into SVG, which is
//! rasterised with the app's own fonts and shown like any other image (on the
//! page, in the thumbnails and in the PDF).

use egui::{Color32, Vec2};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

pub struct Diagram {
    /// The `bytes://` URI the PNG is registered under with egui.
    pub uri: String,
    /// Natural size in points.
    pub size: Vec2,
    pub png: Arc<[u8]>,
}

type Cache = HashMap<u64, Result<Arc<Diagram>, String>>;

fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Fonts for the SVG text: the bundled ones, never the system's.
fn fontdb() -> Arc<resvg::usvg::fontdb::Database> {
    static DB: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = resvg::usvg::fontdb::Database::new();
        for data in [crate::fonts::INTER, crate::fonts::INTER_BOLD, crate::fonts::INTER_ITALIC, crate::fonts::MONO] {
            db.load_font_data(data.to_vec());
        }
        db.set_sans_serif_family("Inter");
        db.set_serif_family("Inter");
        db.set_monospace_family("JetBrains Mono NL");
        Arc::new(db)
    })
    .clone()
}

/// Pixels per point of the raster: sharp up to about 250 % zoom.
const SCALE: f32 = 2.5;
const MAX_PIXELS: f32 = 4096.0;

fn css(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

/// A diagram as SVG for the HTML export (light theme, page background).
pub fn svg(src: &str) -> Option<String> {
    let mut opts = mermaid_rs_renderer::RenderOptions::default();
    opts.theme.background = "transparent".into();
    opts.layout.fast_text_metrics = true;
    let svg = mermaid_rs_renderer::render_with_options(src, opts).ok()?;
    Some(svg[svg.find("<svg")?..].to_string())
}

fn render_png(src: &str, dark: bool, background: Color32) -> Result<(Vec2, Vec<u8>), String> {
    let mut opts = mermaid_rs_renderer::RenderOptions::default();
    if dark {
        opts.theme = mermaid_rs_renderer::Theme::dark();
    }
    opts.theme.background = css(background);
    // Calibrated text widths: no font scan of the system, nothing written to disk.
    opts.layout.fast_text_metrics = true;
    let svg = mermaid_rs_renderer::render_with_options(src, opts).map_err(|e| e.to_string())?;
    let options = resvg::usvg::Options { fontdb: fontdb(), font_family: "Inter".into(), ..Default::default() };
    let tree = resvg::usvg::Tree::from_str(&svg, &options).map_err(|e| e.to_string())?;
    let size = Vec2::new(tree.size().width(), tree.size().height());
    if size.x < 1.0 || size.y < 1.0 {
        return Err("empty diagram".into());
    }
    let scale = SCALE.min(MAX_PIXELS / size.x).min(MAX_PIXELS / size.y);
    let (w, h) = ((size.x * scale).ceil() as u32, (size.y * scale).ceil() as u32);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).ok_or("diagram too large")?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let png = pixmap.encode_png().map_err(|e| e.to_string())?;
    Ok((size, png))
}

/// Rasterises an SVG file with the bundled fonts (for the PDF export).
pub fn rasterize_svg(data: &[u8], scale: f32) -> Option<image::RgbaImage> {
    let options = resvg::usvg::Options { fontdb: fontdb(), font_family: "Inter".into(), ..Default::default() };
    let tree = resvg::usvg::Tree::from_data(data, &options).ok()?;
    let size = tree.size();
    let scale = scale.min(MAX_PIXELS / size.width()).min(MAX_PIXELS / size.height());
    let (w, h) = ((size.width() * scale).ceil() as u32, (size.height() * scale).ceil() as u32);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w.max(1), h.max(1))?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    // tiny-skia pixels are premultiplied; PNG round trip un-premultiplies them.
    image::load_from_memory(&pixmap.encode_png().ok()?).ok().map(|i| i.into_rgba8())
}

/// SVG images (local files and allowed internet ones) for egui, drawn with
/// the bundled fonts at the diagrams' resolution so they stay sharp when
/// zoomed in. The texture keeps the SVG's own size for the layout.
struct SvgLoader {
    cache: Mutex<HashMap<String, Result<Arc<egui::ColorImage>, String>>>,
}

impl egui::load::ImageLoader for SvgLoader {
    fn id(&self) -> &str {
        egui::generate_loader_id!(SvgLoader)
    }

    fn load(&self, ctx: &egui::Context, uri: &str, _: egui::SizeHint) -> egui::load::ImageLoadResult {
        use egui::load::{BytesPoll, ImagePoll, LoadError};
        if !egui::load::has_extension(uri, "svg") && !uri.ends_with(".svg") {
            return Err(LoadError::NotSupported);
        }
        if let Some(r) = self.cache.lock().ok().and_then(|c| c.get(uri).cloned()) {
            return r.map(|image| ImagePoll::Ready { image }).map_err(LoadError::Loading);
        }
        match ctx.try_load_bytes(uri)? {
            BytesPoll::Pending { size } => Ok(ImagePoll::Pending { size }),
            BytesPoll::Ready { bytes, .. } => {
                let result = svg_image(&bytes).map(Arc::new).ok_or_else(|| "unreadable SVG".to_string());
                if let Ok(mut c) = self.cache.lock() {
                    c.insert(uri.to_owned(), result.clone());
                }
                result.map(|image| ImagePoll::Ready { image }).map_err(LoadError::Loading)
            }
        }
    }

    fn forget(&self, uri: &str) {
        if let Ok(mut c) = self.cache.lock() {
            c.remove(uri);
        }
    }

    fn forget_all(&self) {
        if let Ok(mut c) = self.cache.lock() {
            c.clear();
        }
    }

    fn byte_size(&self) -> usize {
        self.cache.lock().map_or(0, |c| c.values().map(|r| r.as_ref().map_or(0, |i| i.pixels.len() * 4)).sum())
    }
}

fn svg_image(data: &[u8]) -> Option<egui::ColorImage> {
    let options = resvg::usvg::Options { fontdb: fontdb(), font_family: "Inter".into(), ..Default::default() };
    let size = resvg::usvg::Tree::from_data(data, &options).ok()?.size();
    let img = rasterize_svg(data, SCALE)?;
    let pixels = [img.width() as usize, img.height() as usize];
    Some(
        egui::ColorImage::from_rgba_unmultiplied(pixels, img.as_raw())
            .with_source_size(Vec2::new(size.width(), size.height())),
    )
}

/// Lets egui show SVG images (see [`SvgLoader`]).
pub fn install_svg_loader(ctx: &egui::Context) {
    ctx.add_image_loader(Arc::new(SvgLoader { cache: Mutex::new(HashMap::new()) }));
}

/// Renders (or fetches from the cache) a diagram and registers its image with egui.
pub fn diagram(ctx: &egui::Context, src: &str, dark: bool, background: Color32) -> Result<Arc<Diagram>, String> {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (src, dark, background.to_array()).hash(&mut h);
    let key = h.finish();
    let cached = cache().lock().ok().and_then(|c| c.get(&key).cloned());
    let result = match cached {
        Some(r) => r,
        None => {
            let r = render_png(src, dark, background).map(|(size, png)| {
                Arc::new(Diagram { uri: format!("bytes://mermaid-{key:016x}.png"), size, png: png.into() })
            });
            if let Ok(mut c) = cache().lock() {
                c.insert(key, r.clone());
            }
            r
        }
    };
    if let Ok(d) = &result {
        // Cheap when already registered; needed once per egui context.
        if ctx.try_load_bytes(&d.uri).is_err() {
            ctx.include_bytes(d.uri.clone(), d.png.clone());
        }
    }
    result
}

/// The PNG of a rendered diagram, for the PDF export.
pub fn png_for(uri: &str) -> Option<Arc<[u8]>> {
    let c = cache().lock().ok()?;
    c.values().find_map(|r| r.as_ref().ok().filter(|d| d.uri == uri).map(|d| d.png.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_common_diagrams() {
        for src in [
            "flowchart LR\n  A[Start] --> B{Ok?}\n  B -->|yes| C[Done]\n  B -->|no| A",
            "sequenceDiagram\n  Alice->>Bob: Hi\n  Bob-->>Alice: Hello",
            "pie title Pets\n  \"Dogs\" : 40\n  \"Cats\" : 60",
        ] {
            let (size, png) = render_png(src, false, Color32::WHITE).unwrap_or_else(|e| panic!("{src}: {e}"));
            assert!(size.x > 20.0 && size.y > 20.0, "{src}: {size:?}");
            assert!(png.starts_with(b"\x89PNG"));
        }
    }

    #[test]
    fn syntax_errors_are_reported() {
        assert!(render_png("this is not mermaid", false, Color32::WHITE).is_err());
    }
}
