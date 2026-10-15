//! A tiny test driver: `TANDAITURUN_SCRIPT=steps.txt tandaiturun [files]`
//! plays a list of input steps (clicks, keys, typing) as if a person did
//! them, and can save screenshots and the app's state along the way. Used to
//! test the app end to end; does nothing otherwise.
//!
//! One step per line (`#` starts a comment):
//!   wait N            let N frames pass
//!   move X Y          move the pointer
//!   click X Y         left click (also `rclick`, `mclick`, `dclick`)
//!   key SPEC          a key with modifiers: `ctrl+shift+s`, `enter`, `f3`
//!   type TEXT         type text
//!   paste TEXT        paste text
//!   copy              the platform "copy" command (Ctrl+C)
//!   open PATH         open a file
//!   append PATH TEXT  append a line to a file (another program changing it)
//!   shot PATH         save a screenshot
//!   dump PATH         write the app state as text
//!   quit              close the app (unsaved changes are discarded)

use crate::app::{App, Dialog};
use std::sync::Mutex;

struct Script {
    steps: Vec<String>,
    next: usize,
    wait: u32,
    /// Events for the following frames (a press now, its release next frame).
    later: Vec<Vec<egui::Event>>,
    shot: Option<String>,
}

static SCRIPT: Mutex<Option<Script>> = Mutex::new(None);

pub fn init_from_env() {
    let Ok(path) = std::env::var("TANDAITURUN_SCRIPT") else { return };
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let steps =
        text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(String::from).collect();
    *SCRIPT.lock().unwrap() = Some(Script { steps, next: 0, wait: 20, later: Vec::new(), shot: None });
}

pub fn active() -> bool {
    SCRIPT.lock().map(|s| s.is_some()).unwrap_or(false)
}

fn modifiers(spec: &str) -> (egui::Modifiers, &str) {
    let mut m = egui::Modifiers::NONE;
    let mut parts: Vec<&str> = spec.split('+').collect();
    let key = parts.pop().unwrap_or("");
    for p in parts {
        match p.to_ascii_lowercase().as_str() {
            "ctrl" | "cmd" => {
                m.ctrl = true;
                m.command = true;
            }
            "shift" => m.shift = true,
            "alt" => m.alt = true,
            other => panic!("unknown modifier {other}"),
        }
    }
    (m, key)
}

/// A key by name, any case: `pagedown`, `F3`, `plus`, `a`.
fn key(k: &str) -> egui::Key {
    egui::Key::ALL
        .iter()
        .copied()
        .find(|key| key.name().eq_ignore_ascii_case(k) || key.symbol_or_name().eq_ignore_ascii_case(k))
        .unwrap_or_else(|| panic!("unknown key {k}"))
}

fn button(x: f32, y: f32, button: egui::PointerButton, pressed: bool) -> egui::Event {
    egui::Event::PointerButton { pos: egui::pos2(x, y), button, pressed, modifiers: Default::default() }
}

fn describe(app: &App) -> String {
    let mut s = String::new();
    let dialog = match &app.dialog {
        None => "none".to_string(),
        Some(Dialog::ConfirmClose { uids, quit }) => format!("confirm-close {} quit={quit}", uids.len()),
        Some(Dialog::Properties) => "properties".into(),
        Some(Dialog::Preferences) => "preferences".into(),
        Some(Dialog::Shortcuts) => "shortcuts".into(),
        Some(Dialog::About { tab, .. }) => format!("about {tab:?}"),
        Some(Dialog::Error(e)) => format!("error: {e}"),
    };
    s += &format!("active: {:?}\ndocs: {}\ndialog: {dialog}\n", app.active, app.docs.len());
    s += &format!(
        "find_open: {} query: {:?} read_mode: {} fullscreen: {}\n",
        app.find_open, app.find_query, app.read_mode, app.fullscreen
    );
    s += &format!(
        "nav: {} {:?} tools: {} theme: {:?} page: {:?} font: {:?} size: {}\n",
        app.settings.show_nav,
        app.settings.nav_tab,
        app.settings.show_tools,
        app.settings.ui_theme,
        app.settings.page_theme,
        app.settings.doc_font,
        app.settings.font_size
    );
    s += &format!("toasts: {:?}\n", app.toasts.iter().map(|t| t.text.clone()).collect::<Vec<_>>());
    for (i, d) in app.docs.iter().enumerate() {
        let v = d.active_view_ref();
        s += &format!(
            "doc {i}: name={} dirty={} editing={} path={:?} len={} pages={} page={} zoom={:?} mode={:?} hits={} current_hit={:?} cursor={:?} marks={} selection={:?}\n",
            d.name(),
            d.dirty(),
            d.editing,
            d.path,
            d.text.len(),
            v.page_count(),
            v.current_page,
            v.zoom,
            v.mode,
            v.search.hits.len(),
            v.search.current,
            d.editor.cursor,
            d.editor.marks.len(),
            v.selected_text(),
        );
        s += &format!("text {i} >>>\n{}\n<<<\n", d.text);
    }
    s
}

pub fn tick(app: &mut App, ctx: &egui::Context) {
    let mut guard = SCRIPT.lock().unwrap();
    let Some(sc) = guard.as_mut() else { return };
    ctx.request_repaint();
    // A screenshot asked for in an earlier frame.
    if let Some(path) = sc.shot.clone() {
        let image = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            let [w, h] = image.size;
            let buf: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
            if let Some(img) = image::RgbaImage::from_raw(w as u32, h as u32, buf) {
                let _ = img.save(&path);
            }
            sc.shot = None;
        } else {
            return;
        }
    }
    if !sc.later.is_empty() {
        let events = sc.later.remove(0);
        app.inject_keys.extend(events);
        return;
    }
    if sc.wait > 0 {
        sc.wait -= 1;
        return;
    }
    let Some(step) = sc.steps.get(sc.next).cloned() else {
        app.allow_close = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        return;
    };
    sc.next += 1;
    sc.wait = 3;
    let (cmd, rest) = step.split_once(' ').unwrap_or((step.as_str(), ""));
    let num = |i: usize| -> f32 { rest.split_whitespace().nth(i).and_then(|v| v.parse().ok()).unwrap_or(0.0) };
    let ev = &mut app.inject_keys;
    match cmd {
        "wait" => sc.wait = num(0) as u32,
        "move" => ev.push(egui::Event::PointerMoved(egui::pos2(num(0), num(1)))),
        "click" | "rclick" | "mclick" | "dclick" => {
            let b = match cmd {
                "rclick" => egui::PointerButton::Secondary,
                "mclick" => egui::PointerButton::Middle,
                _ => egui::PointerButton::Primary,
            };
            let (x, y) = (num(0), num(1));
            ev.extend([egui::Event::PointerMoved(egui::pos2(x, y)), button(x, y, b, true)]);
            sc.later.push(vec![button(x, y, b, false)]);
            if cmd == "dclick" {
                sc.later.push(vec![button(x, y, b, true)]);
                sc.later.push(vec![button(x, y, b, false)]);
            }
        }
        "key" => {
            let (m, k) = modifiers(rest);
            let key = key(k);
            let e = |pressed| egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: m };
            ev.extend([egui::Event::ModifiersChanged(m), e(true)]);
            sc.later.push(vec![e(false), egui::Event::ModifiersChanged(egui::Modifiers::NONE)]);
        }
        "type" => ev.push(egui::Event::Text(rest.to_string())),
        "paste" => ev.push(egui::Event::Paste(rest.to_string())),
        "copy" => ev.push(egui::Event::Copy),
        "open" => app.open_path(std::path::Path::new(rest)),
        "append" => {
            let (path, line) = rest.split_once(' ').unwrap_or((rest, ""));
            let mut f = std::fs::OpenOptions::new().append(true).open(path).expect("append");
            std::io::Write::write_all(&mut f, format!("{line}\n").as_bytes()).expect("append");
        }
        "shot" => {
            sc.shot = Some(rest.to_string());
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        "dump" => {
            let _ = std::fs::write(rest, describe(app));
        }
        "quit" => {
            app.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        other => panic!("unknown step {other}"),
    }
}
