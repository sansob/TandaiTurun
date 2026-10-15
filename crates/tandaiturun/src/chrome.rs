//! The bars around the document: menu bar, tabs, toolbar, formatting bar,
//! find bar, status bar, and the Read Mode overlay.

use crate::app::{Action, App, Dialog};
use crate::editor::Format;
use crate::i18n::{self, tr, trf, Lang};
use crate::layout::{DocFont, PageSize};
use crate::theme::{PageTheme, UiTheme, ACCENT, BRAND};
use crate::view::{PageMode, Tool, ZoomMode, ZOOM_STEPS};
use egui::{Align, Color32, Layout, Margin, RichText, Sense, Stroke, Vec2};

/// A flat icon button like Acrobat's toolbar buttons.
pub fn icon_button(ui: &mut egui::Ui, icon: &str, tip: &str, selected: bool) -> egui::Response {
    let text = RichText::new(icon).size(16.0);
    let b = egui::Button::selectable(selected, text).frame_when_inactive(selected).min_size(Vec2::new(30.0, 28.0));
    ui.add(b).on_hover_text(tip)
}

fn sep(ui: &mut egui::Ui) {
    ui.add_space(2.0);
    let (r, _) = ui.allocate_exact_size(Vec2::new(1.0, 20.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, ui.visuals().widgets.noninteractive.bg_stroke.color);
    ui.add_space(2.0);
}

fn item(ui: &mut egui::Ui, text: &str, shortcut: &str) -> bool {
    let clicked = ui.add(egui::Button::new(text).shortcut_text(shortcut)).clicked();
    if clicked {
        ui.close();
    }
    clicked
}

fn check(ui: &mut egui::Ui, on: bool, text: &str, shortcut: &str) -> bool {
    let label = if on { format!("✔  {text}") } else { format!("     {text}") };
    item(ui, &label, shortcut)
}

pub fn menu_bar(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    egui::Panel::top("menu").frame(egui::Frame::new().fill(p.chrome).inner_margin(Margin::symmetric(8, 2))).show(
        ui,
        |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                let has_doc = app.doc().is_some();
                let editing = app.doc().is_some_and(|d| d.editing);
                ui.menu_button(tr("File"), |ui| {
                    if item(ui, tr("New"), "Ctrl+N") {
                        app.actions.push(Action::New);
                    }
                    if item(ui, tr("Open…"), "Ctrl+O") {
                        app.actions.push(Action::Open);
                    }
                    ui.menu_button(tr("Open Recent"), |ui| {
                        if app.settings.recent.is_empty() {
                            ui.label(RichText::new(tr("No recent files")).weak());
                        }
                        for r in app.settings.recent.clone() {
                            if ui.button(r.display().to_string()).clicked() {
                                app.actions.push(Action::OpenPath(r));
                                ui.close();
                            }
                        }
                        if !app.settings.recent.is_empty() {
                            ui.separator();
                            if ui.button(tr("Clear Recent Files")).clicked() {
                                app.settings.recent.clear();
                                ui.close();
                            }
                        }
                    });
                    ui.separator();
                    ui.add_enabled_ui(has_doc, |ui| {
                        if item(ui, tr("Save"), "Ctrl+S") {
                            app.actions.push(Action::Save);
                        }
                        if item(ui, tr("Save As…"), "Ctrl+Shift+S") {
                            app.actions.push(Action::SaveAs);
                        }
                        if item(ui, tr("Reload from Disk"), "F5") {
                            app.actions.push(Action::Reload);
                        }
                        ui.separator();
                        if item(ui, tr("Export to PDF…"), "") {
                            app.actions.push(Action::ExportPdf);
                        }
                        if item(ui, tr("Export to HTML…"), "") {
                            app.actions.push(Action::ExportHtml);
                        }
                        if item(ui, tr("Print…"), "Ctrl+P") {
                            app.actions.push(Action::Print);
                        }
                        ui.separator();
                        if item(ui, tr("Document Properties…"), "Ctrl+D") {
                            app.dialog = Some(Dialog::Properties);
                        }
                        ui.separator();
                        if item(ui, tr("Close Tab"), "Ctrl+W") {
                            if let Some(d) = app.doc() {
                                app.actions.push(Action::Close(d.uid));
                            }
                        }
                        if item(ui, tr("Close All"), "") {
                            app.actions.push(Action::CloseAll);
                        }
                    });
                    ui.separator();
                    if item(ui, tr("Exit"), "Ctrl+Q") {
                        app.actions.push(Action::Quit);
                    }
                });
                ui.menu_button(tr("Edit"), |ui| {
                    ui.add_enabled_ui(editing, |ui| {
                        if item(ui, tr("Undo"), "Ctrl+Z") {
                            app.actions.push(Action::Undo);
                        }
                        if item(ui, tr("Redo"), "Ctrl+Shift+Z") {
                            app.actions.push(Action::Redo);
                        }
                    });
                    ui.separator();
                    ui.add_enabled_ui(has_doc, |ui| {
                        let label = if editing { tr("Stop Editing") } else { tr("Edit Markdown") };
                        if item(ui, label, "Ctrl+E") {
                            app.actions.push(Action::ToggleEdit);
                        }
                        ui.separator();
                        if item(ui, tr("Find…"), "Ctrl+F") {
                            app.actions.push(Action::Find);
                        }
                        if item(ui, tr("Find Next"), "F3") {
                            app.find_step(true);
                        }
                        if item(ui, tr("Find Previous"), "Shift+F3") {
                            app.find_step(false);
                        }
                        ui.add_enabled_ui(editing, |ui| {
                            if item(ui, tr("Replace…"), "") {
                                app.actions.push(Action::Find);
                                app.show_replace = true;
                            }
                        });
                    });
                    ui.separator();
                    if item(ui, tr("Preferences…"), "Ctrl+,") {
                        app.dialog = Some(Dialog::Preferences);
                    }
                });
                ui.menu_button(tr("View"), |ui| view_menu(app, ui));
                ui.menu_button(tr("Window"), |ui| {
                    if check(ui, app.settings.show_nav, tr("Navigation Pane"), "F4") {
                        app.settings.show_nav = !app.settings.show_nav;
                    }
                    if check(ui, app.settings.show_tools, tr("Tools Pane"), "Shift+F4") {
                        app.settings.show_tools = !app.settings.show_tools;
                    }
                    if check(ui, app.fullscreen, tr("Full Screen"), "F11") {
                        app.fullscreen = !app.fullscreen;
                    }
                    ui.separator();
                    if check(ui, app.active.is_none(), tr("Home"), "") {
                        app.active = None;
                    }
                    for i in 0..app.docs.len() {
                        let name = app.docs[i].name();
                        if check(ui, app.active == Some(i), &name, "") {
                            app.active = Some(i);
                        }
                    }
                });
                ui.menu_button(tr("Help"), |ui| {
                    if item(ui, tr("Welcome Guide"), "") {
                        let text =
                            if i18n::current() == Lang::Id { crate::app::WELCOME_ID } else { crate::app::WELCOME_EN };
                        app.actions.push(Action::OpenText { title: tr("Welcome").to_string(), text: text.to_string() });
                    }
                    if item(ui, tr("Markdown Cheat Sheet"), "") {
                        app.actions.push(Action::OpenText {
                            title: tr("Markdown Cheat Sheet").to_string(),
                            text: crate::app::CHEATSHEET.to_string(),
                        });
                    }
                    if item(ui, tr("Keyboard Shortcuts"), "") {
                        app.dialog = Some(Dialog::Shortcuts);
                    }
                    ui.separator();
                    ui.menu_button(tr("Language"), |ui| {
                        for l in Lang::ALL {
                            if check(ui, i18n::current() == l, l.name(), "") {
                                app.settings.lang = Some(l);
                            }
                        }
                    });
                    ui.separator();
                    if item(ui, tr("Source Code on GitHub"), "") {
                        let _ = webbrowser::open("https://github.com/sansob/TandaiTurun");
                    }
                    if item(ui, tr("Report a Problem"), "") {
                        let _ = webbrowser::open("https://github.com/sansob/TandaiTurun/issues");
                    }
                    if item(ui, tr("Make Default Markdown App"), "") {
                        app.actions.push(Action::MakeDefault);
                    }
                    if item(ui, tr("Open-Source Libraries"), "") {
                        app.dialog =
                            Some(Dialog::About { tab: crate::app::AboutTab::Libraries, filter: String::new() });
                    }
                    if item(ui, tr("About TandaiTurun"), "") {
                        app.dialog = Some(Dialog::About { tab: crate::app::AboutTab::About, filter: String::new() });
                    }
                });
            });
        },
    );
}

fn view_menu(app: &mut App, ui: &mut egui::Ui) {
    let look = app.look();
    let has_doc = app.doc().is_some();
    ui.add_enabled_ui(has_doc, |ui| {
        if item(ui, tr("Zoom In"), "Ctrl+=") {
            if let Some(d) = app.doc_mut() {
                d.active_view().zoom_in(&look);
            }
        }
        if item(ui, tr("Zoom Out"), "Ctrl+-") {
            if let Some(d) = app.doc_mut() {
                d.active_view().zoom_out(&look);
            }
        }
        let zoom = app.doc().map(|d| d.active_view_ref().zoom);
        if check(ui, zoom == Some(ZoomMode::Fixed(1.0)), tr("Actual Size"), "Ctrl+1") {
            set_zoom(app, ZoomMode::Fixed(1.0));
        }
        if check(ui, zoom == Some(ZoomMode::FitPage), tr("Zoom to Page Level"), "Ctrl+0") {
            set_zoom(app, ZoomMode::FitPage);
        }
        if check(ui, zoom == Some(ZoomMode::FitWidth), tr("Fit Width"), "Ctrl+2") {
            set_zoom(app, ZoomMode::FitWidth);
        }
        ui.separator();
        ui.menu_button(tr("Page Display"), |ui| {
            let mode = app.doc().map(|d| d.active_view_ref().mode);
            for m in PageMode::ALL {
                if check(ui, mode == Some(m), tr(m.label()), "") {
                    app.settings.page_mode = m;
                    if let Some(d) = app.doc_mut() {
                        d.active_view().set_mode(m);
                    }
                }
            }
        });
        ui.menu_button(tr("Page Navigation"), |ui| {
            let n = app.doc().map_or(1, |d| d.active_view_ref().page_count());
            let cur = app.doc().map_or(0, |d| d.active_view_ref().current_page);
            let mut go = None;
            if item(ui, tr("First Page"), "Home") {
                go = Some(0);
            }
            if item(ui, tr("Previous Page"), "PgUp") {
                go = Some(cur.saturating_sub(1));
            }
            if item(ui, tr("Next Page"), "PgDn") {
                go = Some((cur + 1).min(n - 1));
            }
            if item(ui, tr("Last Page"), "End") {
                go = Some(n - 1);
            }
            if let (Some(p), Some(d)) = (go, app.doc_mut()) {
                d.active_view().go_to_page(p);
            }
        });
        ui.separator();
    });
    ui.menu_button(tr("Page Color"), |ui| {
        for t in PageTheme::ALL {
            if check(ui, app.settings.page_theme == t, tr(t.label()), "") {
                app.settings.page_theme = t;
            }
        }
    });
    ui.menu_button(tr("Page Size"), |ui| {
        for s in PageSize::ALL {
            if check(ui, app.settings.page_size == s, s.label(), "") {
                app.settings.page_size = s;
            }
        }
    });
    ui.menu_button(tr("Document Font"), |ui| {
        if check(ui, app.settings.doc_font == DocFont::Sans, "Sans (Inter)", "") {
            app.settings.doc_font = DocFont::Sans;
        }
        if check(ui, app.settings.doc_font == DocFont::Serif, "Serif (Liberation Serif)", "") {
            app.settings.doc_font = DocFont::Serif;
        }
        ui.separator();
        for size in [13.0, 14.0, 15.0, 16.0, 18.0, 20.0] {
            if check(ui, app.settings.font_size == size, &format!("{size} pt"), "") {
                app.settings.font_size = size;
            }
        }
    });
    ui.menu_button(tr("Theme"), |ui| {
        for (t, label) in [(UiTheme::System, "System"), (UiTheme::Light, "Light"), (UiTheme::Dark, "Dark")] {
            if check(ui, app.settings.ui_theme == t, tr(label), "") {
                app.settings.ui_theme = t;
            }
        }
    });
    ui.separator();
    ui.add_enabled_ui(has_doc, |ui| {
        if check(ui, app.read_mode, tr("Read Mode"), "Ctrl+H") {
            app.read_mode = !app.read_mode;
        }
    });
    if check(ui, app.fullscreen, tr("Full Screen"), "F11") {
        app.fullscreen = !app.fullscreen;
    }
}

fn set_zoom(app: &mut App, z: ZoomMode) {
    if let Some(d) = app.doc_mut() {
        let editing = d.editing;
        d.active_view().set_zoom(z);
        if !editing {
            app.settings.zoom = z;
        }
    }
}

pub fn tab_strip(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    egui::Panel::top("tabs")
        .frame(egui::Frame::new().fill(p.strip).inner_margin(Margin { left: 6, right: 6, top: 4, bottom: 0 }))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let tab = |ui: &mut egui::Ui, label: &str, active: bool, closable: bool| -> (bool, bool) {
                    let fill = if active { p.chrome } else { Color32::TRANSPARENT };
                    let mut close = false;
                    let r = egui::Frame::new()
                        .fill(fill)
                        .corner_radius(egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 })
                        .inner_margin(Margin { left: 12, right: if closable { 6 } else { 12 }, top: 6, bottom: 6 })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let text = RichText::new(label).color(if active { p.text } else { p.muted });
                                ui.add(egui::Label::new(text).selectable(false).truncate());
                                if closable {
                                    let x =
                                        ui.add(egui::Button::new(RichText::new("✕").size(11.0)).frame(false).small());
                                    if x.clicked() {
                                        close = true;
                                    }
                                }
                            });
                        })
                        .response
                        .interact(Sense::click());
                    if active {
                        let r = r.rect;
                        ui.painter().line_segment([r.left_top(), r.right_top()], Stroke::new(2.0, BRAND));
                    }
                    (r.clicked(), close || r.middle_clicked())
                };
                if tab(ui, &format!("🏠 {}", tr("Home")), app.active.is_none(), false).0 {
                    app.active = None;
                }
                let mut close = None;
                for i in 0..app.docs.len() {
                    let d = &app.docs[i];
                    let label = format!("{}{}", if d.dirty() { "● " } else { "" }, d.name());
                    let (clicked, closed) = tab(ui, &label, app.active == Some(i), true);
                    if clicked {
                        app.active = Some(i);
                    }
                    if closed {
                        close = Some(d.uid);
                    }
                }
                if let Some(uid) = close {
                    app.actions.push(Action::Close(uid));
                }
                if ui
                    .add(egui::Button::new(RichText::new("+").size(16.0)).frame(false))
                    .on_hover_text(tr("New"))
                    .clicked()
                {
                    app.actions.push(Action::New);
                }
            });
        });
}

pub fn toolbar(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    let look = app.look();
    egui::Panel::top("toolbar")
        .frame(
            egui::Frame::new().fill(p.chrome).inner_margin(Margin::symmetric(8, 5)).stroke(Stroke::new(1.0, p.border)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                let editing = app.doc().is_some_and(|d| d.editing);
                if icon_button(ui, "💾", &format!("{} (Ctrl+S)", tr("Save")), false).clicked() {
                    app.actions.push(Action::Save);
                }
                if icon_button(ui, "🖨", &format!("{} (Ctrl+P)", tr("Print…")), false).clicked() {
                    app.actions.push(Action::Print);
                }
                if icon_button(ui, "📄", tr("Export to PDF…"), false).clicked() {
                    app.actions.push(Action::ExportPdf);
                }
                if icon_button(ui, "🔍", &format!("{} (Ctrl+F)", tr("Find…")), app.find_open).clicked() {
                    if app.find_open {
                        app.find_open = false;
                    } else {
                        app.actions.push(Action::Find);
                    }
                }
                sep(ui);

                // Page navigation: ⬆ ⬇ [ 3 ] / 12
                let Some(d) = app.doc_mut() else { return };
                let n = d.active_view_ref().page_count();
                let cur = d.active_view_ref().current_page;
                if icon_button(ui, "⬆", tr("Previous Page"), false).clicked() {
                    d.active_view().go_to_page(cur.saturating_sub(1));
                }
                if icon_button(ui, "⬇", tr("Next Page"), false).clicked() {
                    d.active_view().go_to_page((cur + 1).min(n.saturating_sub(1)));
                }
                if !d.page_input_focus {
                    d.page_input = (cur + 1).to_string();
                }
                let r = ui.add(
                    egui::TextEdit::singleline(&mut d.page_input)
                        .desired_width(34.0)
                        .horizontal_align(Align::Center)
                        .vertical_align(Align::Center),
                );
                d.page_input_focus = r.has_focus();
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Ok(p) = d.page_input.trim().parse::<usize>() {
                        d.active_view().go_to_page(p.max(1) - 1);
                    }
                }
                ui.label(RichText::new(format!("/ {n}")).color(p.muted));
                sep(ui);

                // Tools.
                let tool = d.active_view_ref().tool;
                if icon_button(ui, "⌶", tr("Select Tool"), tool == Tool::Select).clicked() {
                    d.active_view().tool = Tool::Select;
                }
                if icon_button(ui, "✋", tr("Hand Tool"), tool == Tool::Hand).clicked() {
                    d.active_view().tool = Tool::Hand;
                }
                sep(ui);

                // Zoom: − [100%] +
                let z = d.active_view_ref().effective_zoom(&look);
                let mut new_zoom = None;
                if icon_button(ui, "➖", &format!("{} (Ctrl+-)", tr("Zoom Out")), false).clicked() {
                    d.active_view().zoom_out(&look);
                    new_zoom = Some(d.active_view_ref().zoom);
                }
                let mode = d.active_view_ref().zoom;
                egui::ComboBox::from_id_salt("zoom").width(76.0).selected_text(format!("{:.0}%", z * 100.0)).show_ui(
                    ui,
                    |ui| {
                        for s in ZOOM_STEPS {
                            if ui.selectable_label(mode == ZoomMode::Fixed(*s), format!("{:.0}%", s * 100.0)).clicked()
                            {
                                new_zoom = Some(ZoomMode::Fixed(*s));
                            }
                        }
                        ui.separator();
                        if ui.selectable_label(mode == ZoomMode::FitPage, tr("Zoom to Page Level")).clicked() {
                            new_zoom = Some(ZoomMode::FitPage);
                        }
                        if ui.selectable_label(mode == ZoomMode::FitWidth, tr("Fit Width")).clicked() {
                            new_zoom = Some(ZoomMode::FitWidth);
                        }
                    },
                );
                if icon_button(ui, "➕", &format!("{} (Ctrl+=)", tr("Zoom In")), false).clicked() {
                    d.active_view().zoom_in(&look);
                    new_zoom = Some(d.active_view_ref().zoom);
                }
                if icon_button(ui, "↔", tr("Fit Width"), mode == ZoomMode::FitWidth).clicked() {
                    new_zoom = Some(ZoomMode::FitWidth);
                }
                if icon_button(ui, "⛶", tr("Zoom to Page Level"), mode == ZoomMode::FitPage).clicked() {
                    new_zoom = Some(ZoomMode::FitPage);
                }
                if let Some(z) = new_zoom {
                    d.active_view().set_zoom(z);
                    if !editing {
                        app.settings.zoom = z;
                    }
                }
                let Some(d) = app.doc_mut() else { return };
                sep(ui);
                let pm = d.active_view_ref().mode;
                let mut new_mode = None;
                for (m, icon) in [(PageMode::Single, "▯"), (PageMode::Continuous, "☰"), (PageMode::TwoPage, "◫")]
                {
                    if icon_button(ui, icon, tr(m.label()), pm == m).clicked() {
                        d.active_view().set_mode(m);
                        new_mode = Some(m);
                    }
                }

                if let Some(m) = new_mode {
                    app.settings.page_mode = m;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, "📖", &format!("{} (Ctrl+H)", tr("Read Mode")), false).clicked() {
                        app.read_mode = true;
                    }
                    let label = if editing { tr("Done") } else { tr("Edit") };
                    let b = egui::Button::new(RichText::new(format!("✏  {label}")).color(if editing {
                        Color32::WHITE
                    } else {
                        p.text
                    }))
                    .fill(if editing { ACCENT } else { p.hover })
                    .min_size(Vec2::new(84.0, 28.0));
                    if ui.add(b).on_hover_text(format!("{} (Ctrl+E)", tr("Edit Markdown"))).clicked() {
                        app.actions.push(Action::ToggleEdit);
                    }
                });
            });
        });
}

pub fn format_bar(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    egui::Panel::top("format")
        .frame(egui::Frame::new().fill(p.pane).inner_margin(Margin::symmetric(8, 4)).stroke(Stroke::new(1.0, p.border)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.label(RichText::new(tr("Edit Markdown")).strong().color(ACCENT));
                ui.add_space(8.0);
                if icon_button(ui, "↶", &format!("{} (Ctrl+Z)", tr("Undo")), false).clicked() {
                    app.actions.push(Action::Undo);
                }
                if icon_button(ui, "↷", &format!("{} (Ctrl+Shift+Z)", tr("Redo")), false).clicked() {
                    app.actions.push(Action::Redo);
                }
                sep(ui);
                let mut f = |ui: &mut egui::Ui, text: RichText, tip: &str, fmt: Format| {
                    let b = egui::Button::new(text).frame_when_inactive(false).min_size(Vec2::new(30.0, 26.0));
                    if ui.add(b).on_hover_text(tip).clicked() {
                        app.actions.push(Action::Format(fmt));
                    }
                };
                for l in 1..=3u8 {
                    f(
                        ui,
                        RichText::new(format!("H{l}")).strong(),
                        &format!("{} {l} (Ctrl+Alt+{l})", tr("Heading")),
                        Format::Heading(l),
                    );
                }
                f(ui, RichText::new("¶"), tr("Paragraph"), Format::Paragraph);
                sep(ui);
                f(ui, RichText::new("B").strong(), &format!("{} (Ctrl+B)", tr("Bold")), Format::Bold);
                f(ui, RichText::new("I").italics(), &format!("{} (Ctrl+I)", tr("Italic")), Format::Italic);
                f(
                    ui,
                    RichText::new("S").strikethrough(),
                    &format!("{} (Ctrl+Shift+X)", tr("Strikethrough")),
                    Format::Strike,
                );
                f(ui, RichText::new("</>").monospace(), &format!("{} (Ctrl+Shift+C)", tr("Inline Code")), Format::Code);
                sep(ui);
                f(ui, RichText::new("•≡"), tr("Bulleted List"), Format::Bullet);
                f(ui, RichText::new("1≡"), tr("Numbered List"), Format::Numbered);
                f(ui, RichText::new("☑"), tr("Task List"), Format::Task);
                f(ui, RichText::new("❝"), tr("Quote"), Format::Quote);
                f(ui, RichText::new("⇤"), &format!("{} (Shift+Tab)", tr("Decrease Indent")), Format::Outdent);
                f(ui, RichText::new("⇥"), &format!("{} (Tab)", tr("Increase Indent")), Format::Indent);
                sep(ui);
                f(ui, RichText::new("🔗"), &format!("{} (Ctrl+K)", tr("Link")), Format::Link);
                f(ui, RichText::new("🖼"), tr("Image"), Format::Image);
                f(ui, RichText::new("▦"), tr("Table"), Format::Table);
                f(ui, RichText::new("{ }").monospace(), tr("Code Block"), Format::CodeBlock);
                f(ui, RichText::new("―"), tr("Horizontal Rule"), Format::Rule);
                f(ui, RichText::new("⤓"), tr("Page Break"), Format::PageBreak);
            });
        });
}

pub fn find_bar(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    let editing = app.doc().is_some_and(|d| d.editing);
    egui::Panel::top("find")
        .frame(
            egui::Frame::new().fill(p.pane).inner_margin(Margin::symmetric(10, 5)).stroke(Stroke::new(1.0, p.border)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("🔍");
                let r = ui.add(
                    egui::TextEdit::singleline(&mut app.find_query)
                        .hint_text(tr("Find in document"))
                        .desired_width(240.0),
                );
                if app.find_focus {
                    r.request_focus();
                    app.find_focus = false;
                }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let back = ui.input(|i| i.modifiers.shift);
                    app.find_step(!back);
                    r.request_focus();
                }
                let (count, current) = match app.doc() {
                    Some(d) if d.editing => (d.editor.marks.len(), d.editor.current_mark),
                    Some(d) => (d.view.search.hits.len(), d.view.search.current),
                    None => (0, None),
                };
                if icon_button(ui, "⬆", &format!("{} (Shift+F3)", tr("Find Previous")), false).clicked() {
                    app.find_step(false);
                }
                if icon_button(ui, "⬇", &format!("{} (F3)", tr("Find Next")), false).clicked() {
                    app.find_step(true);
                }
                let status = if app.find_query.is_empty() {
                    String::new()
                } else if count == 0 {
                    tr("No matches").to_string()
                } else {
                    trf(
                        "{current} of {count}",
                        &[
                            ("current", &current.map_or("–".into(), |c| (c + 1).to_string())),
                            ("count", &count.to_string()),
                        ],
                    )
                };
                ui.label(RichText::new(status).color(if count == 0 && !app.find_query.is_empty() {
                    Color32::from_rgb(0xD0, 0x3B, 0x3B)
                } else {
                    p.muted
                }));
                ui.checkbox(&mut app.find_opts.case_sensitive, tr("Match case"));
                ui.checkbox(&mut app.find_opts.whole_word, tr("Whole words"));
                if editing {
                    ui.toggle_value(&mut app.show_replace, tr("Replace"));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(egui::Button::new("✕").frame(false)).on_hover_text(tr("Close")).clicked() {
                        app.find_open = false;
                    }
                });
            });
            if editing && app.show_replace {
                ui.horizontal(|ui| {
                    ui.label("⇄");
                    ui.add(
                        egui::TextEdit::singleline(&mut app.replace_text)
                            .hint_text(tr("Replace with"))
                            .desired_width(240.0),
                    );
                    if ui.button(tr("Replace")).clicked() {
                        app.replace_current();
                    }
                    if ui.button(tr("Replace All")).clicked() {
                        app.replace_all();
                    }
                });
            }
        });
}

pub fn disk_banner(app: &mut App, ui: &mut egui::Ui) {
    if !app.doc().is_some_and(|d| d.disk_changed) {
        return;
    }
    egui::Panel::top("disk")
        .frame(egui::Frame::new().fill(Color32::from_rgb(0xFF, 0xF4, 0xCE)).inner_margin(Margin::symmetric(12, 6)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr("This file was changed by another program. You have unsaved changes here."))
                        .color(Color32::from_rgb(0x5C, 0x46, 0x00)),
                );
                if ui.button(tr("Reload (discard my changes)")).clicked() {
                    app.actions.push(Action::Reload);
                }
                if ui.button(tr("Keep my version")).clicked() {
                    if let Some(d) = app.doc_mut() {
                        d.disk_changed = false;
                    }
                }
            });
        });
}

/// Asks before loading internet images, which would tell their servers that
/// the document was opened.
pub fn remote_banner(app: &mut App, ui: &mut egui::Ui) {
    let Some(d) = app.doc() else { return };
    let blocked = d.active_view_ref().layout.as_ref().map_or(0, |l| l.remote_blocked);
    if blocked == 0 || d.allow_remote || app.settings.remote_images {
        return;
    }
    egui::Panel::top("remote")
        .frame(egui::Frame::new().fill(Color32::from_rgb(0xFF, 0xF4, 0xCE)).inner_margin(Margin::symmetric(12, 6)))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let ink = Color32::from_rgb(0x5C, 0x46, 0x00);
                ui.label(RichText::new("🌐").color(ink));
                ui.label(
                    RichText::new(trf(
                        "This document has {n} images from the internet. Showing them lets those websites see that you opened it.",
                        &[("n", &blocked.to_string())],
                    ))
                    .color(ink),
                );
                if ui.button(tr("Show images")).clicked() {
                    if let Some(d) = app.doc_mut() {
                        d.allow_remote = true;
                    }
                }
                if ui.button(tr("Always show images")).clicked() {
                    app.settings.remote_images = true;
                }
            });
        });
}

pub fn status_bar(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    let look = app.look();
    egui::Panel::bottom("status")
        .frame(
            egui::Frame::new().fill(p.chrome).inner_margin(Margin::symmetric(10, 3)).stroke(Stroke::new(1.0, p.border)),
        )
        .show(ui, |ui| {
            let Some(d) = app.doc() else { return };
            ui.horizontal(|ui| {
                let small = |s: String| RichText::new(s).size(12.0).color(p.muted);
                let v = d.active_view_ref();
                ui.label(small(trf(
                    "Page {page} of {pages}",
                    &[("page", &(v.current_page + 1).to_string()), ("pages", &v.page_count().to_string())],
                )));
                ui.label(small(format!("{} {:.0}%", look.page.label(), v.effective_zoom(&look) * 100.0)));
                if d.editing {
                    let at = d.editor.cursor.start.min(d.text.len());
                    let line = d.text[..at].matches('\n').count() + 1;
                    let col = d.text[..at].rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
                    ui.label(small(trf(
                        "Ln {line}, Col {col}",
                        &[("line", &line.to_string()), ("col", &col.to_string())],
                    )));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let words = d.text.split_whitespace().count();
                    ui.label(small(format!(
                        "UTF-8{} · {}",
                        if d.bom { " BOM" } else { "" },
                        match d.line_ending {
                            tandaiturun_core::document::LineEnding::Lf => "LF",
                            tandaiturun_core::document::LineEnding::CrLf => "CRLF",
                        }
                    )));
                    ui.label(small(trf("{n} words", &[("n", &words.to_string())])));
                    if let Some(path) = &d.path {
                        ui.add(egui::Label::new(small(path.display().to_string())).truncate());
                    }
                });
            });
        });
}

/// Read Mode: only the pages, with a small floating page bar.
pub fn read_mode(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    let look = app.look();
    let ctx = ui.ctx().clone();
    let mut events = Vec::new();
    egui::CentralPanel::default().frame(egui::Frame::new().fill(p.canvas)).show(ui, |ui| {
        if let Some(d) = app.doc_mut() {
            let id = egui::Id::new(("view", d.uid));
            events = if d.editing { d.preview.show(ui, id.with("preview"), &p) } else { d.view.show(ui, id, &p) };
        }
    });
    app.handle_view_events(events);
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("read_bar"))
        .fixed_pos(egui::pos2(screen.center().x - 170.0, screen.bottom() - 64.0))
        .order(egui::Order::Foreground)
        .show(&ctx, |ui| {
            egui::Frame::new()
                .fill(Color32::from_black_alpha(190))
                .corner_radius(8)
                .inner_margin(Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.visuals_mut().override_text_color = Some(Color32::WHITE);
                        let Some(d) = app.doc_mut() else { return };
                        let v = d.active_view();
                        let n = v.page_count();
                        if ui.add(egui::Button::new("⬆").frame(false)).clicked() {
                            v.go_to_page(v.current_page.saturating_sub(1));
                        }
                        ui.label(format!("{} / {n}", v.current_page + 1));
                        if ui.add(egui::Button::new("⬇").frame(false)).clicked() {
                            v.go_to_page((v.current_page + 1).min(n - 1));
                        }
                        ui.add_space(8.0);
                        if ui.add(egui::Button::new("➖").frame(false)).clicked() {
                            v.zoom_out(&look);
                        }
                        ui.label(format!("{:.0}%", v.effective_zoom(&look) * 100.0));
                        if ui.add(egui::Button::new("➕").frame(false)).clicked() {
                            v.zoom_in(&look);
                        }
                        ui.add_space(8.0);
                        if ui.add(egui::Button::new(format!("✕ {}", tr("Exit Read Mode"))).frame(false)).clicked() {
                            app.read_mode = false;
                        }
                    });
                });
        });
}
