//! The fonts bundled into the app, all under the SIL Open Font License 1.1
//! (licence texts in `assets/fonts/`). egui's default fonts are not used, so
//! every component of a release is under an OSI-approved licence.
//!
//! The same font files are embedded into exported PDFs, so a PDF looks
//! exactly like the page on screen.

use egui::{FontData, FontDefinitions, FontFamily};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const INTER: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
pub const INTER_BOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-Bold.ttf");
pub const INTER_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/Inter-Italic.ttf");
pub const INTER_BOLD_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/Inter-BoldItalic.ttf");
pub const INTER_SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf");
pub const SERIF: &[u8] = include_bytes!("../../../assets/fonts/LiberationSerif-Regular.ttf");
pub const SERIF_BOLD: &[u8] = include_bytes!("../../../assets/fonts/LiberationSerif-Bold.ttf");
pub const SERIF_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/LiberationSerif-Italic.ttf");
pub const SERIF_BOLD_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/LiberationSerif-BoldItalic.ttf");
pub const MONO: &[u8] = include_bytes!("../../../assets/fonts/JetBrainsMonoNL-Regular.ttf");
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
    v.extend(system().iter().map(|f| f.name));
    v
}

pub fn bytes(name: &str) -> &'static [u8] {
    if let Some((_, b)) = FONTS.iter().find(|(n, _)| *n == name) {
        return b;
    }
    system().iter().find(|f| f.name == name).map_or(INTER, |f| f.data)
}

/// The face in the file (font collections hold several).
pub fn index(name: &str) -> u32 {
    system().iter().find(|f| f.name == name).map_or(0, |f| f.index)
}

/// A font from the operating system, for scripts the bundled fonts don't
/// cover (Chinese, Japanese, Korean, Arabic, Hebrew, Indic, Thai…). Loaded
/// only once a document needs it, and kept for the rest of the run.
#[derive(Clone, Copy)]
pub struct SystemFont {
    pub name: &'static str,
    pub data: &'static [u8],
    pub index: u32,
}

struct System {
    loaded: Vec<SystemFont>,
    /// Files already read that didn't help, or couldn't be read.
    tried: BTreeSet<PathBuf>,
    /// Characters nothing could draw: not looked up again.
    hopeless: BTreeSet<char>,
}

static SYSTEM: Mutex<System> =
    Mutex::new(System { loaded: Vec::new(), tried: BTreeSet::new(), hopeless: BTreeSet::new() });

fn system() -> Vec<SystemFont> {
    SYSTEM.lock().map(|s| s.loaded.clone()).unwrap_or_default()
}

/// Font files worth trying, best first, by file name.
const CANDIDATES: &[&str] = &[
    // Windows
    "msyh.ttc",
    "msyh.ttf",
    "YuGothR.ttc",
    "meiryo.ttc",
    "malgun.ttf",
    "simsun.ttc",
    "segoeui.ttf",
    "Nirmala.ttc",
    "Nirmala.ttf",
    "LeelawUI.ttf",
    "ebrima.ttf",
    "seguisym.ttf",
    "tahoma.ttf",
    // macOS
    "PingFang.ttc",
    "Hiragino Sans GB.ttc",
    "AppleSDGothicNeo.ttc",
    "GeezaPro.ttc",
    "SFHebrew.ttf",
    "Kohinoor.ttc",
    "KohinoorBangla.ttc",
    "KohinoorTelugu.ttc",
    "Thonburi.ttc",
    "Arial Unicode.ttf",
    // Linux
    "NotoSansCJK-Regular.ttc",
    "NotoSansCJKsc-Regular.otf",
    "NotoSansCJKjp-Regular.otf",
    "SourceHanSans-Regular.ttc",
    "wqy-microhei.ttc",
    "wqy-zenhei.ttc",
    "DroidSansFallbackFull.ttf",
    "DroidSansFallback.ttf",
    "fonts-japanese-gothic.ttf",
    "NotoSansArabic-Regular.ttf",
    "NotoNaskhArabic-Regular.ttf",
    "NotoSansHebrew-Regular.ttf",
    "NotoSansDevanagari-Regular.ttf",
    "NotoSansBengali-Regular.ttf",
    "NotoSansTamil-Regular.ttf",
    "NotoSansTelugu-Regular.ttf",
    "NotoSansThai-Regular.ttf",
    "NotoSansArmenian-Regular.ttf",
    "NotoSansGeorgian-Regular.ttf",
    "NotoSansEthiopic-Regular.ttf",
    "Garuda.ttf",
    "Loma.otf",
    "DejaVuSans.ttf",
    "FreeSans.ttf",
    "FreeSerif.ttf",
];

fn font_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if cfg!(windows) {
        let win = std::env::var_os("WINDIR").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        v.push(win.join("Fonts"));
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            v.push(PathBuf::from(local).join(r"Microsoft\Windows\Fonts"));
        }
    } else if cfg!(target_os = "macos") {
        v.extend(["/System/Library/Fonts", "/System/Library/Fonts/Supplemental", "/Library/Fonts"].map(PathBuf::from));
    } else {
        v.extend(["/usr/share/fonts", "/usr/local/share/fonts"].map(PathBuf::from));
        if let Some(data) = std::env::var_os("XDG_DATA_HOME") {
            v.push(PathBuf::from(data).join("fonts"));
        }
        if let Some(home) = std::env::var_os("HOME") {
            v.push(PathBuf::from(&home).join(".local/share/fonts"));
            v.push(PathBuf::from(home).join(".fonts"));
        }
    }
    v
}

/// The candidate files present on this system, best first.
fn candidate_files() -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: u32, found: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if depth > 0 {
                    walk(&p, depth - 1, found);
                }
            } else if p.file_name().and_then(|n| n.to_str()).is_some_and(|n| CANDIDATES.contains(&n)) {
                found.push(p);
            }
        }
    }
    let mut found = Vec::new();
    for d in font_dirs() {
        walk(&d, 4, &mut found);
    }
    let rank = |p: &PathBuf| {
        let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        CANDIDATES.iter().position(|c| *c == n).unwrap_or(usize::MAX)
    };
    found.sort_by_key(rank);
    found.dedup();
    found
}

fn covers(data: &[u8], index: u32, ch: char) -> bool {
    ttf_parser::Face::parse(data, index).is_ok_and(|f| f.glyph_index(ch).is_some())
}

/// Makes sure some font can draw every character of `text`, loading system
/// fonts for the ones the bundled fonts lack. True when a font was added:
/// the caller then installs the new [`definitions`].
pub fn cover(text: &str) -> bool {
    let Ok(mut sys) = SYSTEM.lock() else { return false };
    // Cheap first pass: only letters outside the scripts we bundle can need help.
    let mut missing: BTreeSet<char> = text
        .chars()
        .filter(|c| *c as u32 >= 0x0370 && !c.is_whitespace() && !c.is_control())
        .filter(|c| !sys.hopeless.contains(c))
        .collect();
    if missing.is_empty() {
        return false;
    }
    let have: Vec<ttf_parser::Face> = FONTS
        .iter()
        .filter_map(|(_, b)| ttf_parser::Face::parse(b, 0).ok())
        .chain(sys.loaded.iter().filter_map(|f| ttf_parser::Face::parse(f.data, f.index).ok()))
        .collect();
    missing.retain(|c| !have.iter().any(|f| f.glyph_index(*c).is_some()));
    drop(have);
    if missing.is_empty() {
        return false;
    }
    let mut added = false;
    for path in candidate_files() {
        if missing.is_empty() {
            break;
        }
        if !sys.tried.insert(path.clone()) {
            continue;
        }
        let Ok(data) = std::fs::read(&path) else { continue };
        let faces = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
        let Some(index) = (0..faces).find(|i| missing.iter().any(|c| covers(&data, *i, *c))) else { continue };
        let data: &'static [u8] = Box::leak(data.into_boxed_slice());
        missing.retain(|c| !covers(data, index, *c));
        let name: &'static str = Box::leak(format!("system:{}", path.display()).into_boxed_str());
        sys.loaded.push(SystemFont { name, data, index });
        added = true;
    }
    sys.hopeless.extend(missing);
    added
}

pub fn definitions() -> FontDefinitions {
    let mut defs = FontDefinitions::empty();
    for (name, bytes) in FONTS {
        defs.font_data.insert((*name).to_owned(), Arc::new(FontData::from_static(bytes)));
    }
    for f in system() {
        let mut data = FontData::from_static(f.data);
        data.index = f.index;
        defs.font_data.insert(f.name.to_owned(), Arc::new(data));
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
        for lic in
            ["inter", "liberation", "stixtwomath", "jetbrainsmono", "notoemoji", "notosanssymbols2", "notosansmath"]
        {
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
