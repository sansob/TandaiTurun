//! Checking GitHub for a newer release.
//!
//! The latest release is read from the GitHub API in the background; the app
//! polls [`take`] each frame and recommends the update when there is one. It
//! never downloads or installs anything itself: the release page is opened
//! in the browser, where the reader picks the package for their system.

use serde::Deserialize;
use std::sync::Mutex;
use std::time::Duration;

const LATEST: &str = "https://api.github.com/repos/sansob/TandaiTurun/releases/latest";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// Without the leading `v`, e.g. `0.1.3`.
    pub version: String,
    /// The release page.
    pub url: String,
}

pub enum Outcome {
    /// A newer release than this build.
    Newer(Release),
    UpToDate,
    Failed(String),
}

/// The finished check, and whether the reader asked for it.
static RESULT: Mutex<Option<(Outcome, bool)>> = Mutex::new(None);

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

fn fetch() -> Result<Option<Release>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build())
        .timeout_global(Some(Duration::from_secs(15)))
        .user_agent(concat!("TandaiTurun/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let mut resp =
        agent.get(LATEST).header("Accept", "application/vnd.github+json").call().map_err(|e| e.to_string())?;
    let body = resp.body_mut().with_config().limit(1024 * 1024).read_to_string().map_err(|e| e.to_string())?;
    let r: ApiRelease = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    if r.draft || r.prerelease {
        return Ok(None);
    }
    let version = r.tag_name.trim_start_matches('v').to_string();
    Ok(is_newer(&version, env!("CARGO_PKG_VERSION")).then_some(Release { version, url: r.html_url }))
}

/// `major.minor.patch`; anything after `-` or `+` is ignored.
fn parse(v: &str) -> Option<(u64, u64, u64)> {
    let core = v.trim().trim_start_matches('v').split(['-', '+']).next()?;
    let mut it = core.split('.').map(|n| n.parse::<u64>());
    let major = it.next()?.ok()?;
    let minor = it.next().unwrap_or(Ok(0)).ok()?;
    let patch = it.next().unwrap_or(Ok(0)).ok()?;
    Some((major, minor, patch))
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    matches!((parse(candidate), parse(current)), (Some(a), Some(b)) if a > b)
}

/// Starts a check in the background. `manual` is true when the reader asked
/// for it (then "up to date" and errors are reported too).
pub fn start(ctx: &egui::Context, manual: bool) {
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let outcome = match fetch() {
            Ok(Some(r)) => Outcome::Newer(r),
            Ok(None) => Outcome::UpToDate,
            Err(e) => Outcome::Failed(e),
        };
        if let Ok(mut r) = RESULT.lock() {
            *r = Some((outcome, manual));
        }
        ctx.request_repaint();
    });
}

/// The result of the last check, once.
pub fn take() -> Option<(Outcome, bool)> {
    RESULT.lock().ok()?.take()
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.1.3", "0.1.2"));
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(!is_newer("0.1.2", "0.1.2"));
        assert!(!is_newer("0.1.1", "0.1.2"));
        assert!(!is_newer("0.1.2-rc1", "0.1.2"));
        assert!(!is_newer("garbage", "0.1.2"));
    }
}
