//! "Make TandaiTurun the default app for Markdown files", per platform.
//!
//! - Linux: `xdg-mime default` for `text/markdown` (with a `.desktop` file in
//!   `~/.local/share/applications` when the app isn't installed as a package,
//!   e.g. the AppImage).
//! - macOS: Launch Services, for the Markdown content type.
//! - Windows: apps may not change the default themselves (Windows 10 and
//!   later). We register TandaiTurun for `.md` files for the current user and
//!   open Settings → Default apps, where the user picks it.

/// What happened, for the message shown to the user.
pub enum Outcome {
    Done,
    /// The system settings were opened: the user finishes there.
    #[cfg_attr(not(windows), allow(dead_code))]
    OpenedSettings,
}

pub fn make_default() -> Result<Outcome, String> {
    imp::make_default()
}

#[cfg(target_os = "linux")]
mod imp {
    use super::Outcome;
    use std::path::PathBuf;
    use std::process::Command;

    const MIME: &[&str] = &["text/markdown", "text/x-markdown"];

    fn data_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        let home = std::env::var_os("HOME").map(PathBuf::from);
        match std::env::var_os("XDG_DATA_HOME") {
            Some(d) => dirs.push(PathBuf::from(d)),
            None => dirs.extend(home.map(|h| h.join(".local/share"))),
        }
        let sys = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
        dirs.extend(sys.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
        dirs
    }

    /// The desktop file of an installed package, or one written for this executable.
    fn ensure_desktop_file() -> Result<(), String> {
        if data_dirs().iter().any(|d| d.join("applications/tandaiturun.desktop").is_file()) {
            return Ok(());
        }
        // Not installed as a package (AppImage, tarball, cargo build): point at this binary.
        let exe = std::env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .or_else(|| std::env::current_exe().ok())
            .ok_or("cannot find the program path")?;
        let dir = data_dirs().into_iter().next().ok_or("no data directory")?.join("applications");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let desktop = format!(
            "[Desktop Entry]\nType=Application\nName=TandaiTurun\nComment=Read Markdown like a PDF, edit it like text\nExec=\"{}\" %F\nIcon=tandaiturun\nTerminal=false\nCategories=Office;Viewer;TextEditor;\nMimeType=text/markdown;text/x-markdown;\nStartupWMClass=io.github.sansob.TandaiTurun\n",
            exe.display()
        );
        std::fs::write(dir.join("tandaiturun.desktop"), desktop).map_err(|e| e.to_string())?;
        let _ = Command::new("update-desktop-database").arg(&dir).status();
        Ok(())
    }

    pub fn make_default() -> Result<Outcome, String> {
        ensure_desktop_file()?;
        let mut cmd = Command::new("xdg-mime");
        cmd.arg("default").arg("tandaiturun.desktop").args(MIME);
        match cmd.status() {
            Ok(s) if s.success() => Ok(Outcome::Done),
            Ok(s) => Err(format!("xdg-mime exited with {s}")),
            Err(e) => Err(format!("xdg-mime: {e}")),
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::Outcome;
    use std::ffi::{c_char, c_void, CString};

    type CFStringRef = *const c_void;
    const UTF8: u32 = 0x0800_0100;
    const ROLES_ALL: u32 = 0xFFFF_FFFF;

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFStringCreateWithCString(alloc: *const c_void, s: *const c_char, encoding: u32) -> CFStringRef;
        fn CFRelease(cf: *const c_void);
    }

    #[link(name = "CoreServices", kind = "framework")]
    extern "C" {
        fn LSSetDefaultRoleHandlerForContentType(content_type: CFStringRef, role: u32, bundle_id: CFStringRef) -> i32;
    }

    fn cf(s: &str) -> CFStringRef {
        let c = CString::new(s).expect("no NUL");
        // SAFETY: a valid NUL-terminated UTF-8 string; the result is released by the caller.
        unsafe { CFStringCreateWithCString(std::ptr::null(), c.as_ptr(), UTF8) }
    }

    pub fn make_default() -> Result<Outcome, String> {
        let bundle = cf("io.github.sansob.TandaiTurun");
        let mut status = 0;
        for uti in ["net.daringfireball.markdown", "public.markdown"] {
            let t = cf(uti);
            // SAFETY: both arguments are valid CFStrings, released right after.
            let s = unsafe { LSSetDefaultRoleHandlerForContentType(t, ROLES_ALL, bundle) };
            unsafe { CFRelease(t) };
            if uti == "net.daringfireball.markdown" {
                status = s;
            }
        }
        unsafe { CFRelease(bundle) };
        if status == 0 {
            Ok(Outcome::Done)
        } else {
            Err(format!("Launch Services error {status} (is TandaiTurun in Applications?)"))
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::Outcome;
    use std::process::Command;

    fn reg(args: &[&str]) -> Result<(), String> {
        let status = Command::new("reg").args(args).status().map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("reg {} failed", args.join(" ")))
        }
    }

    pub fn make_default() -> Result<Outcome, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe = exe.display().to_string();
        let class = r"HKCU\Software\Classes\TandaiTurun.Markdown";
        reg(&["add", class, "/ve", "/d", "Markdown document", "/f"])?;
        reg(&["add", &format!(r"{class}\DefaultIcon"), "/ve", "/d", &format!("{exe},0"), "/f"])?;
        reg(&["add", &format!(r"{class}\shell\open\command"), "/ve", "/d", &format!("\"{exe}\" \"%1\""), "/f"])?;
        for ext in [".md", ".markdown"] {
            reg(&[
                "add",
                &format!(r"HKCU\Software\Classes\{ext}\OpenWithProgids"),
                "/v",
                "TandaiTurun.Markdown",
                "/t",
                "REG_NONE",
                "/f",
            ])?;
        }
        // Windows lets only the user pick the default: open the page for it.
        open::that_detached("ms-settings:defaultapps").map_err(|e| e.to_string())?;
        Ok(Outcome::OpenedSettings)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod imp {
    pub fn make_default() -> Result<super::Outcome, String> {
        Err("not supported on this system".into())
    }
}
