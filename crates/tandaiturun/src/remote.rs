//! Downloading internet images, only after the reader allowed it.
//!
//! The system's certificate store and proxy settings are used (through
//! `rustls-platform-verifier` and the usual `HTTPS_PROXY` variables), so it
//! works behind company proxies too. Each image is fetched once, in the
//! background, and handed to egui's image loaders.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// Largest image we download.
const MAX_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Clone)]
pub enum State {
    Loading,
    /// The URI the bytes are registered under with egui (SVGs get a `.svg`
    /// suffix so the SVG loader picks them up), and the bytes.
    Ready {
        uri: String,
        bytes: Arc<[u8]>,
    },
    Failed(String),
}

fn states() -> &'static Mutex<HashMap<String, State>> {
    static S: OnceLock<Mutex<HashMap<String, State>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

fn agent() -> &'static ureq::Agent {
    static A: OnceLock<ureq::Agent> = OnceLock::new();
    A.get_or_init(|| {
        let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
        ureq::Agent::config_builder()
            .tls_config(tls)
            .timeout_global(Some(Duration::from_secs(30)))
            .user_agent(concat!("TandaiTurun/", env!("CARGO_PKG_VERSION")))
            .build()
            .into()
    })
}

fn download(url: &str) -> Result<(Vec<u8>, String), String> {
    let mut resp = agent().get(url).call().map_err(|e| e.to_string())?;
    let mime = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_ascii_lowercase();
    let bytes = resp.body_mut().with_config().limit(MAX_BYTES).read_to_vec().map_err(|e| e.to_string())?;
    Ok((bytes, mime))
}

fn is_svg(mime: &str, bytes: &[u8]) -> bool {
    mime.contains("svg")
        || bytes.trim_ascii_start().starts_with(b"<svg")
        || bytes.trim_ascii_start().starts_with(b"<?xml")
}

/// The state of an internet image; starts the download the first time.
pub fn image(ctx: &egui::Context, url: &str) -> State {
    let mut map = match states().lock() {
        Ok(m) => m,
        Err(_) => return State::Failed("internal error".into()),
    };
    if let Some(s) = map.get(url) {
        if let State::Ready { uri, bytes } = s {
            // Register with this context (the PDF export uses its own).
            if ctx.try_load_bytes(uri).is_err() {
                ctx.include_bytes(uri.clone(), bytes.clone());
            }
        }
        return s.clone();
    }
    map.insert(url.to_string(), State::Loading);
    drop(map);
    let (ctx, url) = (ctx.clone(), url.to_string());
    std::thread::spawn(move || {
        let state = match download(&url) {
            Ok((bytes, mime)) => {
                let uri = if is_svg(&mime, &bytes) { format!("{url}#tandaiturun.svg") } else { url.clone() };
                let bytes: Arc<[u8]> = bytes.into();
                ctx.include_bytes(uri.clone(), bytes.clone());
                State::Ready { uri, bytes }
            }
            Err(e) => State::Failed(e),
        };
        if let Ok(mut m) = states().lock() {
            m.insert(url, state);
        }
        ctx.request_repaint();
    });
    State::Loading
}

/// Downloaded bytes, for the PDF export.
pub fn bytes(uri: &str) -> Option<Arc<[u8]>> {
    let m = states().lock().ok()?;
    m.values().find_map(|s| match s {
        State::Ready { uri: u, bytes } if u == uri => Some(bytes.clone()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn svg_detection() {
        assert!(super::is_svg("image/svg+xml;charset=utf-8", b""));
        assert!(super::is_svg("", b"  <svg xmlns=..."));
        assert!(!super::is_svg("image/png", b"\x89PNG"));
    }
}
