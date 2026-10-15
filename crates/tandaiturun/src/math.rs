//! LaTeX math (`$…$`, `$$…$$`) typeset with `latex-rust` (TeX layout rules,
//! STIX Two Math glyphs), rasterised once and placed like an image: inline
//! formulas sit on the text baseline, display formulas are centred blocks.

use egui::Color32;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::sync::{Arc, OnceLock};

/// A typeset formula. Sizes are in ems of the surrounding text.
pub struct Formula {
    pub uri: String,
    pub width: f32,
    /// Above the baseline.
    pub height: f32,
    /// Below the baseline.
    pub depth: f32,
    pub png: Arc<[u8]>,
}

type Cache = HashMap<u64, Result<Arc<Formula>, String>>;

fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn font() -> Result<&'static latex_rust::MathFont, String> {
    static FONT: OnceLock<Result<latex_rust::MathFont, String>> = OnceLock::new();
    FONT.get_or_init(|| latex_rust::MathFont::stix_two_math().map_err(|e| e.to_string())).as_ref().map_err(Clone::clone)
}

fn f(d: &latex_rust::Dim) -> f32 {
    f32::from_bits(d.to_ieee32_bits())
}

/// Raster resolution: pixels per em. Sharp up to about 300 % zoom at 16 pt.
const PX_PER_EM: f32 = 64.0;

fn typeset(tex: &str, display: bool, color: Color32) -> Result<Formula, String> {
    let font = font()?;
    let ast = latex_rust::parse(tex).map_err(|e| e.to_string())?;
    let style = if display { latex_rust::MathStyle::Display } else { latex_rust::MathStyle::Text };
    let tree = latex_rust::layout(&ast, font, style).map_err(|e| e.to_string())?;
    let (width, height, depth) = (f(&tree.width), f(&tree.height), f(&tree.depth));
    if width.is_nan() || width <= 0.0 || (height + depth).is_nan() || height + depth <= 0.0 {
        return Err("empty formula".into());
    }
    let opts = latex_rust::SvgOptions {
        font_size_pt: latex_rust::Dim::from_i64(PX_PER_EM as i64),
        color: latex_rust::Color::rgb(color.r(), color.g(), color.b()),
        display,
    };
    let svg = latex_rust::render_svg(&tree, font, &opts).map_err(|e| e.to_string())?;
    // `pt` in the SVG are user units here: 1 em = PX_PER_EM pixels.
    let svg = svg.replacen("pt\" height=", "\" height=", 1).replacen("pt\" viewBox=", "\" viewBox=", 1);
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(&svg, &options).map_err(|e| e.to_string())?;
    let (w, h) = (tree.size().width().ceil() as u32, tree.size().height().ceil() as u32);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w.max(1), h.max(1)).ok_or("formula too large")?;
    resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    let png = pixmap.encode_png().map_err(|e| e.to_string())?;
    Ok(Formula { uri: String::new(), width, height, depth, png: png.into() })
}

/// A formula as SVG for the HTML export: glyphs are paths (no font needed),
/// sized in `em` to follow the surrounding text, inline ones on the baseline.
pub fn svg(tex: &str, display: bool) -> Option<String> {
    let font = font().ok()?;
    let ast = latex_rust::parse(tex).ok()?;
    let style = if display { latex_rust::MathStyle::Display } else { latex_rust::MathStyle::Text };
    let tree = latex_rust::layout(&ast, font, style).ok()?;
    let depth = f(&tree.depth);
    // At 10 pt, a size in pt divided by ten is a size in em.
    let opts = latex_rust::SvgOptions {
        font_size_pt: latex_rust::Dim::from_i64(10),
        color: latex_rust::Color::rgb(0x1f, 0x1f, 0x1f),
        display,
    };
    let svg = latex_rust::render_svg(&tree, font, &opts).ok()?;
    let svg = &svg[svg.find("<svg")?..];
    let attr = |name: &str| -> Option<f32> {
        let at = svg.find(&format!(" {name}=\""))? + name.len() + 3;
        svg[at..].split('"').next()?.trim_end_matches("pt").parse().ok()
    };
    let (w, h) = (attr("width")? / 10.0, attr("height")? / 10.0);
    let tag_end = svg.find('>')?;
    let rest = &svg[tag_end..];
    let align = if display { String::new() } else { format!(" style=\"vertical-align:{:.3}em\"", -depth) };
    Some(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.3}em\" height=\"{h:.3}em\" viewBox=\"0 0 {:.4} {:.4}\"{align} role=\"img\" aria-label=\"{}\"{rest}",
        w * 10.0,
        h * 10.0,
        tandaiturun_core::html::escape(tex),
    ))
}

/// Typesets (or fetches from the cache) a formula and registers its image with egui.
pub fn formula(ctx: &egui::Context, tex: &str, display: bool, color: Color32) -> Result<Arc<Formula>, String> {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (tex, display, color.to_array()).hash(&mut h);
    let key = h.finish();
    let cached = cache().lock().ok().and_then(|c| c.get(&key).cloned());
    let result = match cached {
        Some(r) => r,
        None => {
            let r = typeset(tex, display, color).map(|mut fm| {
                fm.uri = format!("bytes://math-{key:016x}.png");
                Arc::new(fm)
            });
            if let Ok(mut c) = cache().lock() {
                c.insert(key, r.clone());
            }
            r
        }
    };
    if let Ok(fm) = &result {
        if ctx.try_load_bytes(&fm.uri).is_err() {
            ctx.include_bytes(fm.uri.clone(), fm.png.clone());
        }
    }
    result
}

/// The PNG of a typeset formula, for the PDF export.
pub fn png_for(uri: &str) -> Option<Arc<[u8]>> {
    let c = cache().lock().ok()?;
    c.values().find_map(|r| r.as_ref().ok().filter(|fm| fm.uri == uri).map(|fm| fm.png.clone()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn svg_for_html_is_sized_in_em() {
        let s = super::svg("x^2", false).unwrap();
        assert!(s.starts_with("<svg xmlns="), "{s}");
        assert!(s.contains("em\" height=") && s.contains("vertical-align:"), "{s}");
        assert!(s.contains("<path"), "{s}");
        assert!(super::svg("\\frac{a}{b}", true).is_some_and(|s| !s.contains("vertical-align")));
    }
    use super::*;

    #[test]
    fn typesets_with_metrics() {
        let fm = typeset(r"\frac{a}{b} + \sqrt{x^2}", false, Color32::BLACK).unwrap();
        assert!(fm.width > 1.0 && fm.height > 0.5 && fm.depth > 0.1, "{} {} {}", fm.width, fm.height, fm.depth);
        assert!(fm.png.starts_with(b"\x89PNG"));
        let x = typeset("x", false, Color32::BLACK).unwrap();
        assert!(x.width < 1.0 && x.depth < 0.1);
        let big = typeset(r"\sum_{i=1}^{n} i = \frac{n(n+1)}{2}", true, Color32::BLACK).unwrap();
        assert!(big.height + big.depth > fm.height + fm.depth);
    }

    #[test]
    fn errors_are_reported() {
        assert!(typeset(r"\frac{", false, Color32::BLACK).is_err());
    }
}
