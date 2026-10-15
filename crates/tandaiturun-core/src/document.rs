//! Loading and saving Markdown files without surprising anyone: the line
//! endings and the byte-order mark of the original file are kept, and saving
//! goes through a temporary file so a crash never leaves half a document.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Line endings of a file, kept so saving doesn't rewrite every line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEnding {
    #[default]
    Lf,
    CrLf,
}

/// The file extensions offered in the open and save dialogs.
pub const EXTENSIONS: &[&str] = &["md", "markdown", "mdown", "mkd", "mkdn", "mdx", "txt"];

/// A Markdown file as read from disk. The text always uses `\n`.
#[derive(Debug, Clone, Default)]
pub struct LoadedFile {
    pub text: String,
    pub line_ending: LineEnding,
    pub bom: bool,
}

/// Decodes file contents. Invalid UTF-8 is replaced rather than refused, so a
/// stray Latin-1 byte doesn't keep a document from opening.
pub fn decode(bytes: &[u8]) -> LoadedFile {
    let (bom, bytes) = match bytes.strip_prefix(b"\xEF\xBB\xBF") {
        Some(rest) => (true, rest),
        None => (false, bytes),
    };
    let raw = String::from_utf8_lossy(bytes);
    let line_ending = if raw.contains("\r\n") { LineEnding::CrLf } else { LineEnding::Lf };
    let text = if raw.contains('\r') { raw.replace("\r\n", "\n").replace('\r', "\n") } else { raw.into_owned() };
    LoadedFile { text, line_ending, bom }
}

/// Encodes text for writing with the given line ending and BOM.
pub fn encode(text: &str, line_ending: LineEnding, bom: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() + 3);
    if bom {
        out.extend_from_slice(b"\xEF\xBB\xBF");
    }
    match line_ending {
        LineEnding::Lf => out.extend_from_slice(text.as_bytes()),
        LineEnding::CrLf => out.extend_from_slice(text.replace('\n', "\r\n").as_bytes()),
    }
    out
}

pub fn load(path: &Path) -> io::Result<LoadedFile> {
    Ok(decode(&fs::read(path)?))
}

/// Writes `bytes` to `path` atomically: a temporary file next to it, then a rename.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "document".into());
    let tmp = dir.join(format!(".{name}.tandaiturun.tmp"));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

pub fn save(path: &Path, text: &str, line_ending: LineEnding, bom: bool) -> io::Result<()> {
    write_atomic(path, &encode(text, line_ending, bom))
}

/// Drops the Windows "verbatim" prefix `canonicalize` adds: `\\?\D:\x` → `D:\x`,
/// `\\?\UNC\server\share` → `\\server\share`. Other paths are returned as they are.
pub fn simplify(path: PathBuf) -> PathBuf {
    let Some(s) = path.to_str() else { return path };
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        // Only drive paths: `\\?\Volume{…}` has no simpler form.
        let b = rest.as_bytes();
        if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
            return PathBuf::from(rest);
        }
    }
    path
}

/// `fs::canonicalize` without the Windows verbatim prefix; the path itself if it fails.
pub fn canonical(path: &Path) -> PathBuf {
    simplify(fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()))
}

/// Gives `path` a Markdown extension when it has none ("notes" → "notes.md").
pub fn with_md_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_some() {
        path
    } else {
        path.with_extension("md")
    }
}

/// Whether a path looks like a document TandaiTurun can open.
pub fn is_markdown_path(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// The document title: the YAML `title:` if there is one, then the first
/// heading, then the file name.
pub fn title(text: &str, path: Option<&Path>) -> String {
    if let Some(t) = front_matter(text).and_then(|fm| {
        fm.lines().find_map(|l| l.strip_prefix("title:").map(|v| v.trim().trim_matches(['"', '\'']).to_string()))
    }) {
        if !t.is_empty() {
            return t;
        }
    }
    if let Some(h) = crate::outline::outline(text).into_iter().next() {
        if !h.text.is_empty() {
            return h.text;
        }
    }
    path.and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Untitled".into())
}

/// The YAML front matter between the leading `---` lines, without the fences.
pub fn front_matter(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("---\n")?;
    let end = rest.find("\n---\n").map(|i| (i, i + 5)).or_else(|| {
        rest.strip_suffix("\n---").map(|r| (r.len(), rest.len())).or_else(|| rest.find("\n...\n").map(|i| (i, i + 5)))
    })?;
    Some(&rest[..end.0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_endings_and_bom_round_trip() {
        let bytes = b"\xEF\xBB\xBF# Hi\r\n\r\ntext\r\n";
        let f = decode(bytes);
        assert_eq!(f.text, "# Hi\n\ntext\n");
        assert_eq!(f.line_ending, LineEnding::CrLf);
        assert!(f.bom);
        assert_eq!(encode(&f.text, f.line_ending, f.bom), bytes);
        let plain = decode(b"a\nb");
        assert_eq!((plain.line_ending, plain.bom), (LineEnding::Lf, false));
    }

    #[test]
    fn invalid_utf8_still_opens() {
        assert_eq!(decode(b"caf\xe9").text, "caf\u{FFFD}");
    }

    #[test]
    fn atomic_save_writes_and_cleans_up() {
        let dir = std::env::temp_dir().join(format!("tandaiturun-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("doc.md");
        save(&p, "a\nb\n", LineEnding::CrLf, false).unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"a\r\nb\r\n");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "no temp file left behind");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn titles() {
        assert_eq!(title("---\ntitle: \"Laporan\"\n---\n# H", None), "Laporan");
        assert_eq!(title("intro\n\n## Second *level*\n", None), "Second level");
        assert_eq!(title("no headings", Some(Path::new("/x/catatan.md"))), "catatan");
        assert_eq!(title("", None), "Untitled");
    }

    #[test]
    fn front_matter_variants() {
        assert_eq!(front_matter("---\na: 1\n---\nbody"), Some("a: 1"));
        assert_eq!(front_matter("---\na: 1\n---"), Some("a: 1"));
        assert_eq!(front_matter("# no"), None);
    }

    #[test]
    fn verbatim_prefixes_are_dropped() {
        assert_eq!(simplify(PathBuf::from(r"\\?\D:\Dolphin\a.md")), PathBuf::from(r"D:\Dolphin\a.md"));
        assert_eq!(simplify(PathBuf::from(r"\\?\UNC\srv\share\a.md")), PathBuf::from(r"\\srv\share\a.md"));
        assert_eq!(simplify(PathBuf::from(r"\\?\Volume{x}\a.md")), PathBuf::from(r"\\?\Volume{x}\a.md"));
        assert_eq!(simplify(PathBuf::from("/home/a.md")), PathBuf::from("/home/a.md"));
    }

    #[test]
    fn extensions() {
        assert!(is_markdown_path(Path::new("README.MD")));
        assert!(!is_markdown_path(Path::new("x.pdf")));
        assert_eq!(with_md_extension(PathBuf::from("notes")), PathBuf::from("notes.md"));
        assert_eq!(with_md_extension(PathBuf::from("notes.txt")), PathBuf::from("notes.txt"));
    }
}
