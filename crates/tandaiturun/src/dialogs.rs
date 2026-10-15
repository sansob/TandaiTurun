//! Modal dialogs and toast notifications.

use crate::app::{AboutTab, App, Dialog};
use crate::i18n::{self, tr, trf, Lang};
use crate::layout::{DocFont, PageSize};
use crate::theme::{PageTheme, UiTheme, ACCENT, BRAND};
use egui::{Id, Margin, RichText, Vec2};
use std::time::Duration;

fn frame(app: &App) -> egui::Frame {
    egui::Frame::new()
        .fill(app.palette.chrome)
        .stroke(egui::Stroke::new(1.0, app.palette.border))
        .corner_radius(10)
        .inner_margin(Margin::same(20))
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(dialog) = app.dialog.take() else { return };
    let mut keep = true;
    let f = frame(app);
    match &dialog {
        Dialog::ConfirmClose { uids, quit } => {
            let names: Vec<String> = app.docs.iter().filter(|d| uids.contains(&d.uid)).map(|d| d.name()).collect();
            let r = egui::Modal::new(Id::new("confirm_close")).frame(f).show(ctx, |ui| {
                ui.set_max_width(440.0);
                ui.label(RichText::new(tr("Save changes?")).size(18.0).strong());
                ui.add_space(6.0);
                let msg = if names.len() == 1 {
                    trf("“{file}” has unsaved changes. Save them before closing?", &[("file", &names[0])])
                } else {
                    trf("{n} documents have unsaved changes:", &[("n", &names.len().to_string())])
                };
                ui.label(msg);
                if names.len() > 1 {
                    for n in &names {
                        ui.label(format!("  • {n}"));
                    }
                }
                ui.add_space(12.0);
                let mut choice = None;
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(RichText::new(tr("Save")).color(egui::Color32::WHITE)).fill(ACCENT))
                        .clicked()
                    {
                        choice = Some(0);
                    }
                    if ui.button(tr("Don't Save")).clicked() {
                        choice = Some(1);
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        choice = Some(2);
                    }
                });
                choice
            });
            let choice = r.inner.or(if r.should_close() { Some(2) } else { None });
            match choice {
                Some(0) => {
                    let mut all_saved = true;
                    for uid in uids.clone() {
                        if let Some(i) = app.docs.iter().position(|d| d.uid == uid) {
                            all_saved &= app.save(i, false);
                        }
                    }
                    if all_saved {
                        app.close_uids(uids);
                        if *quit {
                            app.allow_close = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                    keep = false;
                }
                Some(1) => {
                    app.close_uids(uids);
                    if *quit {
                        app.allow_close = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    keep = false;
                }
                Some(_) => keep = false,
                None => {}
            }
        }
        Dialog::Properties => {
            let r = egui::Modal::new(Id::new("properties")).frame(f).show(ctx, |ui| properties(app, ui));
            keep = !(r.inner || r.should_close());
        }
        Dialog::Preferences => {
            let r = egui::Modal::new(Id::new("preferences")).frame(f).show(ctx, |ui| preferences(app, ui));
            keep = !(r.inner || r.should_close());
        }
        Dialog::Shortcuts => {
            let r = egui::Modal::new(Id::new("shortcuts")).frame(f).show(ctx, shortcuts);
            keep = !(r.inner || r.should_close());
        }
        Dialog::About { tab, filter } => {
            let (mut tab, mut filter) = (*tab, filter.clone());
            let r = egui::Modal::new(Id::new("about")).frame(f).show(ctx, |ui| about(app, ui, &mut tab, &mut filter));
            if !(r.inner || r.should_close()) && app.dialog.is_none() {
                app.dialog = Some(Dialog::About { tab, filter });
            }
            return;
        }
        Dialog::Error(msg) => {
            let r = egui::Modal::new(Id::new("error")).frame(f).show(ctx, |ui| {
                ui.set_max_width(460.0);
                ui.label(RichText::new(tr("Something went wrong")).size(17.0).strong().color(BRAND));
                ui.add_space(6.0);
                ui.label(msg);
                ui.add_space(10.0);
                ui.button(tr("OK")).clicked()
            });
            keep = !(r.inner || r.should_close());
        }
    }
    if keep && app.dialog.is_none() {
        app.dialog = Some(dialog);
    }
}

fn row(ui: &mut egui::Ui, k: &str, v: impl Into<String>) {
    ui.label(RichText::new(k).color(ui.visuals().weak_text_color()));
    ui.add(egui::Label::new(v.into()).wrap());
    ui.end_row();
}

fn properties(app: &mut App, ui: &mut egui::Ui) -> bool {
    ui.set_width(520.0);
    ui.label(RichText::new(tr("Document Properties")).size(18.0).strong());
    ui.add_space(8.0);
    let Some(d) = app.doc() else { return ui.button(tr("Close")).clicked() };
    let s = tandaiturun_core::stats::stats(&d.text);
    let title = tandaiturun_core::document::title(&d.text, d.path.as_deref());
    let pages = d.active_view_ref().page_count();
    egui::Grid::new("props").num_columns(2).spacing(Vec2::new(16.0, 6.0)).show(ui, |ui| {
        row(ui, tr("Title"), title);
        row(ui, tr("File"), d.name());
        if let Some(p) = &d.path {
            row(ui, tr("Location"), p.parent().map(|x| x.display().to_string()).unwrap_or_default());
            if let Ok(m) = std::fs::metadata(p) {
                row(ui, tr("Size"), format_bytes(m.len()));
            }
        }
        row(ui, tr("Pages"), format!("{pages} ({})", app.settings.page_size.label()));
        row(ui, tr("Words"), s.words.to_string());
        row(ui, tr("Characters"), format!("{} ({} {})", s.characters, s.characters_no_spaces, tr("without spaces")));
        row(ui, tr("Lines"), s.lines.to_string());
        row(ui, tr("Reading time"), trf("about {n} min", &[("n", &s.reading_minutes().to_string())]));
        row(ui, tr("Headings"), s.headings.to_string());
        row(ui, tr("Links"), s.links.to_string());
        row(ui, tr("Images"), s.images.to_string());
        row(ui, tr("Code blocks"), s.code_blocks.to_string());
        row(ui, tr("Tables"), s.tables.to_string());
        let le = match d.line_ending {
            tandaiturun_core::document::LineEnding::Lf => "LF (Unix)",
            tandaiturun_core::document::LineEnding::CrLf => "CRLF (Windows)",
        };
        row(ui, tr("Encoding"), format!("UTF-8{} · {le}", if d.bom { " + BOM" } else { "" }));
    });
    if let Some(fm) = tandaiturun_core::document::front_matter(&d.text) {
        ui.add_space(8.0);
        ui.label(RichText::new(tr("Front matter")).strong());
        egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
            ui.add(egui::Label::new(RichText::new(fm).monospace().size(12.5)).wrap());
        });
    }
    ui.add_space(12.0);
    ui.button(tr("Close")).clicked()
}

fn format_bytes(n: u64) -> String {
    if n < 1024 {
        format!("{n} B")
    } else if n < 1024 * 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{:.1} MB", n as f64 / 1024.0 / 1024.0)
    }
}

fn preferences(app: &mut App, ui: &mut egui::Ui) -> bool {
    ui.set_width(460.0);
    ui.label(RichText::new(tr("Preferences")).size(18.0).strong());
    ui.add_space(8.0);
    let mut make_default = false;
    let s = &mut app.settings;
    egui::Grid::new("prefs").num_columns(2).spacing(Vec2::new(16.0, 10.0)).show(ui, |ui| {
        ui.label(tr("Language"));
        let cur = s.lang.unwrap_or_else(Lang::from_system);
        egui::ComboBox::from_id_salt("lang").width(180.0).selected_text(cur.name()).show_ui(ui, |ui| {
            for l in Lang::ALL {
                if ui.selectable_label(cur == l, l.name()).clicked() {
                    s.lang = Some(l);
                    i18n::set(l);
                }
            }
        });
        ui.end_row();
        ui.label(tr("Theme"));
        ui.horizontal(|ui| {
            for (t, label) in [(UiTheme::System, "System"), (UiTheme::Light, "Light"), (UiTheme::Dark, "Dark")] {
                ui.selectable_value(&mut s.ui_theme, t, tr(label));
            }
        });
        ui.end_row();
        ui.label(tr("Page Color"));
        ui.horizontal(|ui| {
            for t in PageTheme::ALL {
                ui.selectable_value(&mut s.page_theme, t, tr(t.label()));
            }
        });
        ui.end_row();
        ui.label(tr("Page Size"));
        ui.horizontal(|ui| {
            for p in PageSize::ALL {
                ui.selectable_value(&mut s.page_size, p, p.label());
            }
        });
        ui.end_row();
        ui.label(tr("Document Font"));
        ui.horizontal(|ui| {
            ui.selectable_value(&mut s.doc_font, DocFont::Sans, "Sans");
            ui.selectable_value(&mut s.doc_font, DocFont::Serif, "Serif");
        });
        ui.end_row();
        ui.label(tr("Text Size"));
        ui.add(egui::Slider::new(&mut s.font_size, 10.0..=28.0).step_by(1.0).suffix(" pt"));
        ui.end_row();
        ui.label(tr("Editor Text Size"));
        ui.add(egui::Slider::new(&mut s.editor_font_size, 10.0..=24.0).step_by(1.0).suffix(" pt"));
        ui.end_row();
        ui.label(tr("Startup"));
        ui.checkbox(&mut s.restore_session, tr("Reopen the documents from last time"));
        ui.end_row();
        ui.label(tr("Files"));
        ui.checkbox(&mut s.auto_reload, tr("Reload documents changed by other programs"));
        ui.end_row();
        ui.label(tr("Internet images"));
        ui.checkbox(&mut s.remote_images, tr("Show images from the internet without asking"));
        ui.end_row();
        ui.label(tr("Default app"));
        if ui.button(tr("Make TandaiTurun the default app for .md files")).clicked() {
            make_default = true;
        }
        ui.end_row();
    });
    if make_default {
        app.actions.push(crate::app::Action::MakeDefault);
    }
    ui.add_space(12.0);
    ui.button(tr("Close")).clicked()
}

fn shortcuts(ui: &mut egui::Ui) -> bool {
    ui.set_width(520.0);
    ui.label(RichText::new(tr("Keyboard Shortcuts")).size(18.0).strong());
    ui.add_space(8.0);
    let list: &[(&str, &str)] = &[
        ("Ctrl+O", "Open…"),
        ("Ctrl+N", "New"),
        ("Ctrl+S", "Save"),
        ("Ctrl+Shift+S", "Save As…"),
        ("Ctrl+P", "Print…"),
        ("Ctrl+W", "Close Tab"),
        ("Ctrl+Tab", "Next tab"),
        ("Ctrl+E", "Edit Markdown"),
        ("Ctrl+F / F3", "Find…"),
        ("Ctrl+= / Ctrl+-", "Zoom In"),
        ("Ctrl+1 / Ctrl+0 / Ctrl+2", "Actual Size, page, width"),
        ("PgUp / PgDn / Home / End", "Page Navigation"),
        ("Ctrl+H", "Read Mode"),
        ("F4 / Shift+F4", "Navigation Pane"),
        ("F11", "Full Screen"),
        ("Ctrl+D", "Document Properties…"),
        ("Ctrl+B / Ctrl+I / Ctrl+K", "Bold, italic, link"),
        ("Ctrl+Alt+1…6", "Heading"),
        ("Tab / Shift+Tab", "Indent list items"),
    ];
    egui::Grid::new("keys").num_columns(2).striped(true).spacing(Vec2::new(20.0, 6.0)).show(ui, |ui| {
        for (k, what) in list {
            ui.label(RichText::new(*k).monospace());
            ui.label(tr(what));
            ui.end_row();
        }
    });
    ui.add_space(12.0);
    ui.button(tr("Close")).clicked()
}

/// The crates compiled into the app (scripts/gen-credits.py).
const CREDITS: &str = include_str!("../credits.tsv");

pub fn credits() -> impl Iterator<Item = [&'static str; 4]> {
    CREDITS.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()).filter_map(|l| {
        let mut it = l.split('\t');
        Some([it.next()?, it.next()?, it.next()?, it.next()?])
    })
}

const FONTS: &[(&str, &str, &str)] = &[
    ("Inter", "Document and interface text", "https://github.com/rsms/inter"),
    ("Liberation Serif", "Serif document text", "https://github.com/liberationfonts/liberation-fonts"),
    ("JetBrains Mono", "Code", "https://github.com/JetBrains/JetBrainsMono"),
    ("Noto Emoji", "Icons", "https://github.com/googlefonts/noto-emoji"),
    ("Noto Sans Symbols 2", "Symbols", "https://github.com/notofonts/symbols"),
    ("Noto Sans Math (arrows)", "Arrows", "https://github.com/notofonts/math"),
    ("STIX Two Math", "Formulas", "https://github.com/stipub/stixfonts"),
];

fn about(app: &App, ui: &mut egui::Ui, tab: &mut AboutTab, filter: &mut String) -> bool {
    ui.set_width(560.0);
    ui.horizontal(|ui| {
        ui.selectable_value(tab, AboutTab::About, tr("About"));
        ui.selectable_value(tab, AboutTab::Libraries, tr("Libraries"));
        ui.selectable_value(tab, AboutTab::Fonts, tr("Fonts"));
    });
    ui.separator();
    match tab {
        AboutTab::About => {
            ui.vertical_centered(|ui| {
                ui.add_space(6.0);
                ui.add(
                    egui::Image::new(egui::include_image!("../../../assets/icon-128.png"))
                        .fit_to_exact_size(Vec2::splat(72.0)),
                );
                ui.label(RichText::new("TandaiTurun").size(22.0).strong());
                ui.label(
                    RichText::new(format!("{} {}", tr("Version"), env!("CARGO_PKG_VERSION"))).color(app.palette.muted),
                );
                ui.add_space(6.0);
                ui.label(tr("A fast, native, open-source Markdown reader and editor written in Rust."));
                ui.label(
                    RichText::new(tr("“Tandai turun” is Indonesian for “mark down”."))
                        .italics()
                        .color(app.palette.muted),
                );
                ui.add_space(6.0);
                ui.hyperlink_to("github.com/sansob/TandaiTurun", "https://github.com/sansob/TandaiTurun");
                ui.label(
                    RichText::new("© TandaiTurun contributors · MIT OR Apache-2.0").size(12.0).color(app.palette.muted),
                );
                ui.add_space(4.0);
                let n = credits().count().to_string();
                let thanks = trf("Built with {n} open-source libraries and 7 open fonts. Thank you!", &[("n", &n)]);
                ui.label(RichText::new(thanks).size(12.5));
            });
        }
        AboutTab::Libraries => {
            ui.label(
                RichText::new(tr(
                    "TandaiTurun is built on these open-source Rust libraries. Click a name to visit the project.",
                ))
                .size(12.5)
                .color(app.palette.muted),
            );
            ui.add(egui::TextEdit::singleline(filter).hint_text(tr("Filter")).desired_width(f32::INFINITY));
            let needle = filter.to_lowercase();
            let rows: Vec<[&str; 4]> = credits()
                .filter(|r| {
                    needle.is_empty() || r[0].to_lowercase().contains(&needle) || r[2].to_lowercase().contains(&needle)
                })
                .collect();
            egui::ScrollArea::vertical().max_height(320.0).auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("credits").num_columns(3).striped(true).spacing(Vec2::new(14.0, 4.0)).show(ui, |ui| {
                    for [name, version, licence, url] in rows {
                        ui.hyperlink_to(name, url);
                        ui.label(RichText::new(version).color(app.palette.muted).size(12.5));
                        ui.label(RichText::new(licence).size(12.5));
                        ui.end_row();
                    }
                });
            });
        }
        AboutTab::Fonts => {
            ui.label(
                RichText::new(tr("All bundled fonts are under the SIL Open Font License 1.1. The licence texts are installed with the app."))
                    .size(12.5)
                    .color(app.palette.muted),
            );
            ui.add_space(4.0);
            egui::Grid::new("fonts").num_columns(2).striped(true).spacing(Vec2::new(14.0, 6.0)).show(ui, |ui| {
                for (name, what, url) in FONTS {
                    ui.hyperlink_to(*name, *url);
                    ui.label(tr(what));
                    ui.end_row();
                }
            });
        }
    }
    ui.add_space(10.0);
    ui.vertical_centered(|ui| ui.button(tr("Close")).clicked()).inner
}

pub fn toasts(app: &mut App, ctx: &egui::Context) {
    app.toasts.retain(|t| t.at.elapsed() < Duration::from_secs(4));
    if app.toasts.is_empty() {
        return;
    }
    ctx.request_repaint_after(Duration::from_millis(250));
    let screen = ctx.content_rect();
    egui::Area::new(Id::new("toasts"))
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -48.0))
        .show(ctx, |ui| {
            ui.set_max_width(screen.width() * 0.7);
            for t in &app.toasts {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(0x2C, 0x2C, 0x2C))
                    .corner_radius(6)
                    .inner_margin(Margin::symmetric(14, 8))
                    .show(ui, |ui| {
                        ui.label(RichText::new(&t.text).color(egui::Color32::WHITE));
                    });
                ui.add_space(4.0);
            }
        });
}

#[cfg(test)]
mod tests {
    #[test]
    fn credits_list_the_main_libraries() {
        let names: Vec<&str> = super::credits().map(|r| r[0]).collect();
        for must in [
            "egui",
            "eframe",
            "pulldown-cmark",
            "mermaid-rs-renderer",
            "resvg",
            "pdf-writer",
            "subsetter",
            "ttf-parser",
            "rfd",
        ] {
            assert!(names.contains(&must), "{must} missing: run scripts/gen-credits.py");
        }
        assert!(super::credits().all(|r| !r[2].is_empty() && r[3].starts_with("http")));
    }

    #[test]
    fn credits_match_cargo_lock() {
        // Windows checkouts may turn the line endings into CRLF.
        let lock = include_str!("../../../Cargo.lock").replace("\r\n", "\n");
        for [name, version, ..] in super::credits() {
            let entry = format!("name = \"{name}\"\nversion = \"{version}\"");
            assert!(lock.contains(&entry), "{name} {version} is not in Cargo.lock: run scripts/gen-credits.py");
        }
    }
}
