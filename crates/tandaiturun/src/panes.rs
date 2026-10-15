//! The side panes (navigation on the left, tools on the right), the
//! document area and the Home tab.

use crate::app::{Action, App, Dialog, NavTab};
use crate::chrome::icon_button;
use crate::i18n::{self, tr, trf, Lang};
use crate::theme::{PageTheme, ACCENT, BRAND};
use egui::{Align, Color32, Layout, Margin, RichText, Sense, Stroke, Vec2};

pub fn nav_rail(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    egui::Panel::left("rail")
        .exact_size(44.0)
        .resizable(false)
        .frame(egui::Frame::new().fill(p.pane).inner_margin(Margin::symmetric(6, 8)).stroke(Stroke::new(1.0, p.border)))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                for (tab, icon, tip) in [
                    (NavTab::Thumbnails, "▦", tr("Page Thumbnails")),
                    (NavTab::Bookmarks, "🔖", tr("Bookmarks")),
                    (NavTab::Search, "🔍", tr("Search")),
                ] {
                    let on = app.settings.show_nav && app.settings.nav_tab == tab;
                    if icon_button(ui, icon, tip, on).clicked() {
                        if on {
                            app.settings.show_nav = false;
                        } else {
                            app.settings.show_nav = true;
                            app.settings.nav_tab = tab;
                        }
                    }
                    ui.add_space(4.0);
                }
            });
        });
}

pub fn nav_pane(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    egui::Panel::left("nav")
        .default_size(250.0)
        .size_range(170.0..=480.0)
        .resizable(true)
        .frame(
            egui::Frame::new().fill(p.pane).inner_margin(Margin::symmetric(10, 10)).stroke(Stroke::new(1.0, p.border)),
        )
        .show(ui, |ui| {
            let title = match app.settings.nav_tab {
                NavTab::Thumbnails => tr("Page Thumbnails"),
                NavTab::Bookmarks => tr("Bookmarks"),
                NavTab::Search => tr("Search"),
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(egui::Button::new("✕").frame(false)).on_hover_text(tr("Hide (F4)")).clicked() {
                        app.settings.show_nav = false;
                    }
                });
            });
            ui.separator();
            match app.settings.nav_tab {
                NavTab::Thumbnails => thumbnails(app, ui),
                NavTab::Bookmarks => bookmarks(app, ui),
                NavTab::Search => search_pane(app, ui),
            }
        });
}

fn thumbnails(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    let Some(d) = app.doc_mut() else { return };
    let v = d.active_view();
    let Some(l) = v.layout.as_ref() else { return };
    let n = l.page_count();
    let w = (ui.available_width() - 40.0).clamp(80.0, 200.0);
    let h = w * l.page_size.y / l.page_size.x;
    let row_h = h + 30.0;
    let current = v.current_page;
    let mut go = None;
    egui::ScrollArea::vertical().auto_shrink(false).show_rows(ui, row_h, n, |ui, range| {
        for i in range {
            ui.vertical_centered(|ui| {
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(w + 8.0, h + 8.0), Sense::click());
                let page = rect.shrink(4.0);
                let painter = ui.painter_at(rect.expand(2.0));
                crate::view::paint_thumbnail(ui, &painter, l, i, page);
                if i == current {
                    painter.rect_stroke(rect, 3.0, Stroke::new(2.5, ACCENT), egui::StrokeKind::Inside);
                } else if resp.hovered() {
                    painter.rect_stroke(rect, 3.0, Stroke::new(1.5, p.border), egui::StrokeKind::Inside);
                }
                if resp.clicked() {
                    go = Some(i);
                }
                ui.label(RichText::new((i + 1).to_string()).size(12.0).color(p.muted));
            });
        }
    });
    if let Some(i) = go {
        v.go_to_page(i);
    }
}

fn bookmarks(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    let Some(d) = app.doc_mut() else { return };
    let headings = d.outline().to_vec();
    if headings.is_empty() {
        ui.label(RichText::new(tr("This document has no headings.")).color(p.muted));
        return;
    }
    let v = d.active_view_ref();
    let current = v.layout.as_ref().and_then(|l| l.heading_at(v.top_flow + 40.0));
    let min_level = headings.iter().map(|h| h.level).min().unwrap_or(1);
    let mut go = None;
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        for (i, h) in headings.iter().enumerate() {
            let indent = (h.level - min_level) as f32 * 14.0;
            ui.horizontal(|ui| {
                ui.add_space(indent);
                let selected = current == Some(i);
                let text = if h.level == min_level { RichText::new(&h.text).strong() } else { RichText::new(&h.text) };
                let r = ui.add(egui::Button::selectable(selected, text).frame_when_inactive(selected).truncate());
                if r.clicked() {
                    go = Some((i, h.offset));
                }
            });
        }
    });
    if let Some((i, offset)) = go {
        if d.editing {
            d.preview.go_to_heading(i);
            d.editor.select(offset..offset);
            d.editor.focus_next = true;
        } else {
            d.view.go_to_heading(i);
        }
    }
}

fn search_pane(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    let r = ui.add(
        egui::TextEdit::singleline(&mut app.find_query).hint_text(tr("Find in document")).desired_width(f32::INFINITY),
    );
    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        app.find_step(true);
        r.request_focus();
    }
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut app.find_opts.case_sensitive, tr("Match case"));
        ui.checkbox(&mut app.find_opts.whole_word, tr("Whole words"));
    });
    ui.separator();
    let query_empty = app.find_query.is_empty();
    let Some(d) = app.doc_mut() else { return };
    if d.editing {
        let n = d.editor.marks.len();
        ui.label(RichText::new(trf("{n} matches in the source", &[("n", &n.to_string())])).color(p.muted));
        return;
    }
    let hits = &d.view.search.hits;
    if query_empty {
        ui.label(RichText::new(tr("Type to search the whole document.")).color(p.muted));
        return;
    }
    ui.label(RichText::new(trf("{n} results", &[("n", &hits.len().to_string())])).color(p.muted));
    let current = d.view.search.current;
    let mut pick = None;
    let mut last_page = usize::MAX;
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        for (i, h) in hits.iter().enumerate().take(2000) {
            if h.page != last_page {
                last_page = h.page;
                ui.add_space(4.0);
                ui.label(RichText::new(trf("Page {page}", &[("page", &(h.page + 1).to_string())])).strong().size(12.0));
            }
            let r = ui.add(
                egui::Button::selectable(current == Some(i), RichText::new(&h.snippet).size(12.5))
                    .frame_when_inactive(current == Some(i))
                    .wrap(),
            );
            if r.clicked() {
                pick = Some(i);
            }
        }
    });
    if let Some(i) = pick {
        d.view.select_hit(i);
    }
}

pub fn tools_pane(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    egui::Panel::right("tools")
        .default_size(220.0)
        .size_range(180.0..=320.0)
        .resizable(true)
        .frame(
            egui::Frame::new().fill(p.pane).inner_margin(Margin::symmetric(10, 10)).stroke(Stroke::new(1.0, p.border)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Tools")).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(egui::Button::new("✕").frame(false)).on_hover_text(tr("Hide (Shift+F4)")).clicked() {
                        app.settings.show_tools = false;
                    }
                });
            });
            ui.separator();
            let editing = app.doc().is_some_and(|d| d.editing);
            let tool = |ui: &mut egui::Ui, icon: &str, color: Color32, label: &str| -> bool {
                let (rect, r) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::click());
                if r.hovered() {
                    ui.painter().rect_filled(rect, 4.0, ui.visuals().widgets.hovered.bg_fill);
                }
                let painter = ui.painter();
                painter.text(
                    rect.left_center() + Vec2::new(16.0, 0.0),
                    egui::Align2::CENTER_CENTER,
                    icon,
                    egui::FontId::proportional(17.0),
                    color,
                );
                painter.text(
                    rect.left_center() + Vec2::new(36.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(13.5),
                    ui.visuals().text_color(),
                );
                r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
            };
            let edit_label = if editing { tr("Stop Editing") } else { tr("Edit Markdown") };
            if tool(ui, "✏", ACCENT, edit_label) {
                app.actions.push(Action::ToggleEdit);
            }
            if tool(ui, "📄", BRAND, tr("Export to PDF")) {
                app.actions.push(Action::ExportPdf);
            }
            if tool(ui, "🌐", Color32::from_rgb(0x2E, 0x9E, 0x5B), tr("Export to HTML")) {
                app.actions.push(Action::ExportHtml);
            }
            if tool(ui, "🖨", Color32::from_rgb(0x6E, 0x6E, 0x6E), tr("Print")) {
                app.actions.push(Action::Print);
            }
            if tool(ui, "🔍", Color32::from_rgb(0x8E, 0x44, 0xAD), tr("Find & Replace")) {
                app.actions.push(Action::Find);
                app.show_replace = editing;
            }
            if tool(ui, "ℹ", Color32::from_rgb(0x00, 0x8B, 0xA8), tr("Document Properties")) {
                app.dialog = Some(Dialog::Properties);
            }
            if tool(ui, "📖", Color32::from_rgb(0xD9, 0x7A, 0x00), tr("Read Mode")) {
                app.read_mode = true;
            }
            ui.add_space(8.0);
            ui.separator();
            ui.label(RichText::new(tr("Page Color")).size(12.0).color(p.muted));
            ui.horizontal(|ui| {
                for t in PageTheme::ALL {
                    let c = t.colors();
                    let (rect, r) = ui.allocate_exact_size(Vec2::new(44.0, 30.0), Sense::click());
                    ui.painter().rect_filled(rect, 4.0, c.paper);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Aa",
                        egui::FontId::proportional(13.0),
                        c.text,
                    );
                    let stroke = if app.settings.page_theme == t {
                        Stroke::new(2.0, ACCENT)
                    } else {
                        Stroke::new(1.0, p.border)
                    };
                    ui.painter().rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Inside);
                    if r.on_hover_text(tr(t.label())).clicked() {
                        app.settings.page_theme = t;
                    }
                }
            });
            ui.add_space(6.0);
            ui.label(RichText::new(tr("Text Size")).size(12.0).color(p.muted));
            ui.horizontal(|ui| {
                if ui.button("A−").clicked() {
                    app.settings.font_size = (app.settings.font_size - 1.0).max(10.0);
                }
                ui.label(format!("{:.0} pt", app.settings.font_size));
                if ui.button("A+").clicked() {
                    app.settings.font_size = (app.settings.font_size + 1.0).min(28.0);
                }
            });
        });
}

/// The document area: the pages, or the editor beside a live preview.
pub fn document(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    let editor_size = app.settings.editor_font_size;
    let mut events = Vec::new();
    let Some(d) = app.doc_mut() else { return };
    let id = egui::Id::new(("view", d.uid));
    if d.editing {
        let dark = p.dark;
        egui::Panel::left("editor")
            .default_size(ui.available_width() * 0.5)
            .size_range(260.0..=(ui.available_width() - 200.0).max(300.0))
            .resizable(true)
            .frame(egui::Frame::new().fill(p.input).stroke(Stroke::new(1.0, p.border)))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().id_salt(("editor_scroll", d.uid)).auto_shrink(false).show(ui, |ui| {
                    ui.add_space(10.0);
                    let out = d.editor.show(ui, d.uid, &mut d.text, editor_size, dark);
                    if out.changed {
                        d.changed();
                    }
                    if out.cursor_moved && !out.changed {
                        let at = d.editor.cursor.start;
                        d.preview.go_to_offset(at);
                    }
                });
            });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(p.canvas)).show(ui, |ui| {
            events = d.preview.show(ui, id.with("preview"), &p);
        });
    } else {
        egui::CentralPanel::default().frame(egui::Frame::new().fill(p.canvas)).show(ui, |ui| {
            events = d.view.show(ui, id, &p);
        });
    }
    app.handle_view_events(events);
}

/// The Home tab: open, new, recent files and the guides.
pub fn home(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    egui::CentralPanel::default().frame(egui::Frame::new().fill(p.pane).inner_margin(Margin::symmetric(40, 30))).show(
        ui,
        |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.set_max_width(980.0);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Image::new(egui::include_image!("../../../assets/icon-128.png"))
                            .fit_to_exact_size(Vec2::splat(54.0)),
                    );
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(tr("Welcome to TandaiTurun"))
                                .size(26.0)
                                .family(egui::FontFamily::Name("semibold".into())),
                        );
                        ui.label(
                            RichText::new(tr("Read Markdown like a PDF. Edit it like text.")).size(15.0).color(p.muted),
                        );
                    });
                });
                ui.add_space(22.0);
                ui.horizontal(|ui| {
                    let big = |ui: &mut egui::Ui, icon: &str, title: &str, sub: &str, color: Color32| -> bool {
                        egui::Frame::new()
                            .fill(p.chrome)
                            .stroke(Stroke::new(1.0, p.border))
                            .corner_radius(8)
                            .inner_margin(Margin::same(16))
                            .show(ui, |ui| {
                                ui.set_width(190.0);
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(icon).size(26.0).color(color));
                                    ui.add_space(4.0);
                                    ui.label(RichText::new(title).strong().size(15.0));
                                    ui.label(RichText::new(sub).size(12.5).color(p.muted));
                                });
                            })
                            .response
                            .interact(Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                    };
                    if big(ui, "📂", tr("Open a file"), tr("Markdown, README, notes…"), ACCENT) {
                        app.actions.push(Action::Open);
                    }
                    if big(ui, "📝", tr("New document"), tr("Start writing in Markdown"), BRAND) {
                        app.actions.push(Action::New);
                    }
                    if big(
                        ui,
                        "📘",
                        tr("Welcome guide"),
                        tr("What TandaiTurun can do"),
                        Color32::from_rgb(0x2E, 0x9E, 0x5B),
                    ) {
                        let text =
                            if i18n::current() == Lang::Id { crate::app::WELCOME_ID } else { crate::app::WELCOME_EN };
                        app.actions.push(Action::OpenText { title: tr("Welcome").to_string(), text: text.to_string() });
                    }
                    if big(
                        ui,
                        "🧾",
                        tr("Cheat sheet"),
                        tr("Markdown syntax at a glance"),
                        Color32::from_rgb(0xD9, 0x7A, 0x00),
                    ) {
                        app.actions.push(Action::OpenText {
                            title: tr("Markdown Cheat Sheet").to_string(),
                            text: crate::app::CHEATSHEET.to_string(),
                        });
                    }
                });
                ui.add_space(10.0);
                let star = RichText::new(format!("⭐  {}", tr("Make TandaiTurun the default app for .md files")))
                    .color(ACCENT);
                if ui.add(egui::Button::new(star).frame_when_inactive(false)).clicked() {
                    app.actions.push(Action::MakeDefault);
                }
                ui.add_space(16.0);
                ui.label(RichText::new(tr("Recent")).size(17.0).family(egui::FontFamily::Name("semibold".into())));
                ui.add_space(6.0);
                if app.settings.recent.is_empty() {
                    ui.label(
                        RichText::new(tr(
                            "Files you open will show up here. You can also drop files onto this window.",
                        ))
                        .color(p.muted),
                    );
                    return;
                }
                let mut remove = None;
                egui::Frame::new().fill(p.chrome).stroke(Stroke::new(1.0, p.border)).corner_radius(6).show(ui, |ui| {
                    egui::Grid::new("recent")
                        .num_columns(4)
                        .striped(true)
                        .spacing(Vec2::new(18.0, 8.0))
                        .min_col_width(60.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new(tr("Name")).strong());
                            ui.label(RichText::new(tr("Location")).strong());
                            ui.label(RichText::new(tr("Modified")).strong());
                            ui.label("");
                            ui.end_row();
                            for path in app.settings.recent.clone() {
                                let name =
                                    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                                let exists = path.is_file();
                                let text = if exists {
                                    RichText::new(format!("📄  {name}"))
                                } else {
                                    RichText::new(format!("📄  {name}")).strikethrough().color(p.muted)
                                };
                                if ui.add(egui::Button::new(text).frame_when_inactive(false)).clicked() && exists {
                                    app.actions.push(Action::OpenPath(path.clone()));
                                }
                                let dir = path.parent().map(|d| d.display().to_string()).unwrap_or_default();
                                ui.add(egui::Label::new(RichText::new(dir).color(p.muted).size(12.5)).truncate());
                                let modified = std::fs::metadata(&path)
                                    .and_then(|m| m.modified())
                                    .ok()
                                    .map(ago)
                                    .unwrap_or_default();
                                ui.label(RichText::new(modified).color(p.muted).size(12.5));
                                if ui
                                    .add(egui::Button::new("✕").frame(false))
                                    .on_hover_text(tr("Remove from list"))
                                    .clicked()
                                {
                                    remove = Some(path.clone());
                                }
                                ui.end_row();
                            }
                        });
                });
                if let Some(r) = remove {
                    app.settings.recent.retain(|x| x != &r);
                }
            });
        },
    );
}

/// "5 minutes ago", "yesterday"…
fn ago(t: std::time::SystemTime) -> String {
    let secs = t.elapsed().map_or(0, |d| d.as_secs());
    let n = |v: u64| v.to_string();
    match secs {
        0..=59 => tr("just now").to_string(),
        60..=3599 => trf("{n} min ago", &[("n", &n(secs / 60))]),
        3600..=86_399 => trf("{n} h ago", &[("n", &n(secs / 3600))]),
        86_400..=172_799 => tr("yesterday").to_string(),
        _ => trf("{n} days ago", &[("n", &n(secs / 86_400))]),
    }
}
