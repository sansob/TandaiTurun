//! HTML export: the core's HTML with what only the app can draw — formulas
//! and Mermaid diagrams as inline SVG, local images inlined as `data:` URLs,
//! so the file shows the same thing wherever it is opened or moved.

use crate::layout::{resolve_image, ImageSource};
use std::path::{Path, PathBuf};

struct Embed {
    base_dir: Option<PathBuf>,
}

impl tandaiturun_core::html::Embed for Embed {
    fn math(&self, tex: &str, display: bool) -> Option<String> {
        crate::math::svg(tex, display)
    }

    fn is_diagram(&self, lang: &str) -> bool {
        lang.eq_ignore_ascii_case("mermaid")
    }

    fn diagram(&self, _lang: &str, source: &str) -> Option<String> {
        crate::mermaid::svg(source)
    }

    fn image(&self, url: &str) -> Option<String> {
        let ImageSource::Local(uri) = resolve_image(url, self.base_dir.as_deref()) else { return None };
        let path = uri.strip_prefix("file:///").map_or_else(
            || uri.strip_prefix("file://").unwrap_or(&uri).to_string(),
            |p| if cfg!(windows) { p.to_string() } else { format!("/{p}") },
        );
        let bytes = std::fs::read(&path).ok()?;
        Some(format!("data:{};base64,{}", mime(Path::new(&path), &bytes)?, base64(&bytes)))
    }
}

fn mime(path: &Path, bytes: &[u8]) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "svg" => "image/svg+xml",
        _ if bytes.starts_with(b"\x89PNG") => "image/png",
        _ => return None,
    })
}

fn base64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, b)| n | (*b as u32) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The whole HTML file for a document whose images are relative to `base_dir`.
pub fn document(markdown: &str, title: &str, base_dir: Option<PathBuf>) -> String {
    tandaiturun_core::html::document_with(markdown, title, &Embed { base_dir })
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(super::base64(b""), "");
        assert_eq!(super::base64(b"f"), "Zg==");
        assert_eq!(super::base64(b"fo"), "Zm8=");
        assert_eq!(super::base64(b"foo"), "Zm9v");
        assert_eq!(super::base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn local_images_are_inlined_and_math_is_svg() {
        let dir = std::env::temp_dir().join(format!("tandaiturun-html-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("dot.png"), b"\x89PNG\r\n\x1a\nrest").unwrap();
        let html = super::document("![d](dot.png) $x^2$\n\n![w](https://example.com/a.png)\n", "t", Some(dir.clone()));
        assert!(html.contains("src=\"data:image/png;base64,iVBORw0KGgpyZXN0\""), "{html}");
        assert!(html.contains("src=\"https://example.com/a.png\""), "{html}");
        assert!(html.contains("<span class=\"math\"><svg"), "{html}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
