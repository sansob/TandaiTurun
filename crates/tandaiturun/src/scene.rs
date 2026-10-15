//! Scripted scenes, used for the screenshots in the README and for the
//! start-up smoke test of the packages.
//!
//! `TANDAITURUN_SCENE=<name> TANDAITURUN_SCREENSHOT=out.png tandaiturun [file.md]`
//! sets up a scene, waits until the pages are laid out, saves a screenshot
//! and exits. Not used otherwise.

use crate::app::{App, NavTab};
use crate::theme::{PageTheme, UiTheme};
use crate::view::{PageMode, ZoomMode};
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct SceneState {
    name: String,
    output: Option<String>,
    started: Instant,
    setup_done: bool,
    frames: u32,
    requested: bool,
}

static SCENE: Mutex<Option<SceneState>> = Mutex::new(None);

pub fn init_from_env() {
    if let Ok(name) = std::env::var("TANDAITURUN_SCENE") {
        *SCENE.lock().unwrap() = Some(SceneState {
            name,
            output: std::env::var("TANDAITURUN_SCREENSHOT").ok(),
            started: Instant::now(),
            setup_done: false,
            frames: 0,
            requested: false,
        });
    }
}

/// Where the ✕ of the first document tab is (scenes only).
pub fn first_close_button(ctx: &egui::Context) -> Option<egui::Pos2> {
    ctx.data(|d| d.get_temp(egui::Id::new("scene_first_x")))
}

pub fn active() -> bool {
    SCENE.lock().map(|s| s.is_some()).unwrap_or(false)
}

fn setup(app: &mut App, name: &str) {
    let s = &mut app.settings;
    s.ui_theme = UiTheme::Light;
    s.page_theme = PageTheme::Paper;
    s.show_nav = true;
    s.show_tools = true;
    s.nav_tab = NavTab::Bookmarks;
    s.lang = Some(crate::i18n::Lang::En);
    if app.docs.is_empty() && name != "home" {
        app.open_text("Welcome", crate::app::WELCOME_EN);
    }
    let first = !app.docs.is_empty();
    match name {
        "home" => app.active = None,
        "edit" => {
            if first {
                app.active = Some(0);
                app.toggle_edit();
            }
        }
        "dark" => {
            app.settings.ui_theme = UiTheme::Dark;
            app.settings.page_theme = PageTheme::Night;
        }
        "thumbs" => {
            app.settings.nav_tab = NavTab::Thumbnails;
            if let Some(d) = app.doc_mut() {
                d.view.set_zoom(ZoomMode::FitPage);
                d.view.set_mode(PageMode::TwoPage);
            }
        }
        "search" => {
            app.settings.nav_tab = NavTab::Search;
            app.find_open = true;
            app.find_query = "markdown".into();
        }
        "bahasa" => {
            app.settings.lang = Some(crate::i18n::Lang::Id);
            app.settings.page_theme = PageTheme::Sepia;
            app.settings.doc_font = crate::layout::DocFont::Serif;
        }
        "properties" => app.dialog = Some(crate::app::Dialog::Properties),
        "preferences" => app.dialog = Some(crate::app::Dialog::Preferences),
        "readmode" => app.read_mode = true,
        "remote" => {
            if let Some(d) = app.doc_mut() {
                d.allow_remote = true;
            }
        }
        "libraries" => {
            app.dialog = Some(crate::app::Dialog::About { tab: crate::app::AboutTab::Libraries, filter: String::new() })
        }
        _ => {}
    }
}

pub fn tick(app: &mut App, ctx: &egui::Context) {
    let mut guard = SCENE.lock().unwrap();
    let Some(state) = guard.as_mut() else { return };
    if !state.setup_done {
        let name = state.name.clone();
        setup(app, &name);
        state.setup_done = true;
    }
    state.frames += 1;
    if state.name == "menuhover" {
        // Click "File", then point at "View": the open menu should follow the pointer.
        let at = |x: f32| egui::pos2(x, 15.0);
        let press = |pressed| egui::Event::PointerButton {
            pos: at(20.0),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        match state.frames {
            5 => app.inject_keys.extend([egui::Event::PointerMoved(at(20.0)), press(true)]),
            6 => app.inject_keys.push(press(false)),
            12 => app.inject_keys.push(egui::Event::PointerMoved(at(60.0))),
            16 => app.inject_keys.push(egui::Event::PointerMoved(at(92.0))),
            _ => {}
        }
    }
    if state.name == "tabs" {
        // Right-click the first document tab: its menu shows the close commands.
        let at = egui::pos2(150.0, 57.0);
        let press = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Default::default(),
        };
        match state.frames {
            5 => app.inject_keys.extend([egui::Event::PointerMoved(at), press(true)]),
            6 => app.inject_keys.push(press(false)),
            _ => {}
        }
    }
    if state.name == "tabclose" && app.docs.len() > 1 {
        // Click the ✕ of the first document tab.
        if let Some(x) = crate::scene::first_close_button(ctx) {
            let press = |pressed| egui::Event::PointerButton {
                pos: x,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            };
            match state.frames {
                8 => app.inject_keys.extend([egui::Event::PointerMoved(x), press(true)]),
                9 => app.inject_keys.push(press(false)),
                _ => {}
            }
        }
    }
    ctx.request_repaint();
    // Give layout, images and fonts a few frames to settle.
    let pending = app.doc().is_some_and(|d| d.active_view_ref().layout.as_ref().is_some_and(|l| l.pending_images));
    let ready = state.frames > 30 && (!pending || state.started.elapsed() > Duration::from_secs(8));
    let Some(output) = state.output.clone() else {
        if ready {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        return;
    };
    if ready && !state.requested {
        state.requested = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        return;
    }
    let shot = ctx.input(|i| {
        i.raw.events.iter().find_map(|e| match e {
            egui::Event::Screenshot { image, .. } => Some(image.clone()),
            _ => None,
        })
    });
    if let Some(image) = shot {
        let [w, h] = image.size;
        let mut buf = Vec::with_capacity(w * h * 4);
        for px in &image.pixels {
            buf.extend_from_slice(&px.to_array());
        }
        if let Some(img) = image::RgbaImage::from_raw(w as u32, h as u32, buf) {
            match img.save(&output) {
                Ok(()) => eprintln!("saved {output}"),
                Err(e) => eprintln!("screenshot failed: {e}"),
            }
        }
        app.allow_close = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    if state.started.elapsed() > Duration::from_secs(60) {
        eprintln!("scene timed out");
        std::process::exit(1);
    }
}
