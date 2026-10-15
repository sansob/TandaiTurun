//! UI translations. English is the source language; `tr` returns the
//! translation for the current language, or the English text when there is
//! none (so a missing entry never breaks the UI).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    En,
    Id,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Id];

    /// The language's own name, for the picker.
    pub fn name(&self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Id => "Bahasa Indonesia",
        }
    }

    pub fn from_code(code: &str) -> Lang {
        match code.trim().to_ascii_lowercase().split(['-', '_', '.']).next() {
            Some("id") | Some("in") => Lang::Id,
            _ => Lang::En,
        }
    }

    /// From `LC_ALL` / `LC_MESSAGES` / `LANG` when no setting is saved.
    pub fn from_system() -> Lang {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
            if let Ok(v) = std::env::var(var) {
                if !v.trim().is_empty() && v != "C" && v != "POSIX" {
                    return Lang::from_code(&v);
                }
            }
        }
        Lang::En
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn current() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == Lang::Id as u8 {
        Lang::Id
    } else {
        Lang::En
    }
}

fn indonesian() -> &'static HashMap<&'static str, &'static str> {
    static TABLE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    TABLE.get_or_init(|| crate::i18n_id::ENTRIES.iter().copied().collect())
}

/// The text in the current language.
pub fn tr(english: &str) -> &str {
    match current() {
        Lang::En => english,
        Lang::Id => indonesian().get(english).copied().unwrap_or(english),
    }
}

/// `tr` for templates with `{name}` placeholders: `trf("Saved {file}", &[("file", path)])`.
pub fn trf(english: &str, args: &[(&str, &str)]) -> String {
    let mut out = tr(english).to_string();
    for (k, v) in args {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_and_fallback() {
        assert_eq!(Lang::from_code("id_ID.UTF-8"), Lang::Id);
        assert_eq!(Lang::from_code("en_US"), Lang::En);
        set(Lang::Id);
        assert_eq!(tr("Open"), "Buka");
        assert_eq!(tr("no such text"), "no such text");
        assert_eq!(trf("Hi {who}", &[("who", "Budi")]), "Hi Budi");
        set(Lang::En);
        assert_eq!(tr("Open"), "Open");
    }

    #[test]
    fn placeholders_survive_translation() {
        for (en, id) in crate::i18n_id::ENTRIES {
            let holes = |s: &str| {
                let mut v: Vec<String> = s
                    .match_indices('{')
                    .filter_map(|(i, _)| s[i..].find('}').map(|j| s[i..i + j + 1].to_string()))
                    .collect();
                v.sort();
                v
            };
            assert_eq!(holes(en), holes(id), "{en:?} → {id:?}");
        }
    }

    #[test]
    fn no_duplicate_keys() {
        let mut seen = std::collections::HashSet::new();
        for (en, _) in crate::i18n_id::ENTRIES {
            assert!(seen.insert(en), "duplicate entry {en:?}");
        }
    }

    /// Every literal passed to `tr(` / `trf(` has a translation or is listed
    /// as the same in both languages.
    #[test]
    fn every_ui_text_is_translated() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let keys: std::collections::HashSet<&str> = crate::i18n_id::ENTRIES.iter().map(|(en, _)| *en).collect();
        let mut missing = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.ends_with("i18n.rs") || path.ends_with("i18n_id.rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            for marker in ["tr(", "trf("] {
                for (at, _) in src.match_indices(marker) {
                    let prev = src[..at].chars().next_back().unwrap_or(' ');
                    if prev.is_alphanumeric() || prev == '_' {
                        continue;
                    }
                    let Some(rest) = src[at + marker.len()..].trim_start().strip_prefix('"') else { continue };
                    let Some(end) = rest.find('"') else { continue };
                    let text = &rest[..end];
                    if !keys.contains(text) && !crate::i18n_id::SAME.contains(&text) {
                        missing.push(format!("{}: {text}", path.display()));
                    }
                }
            }
        }
        missing.sort();
        missing.dedup();
        assert!(missing.is_empty(), "untranslated UI text:\n{}", missing.join("\n"));
    }
}
