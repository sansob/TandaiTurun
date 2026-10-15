//! The fonts bundled into the app, all under the SIL Open Font License 1.1
//! (licence texts in `assets/fonts/`). egui's default fonts are not used, so
//! every component of a release is under an OSI-approved licence.
//!
//! The same font files are embedded into exported PDFs, so a PDF looks
//! exactly like the page on screen.

use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

pub const INTER: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
pub const INTER_BOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-Bold.ttf");
pub const INTER_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/Inter-Italic.ttf");
pub const INTER_BOLD_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/Inter-BoldItalic.ttf");
pub const INTER_SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf");
pub const SERIF: &[u8] = include_bytes!("../../../assets/fonts/LiberationSerif-Regular.ttf");
pub const SERIF_BOLD: &[u8] = include_bytes!("../../../assets/fonts/LiberationSerif-Bold.ttf");
pub const SERIF_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/LiberationSerif-Italic.ttf");
pub const SERIF_BOLD_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/LiberationSerif-BoldItalic.ttf");
pub const MONO: &[u8] = include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf");
pub const EMOJI: &[u8] = include_bytes!("../../../assets/fonts/NotoEmoji-Regular.ttf");
pub const SYMBOLS: &[u8] = include_bytes!("../../../assets/fonts/NotoSansSymbols2-Regular.ttf");
// Only the arrow blocks (⇅ ⟳ …), subset with pyftsubset.
pub const ARROWS: &[u8] = include_bytes!("../../../assets/fonts/NotoSansMath-Arrows.ttf");

/// Every font by name, in the order the fallbacks are tried.
pub const FONTS: &[(&str, &[u8])] = &[
    ("Inter", INTER),
    ("Inter-Bold", INTER_BOLD),
    ("Inter-Italic", INTER_ITALIC),
    ("Inter-BoldItalic", INTER_BOLD_ITALIC),
    ("Inter-SemiBold", INTER_SEMIBOLD),
    ("Serif", SERIF),
    ("Serif-Bold", SERIF_BOLD),
    ("Serif-Italic", SERIF_ITALIC),
    ("Serif-BoldItalic", SERIF_BOLD_ITALIC),
    ("JetBrainsMono", MONO),
    ("NotoEmoji", EMOJI),
    ("NotoSansSymbols2", SYMBOLS),
    ("NotoSansMathArrows", ARROWS),
];

/// Tried after the main font for icons, symbols and code glyphs.
const FALLBACKS: &[&str] = &["NotoEmoji", "NotoSansSymbols2", "NotoSansMathArrows", "JetBrainsMono"];

/// The document text families (beside egui's Proportional and Monospace).
pub const FAMILIES: &[(&str, &str)] = &[
    ("bold", "Inter-Bold"),
    ("italic", "Inter-Italic"),
    ("bolditalic", "Inter-BoldItalic"),
    ("semibold", "Inter-SemiBold"),
    ("serif", "Serif"),
    ("serif-bold", "Serif-Bold"),
    ("serif-italic", "Serif-Italic"),
    ("serif-bolditalic", "Serif-BoldItalic"),
];

/// The fonts of a family, main font first: what egui tries for each glyph.
pub fn chain(family: &FontFamily) -> Vec<&'static str> {
    let first = match family {
        FontFamily::Proportional => "Inter",
        FontFamily::Monospace => "JetBrainsMono",
        FontFamily::Name(n) => FAMILIES.iter().find(|(f, _)| *f == n.as_ref()).map_or("Inter", |(_, font)| *font),
    };
    let mut v = vec![first];
    if matches!(family, FontFamily::Monospace) {
        v.push("Inter");
    }
    v.extend(FALLBACKS.iter().filter(|f| **f != first));
    v
}

pub fn bytes(name: &str) -> &'static [u8] {
    FONTS.iter().find(|(n, _)| *n == name).map_or(INTER, |(_, b)| *b)
}

pub fn definitions() -> FontDefinitions {
    let mut defs = FontDefinitions::empty();
    for (name, bytes) in FONTS {
        defs.font_data.insert((*name).to_owned(), Arc::new(FontData::from_static(bytes)));
    }
    let to_vec = |v: Vec<&str>| v.into_iter().map(str::to_owned).collect::<Vec<_>>();
    defs.families.insert(FontFamily::Proportional, to_vec(chain(&FontFamily::Proportional)));
    defs.families.insert(FontFamily::Monospace, to_vec(chain(&FontFamily::Monospace)));
    for (family, _) in FAMILIES {
        let f = FontFamily::Name((*family).into());
        defs.families.insert(f.clone(), to_vec(chain(&f)));
    }
    defs
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_bundled_font_has_its_licence() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts");
        for lic in ["inter", "liberation", "jetbrainsmono", "notoemoji", "notosanssymbols2", "notosansmath"] {
            let text = std::fs::read_to_string(dir.join(format!("OFL-{lic}.txt"))).unwrap();
            assert!(text.contains("SIL Open Font License"), "{lic}");
        }
    }

    #[test]
    fn every_font_parses() {
        for (name, bytes) in super::FONTS {
            assert!(ttf_parser::Face::parse(bytes, 0).is_ok(), "{name}");
        }
    }
}
