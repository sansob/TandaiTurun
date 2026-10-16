//! The application: open documents, settings, and the frame-by-frame UI.

use crate::editor::{Editor, Format};
use crate::i18n::{self, tr, trf, Lang};
use crate::layout::{DocFont, PageSize};
use crate::theme::{self, PageTheme, Palette, UiTheme};
use crate::view::{DocView, Look, PageMode, ViewEvent, ZoomMode};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use tandaiturun_core::document::{self, LineEnding};
use tandaiturun_core::outline::Heading;
use tandaiturun_core::search::SearchOptions;

pub const WELCOME_EN: &str = include_str!("../../../examples/welcome.md");
pub const WELCOME_ID: &str = include_str!("../../../examples/welcome.id.md");
pub const CHEATSHEET: &str = include_str!("../../../examples/cheatsheet.md");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NavTab {
    Thumbnails,
    #[default]
    Bookmarks,
    Search,
}

/// Saved between runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub ui_theme: UiTheme,
    pub lang: Option<Lang>,
    pub page_theme: PageTheme,
    pub page_size: PageSize,
    pub doc_font: DocFont,
    pub font_size: f32,
    pub editor_font_size: f32,
    pub zoom: ZoomMode,
    pub page_mode: PageMode,
    pub show_nav: bool,
    pub nav_tab: NavTab,
    pub show_tools: bool,
    pub recent: Vec<PathBuf>,
    pub restore_session: bool,
    pub session: Vec<PathBuf>,
    pub auto_reload: bool,
    /// Load images from the internet without asking.
    pub remote_images: bool,
    /// Look for a newer release on GitHub at start-up.
    pub check_updates: bool,
    /// A release the reader chose to skip; not recommended again at start-up.
    pub skipped_version: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            ui_theme: UiTheme::System,
            lang: None,
            page_theme: PageTheme::Paper,
            page_size: PageSize::A4,
            doc_font: DocFont::Sans,
            font_size: 15.0,
            editor_font_size: 14.0,
            zoom: ZoomMode::FitWidth,
            page_mode: PageMode::Continuous,
            show_nav: true,
            nav_tab: NavTab::Bookmarks,
            show_tools: true,
            recent: Vec::new(),
            restore_session: true,
            session: Vec::new(),
            auto_reload: true,
            remote_images: false,
            check_updates: true,
            skipped_version: None,
        }
    }
}

impl Settings {
    pub fn look(&self) -> Look {
        Look {
            page: self.page_size,
            font: self.doc_font,
            font_size: self.font_size,
            theme: self.page_theme,
            allow_remote: self.remote_images,
        }
    }

    pub fn add_recent(&mut self, path: &Path) {
        let p = path.to_path_buf();
        self.recent.retain(|r| r != &p);
        self.recent.insert(0, p);
        self.recent.truncate(15);
    }
}

pub struct Doc {
    pub uid: u64,
    pub path: Option<PathBuf>,
    /// Name for documents that were never saved.
    pub untitled: String,
    pub text: String,
    pub saved: String,
    pub rev: u64,
    pub line_ending: LineEnding,
    pub bom: bool,
    pub mtime: Option<SystemTime>,
    pub disk_changed: bool,
    pub editing: bool,
    pub view: DocView,
    pub preview: DocView,
    pub editor: Editor,
    /// The text the preview shows (updated shortly after typing stops).
    pub preview_text: String,
    pub preview_rev: u64,
    pub last_edit: Instant,
    outline: (u64, Vec<Heading>),
    pub page_input: String,
    pub page_input_focus: bool,
    /// Internet images were allowed for this document.
    pub allow_remote: bool,
    /// The revision last checked for characters needing a system font.
    fonts_rev: u64,
    /// The file was deleted or moved away on disk.
    pub disk_missing: bool,
}

impl Doc {
    pub fn new(uid: u64, path: Option<PathBuf>, text: String, settings: &Settings) -> Doc {
        let view = DocView::new(settings.zoom, settings.page_mode);
        let preview = DocView::new(ZoomMode::FitWidth, PageMode::Continuous);
        Doc {
            uid,
            path,
            untitled: String::new(),
            saved: text.clone(),
            preview_text: text.clone(),
            text,
            rev: 1,
            line_ending: LineEnding::Lf,
            bom: false,
            mtime: None,
            disk_changed: false,
            editing: false,
            view,
            preview,
            editor: Editor::default(),
            preview_rev: 1,
            fonts_rev: 0,
            disk_missing: false,
            last_edit: Instant::now(),
            outline: (0, Vec::new()),
            page_input: "1".into(),
            page_input_focus: false,
            allow_remote: false,
        }
    }

    pub fn dirty(&self) -> bool {
        self.text != self.saved
    }

    pub fn name(&self) -> String {
        match &self.path {
            Some(p) => p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            None => self.untitled.clone(),
        }
    }

    pub fn base_dir(&self) -> Option<PathBuf> {
        self.path.as_ref().and_then(|p| p.parent()).map(Path::to_path_buf)
    }

    pub fn outline(&mut self) -> &[Heading] {
        let text = if self.editing { &self.preview_text } else { &self.text };
        let rev = if self.editing { self.preview_rev } else { self.rev };
        if self.outline.0 != rev {
            self.outline = (rev, tandaiturun_core::outline::outline(text));
        }
        &self.outline.1
    }

    /// The view the toolbar and the side panes act on.
    pub fn active_view(&mut self) -> &mut DocView {
        if self.editing {
            &mut self.preview
        } else {
            &mut self.view
        }
    }

    pub fn active_view_ref(&self) -> &DocView {
        if self.editing {
            &self.preview
        } else {
            &self.view
        }
    }

    pub fn changed(&mut self) {
        self.rev += 1;
        self.last_edit = Instant::now();
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    New,
    Open,
    OpenPath(PathBuf),
    OpenText {
        title: String,
        text: String,
    },
    Save,
    SaveAs,
    Reload,
    ExportPdf,
    ExportHtml,
    Print,
    Close(u64),
    /// Close every tab but this one.
    CloseOthers(u64),
    /// Close the tabs after this one.
    CloseRight(u64),
    CloseAll,
    Quit,
    ToggleEdit,
    EditAt(usize),
    Find,
    Format(Format),
    Undo,
    Redo,
    MakeDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AboutTab {
    About,
    Libraries,
    Fonts,
}

pub enum Dialog {
    /// Unsaved changes in these tabs; `quit` closes the app afterwards.
    ConfirmClose {
        uids: Vec<u64>,
        quit: bool,
    },
    Properties,
    Preferences,
    Shortcuts,
    About {
        tab: AboutTab,
        filter: String,
    },
    Error(String),
    /// A newer release is out.
    Update(crate::update::Release),
}

pub struct Toast {
    pub text: String,
    pub at: Instant,
}

pub struct App {
    pub settings: Settings,
    pub docs: Vec<Doc>,
    /// `None` is the Home tab.
    pub active: Option<usize>,
    pub next_uid: u64,
    pub untitled_count: usize,
    pub palette: Palette,
    applied: Option<(bool, Lang)>,
    pub dialog: Option<Dialog>,
    pub actions: Vec<Action>,
    pub toasts: Vec<Toast>,
    pub find_open: bool,
    pub find_focus: bool,
    pub find_query: String,
    pub replace_text: String,
    pub show_replace: bool,
    pub find_opts: SearchOptions,
    pub read_mode: bool,
    pub fullscreen: bool,
    pub allow_close: bool,
    last_poll: Instant,
    pub inject_keys: Vec<egui::Event>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, files: Vec<PathBuf>) -> App {
        let mut settings: Settings = cc.storage.and_then(|s| eframe::get_value(s, "settings")).unwrap_or_default();
        // 0.1.0 saved Windows paths with the `\\?\` prefix.
        for list in [&mut settings.recent, &mut settings.session] {
            for p in list.iter_mut() {
                *p = document::simplify(std::mem::take(p));
            }
            let mut seen = std::collections::HashSet::new();
            list.retain(|p| seen.insert(p.clone()));
        }
        i18n::set(settings.lang.unwrap_or_else(Lang::from_system));
        cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let mut app = App {
            palette: Palette::of(false),
            settings,
            docs: Vec::new(),
            active: None,
            next_uid: 1,
            untitled_count: 0,
            applied: None,
            dialog: None,
            actions: Vec::new(),
            toasts: Vec::new(),
            find_open: false,
            find_focus: false,
            find_query: String::new(),
            replace_text: String::new(),
            show_replace: false,
            find_opts: SearchOptions::default(),
            read_mode: false,
            fullscreen: false,
            allow_close: false,
            last_poll: Instant::now(),
            inject_keys: Vec::new(),
        };
        let to_open = if files.is_empty() && app.settings.restore_session {
            app.settings.session.iter().filter(|p| p.is_file()).cloned().collect()
        } else {
            files
        };
        for f in to_open {
            app.open_path(&f);
        }
        // Scripted runs (tests, screenshots) stay offline and undisturbed.
        let scripted =
            std::env::var_os("TANDAITURUN_SCRIPT").is_some() || std::env::var_os("TANDAITURUN_SCENE").is_some();
        if app.settings.check_updates && !scripted {
            crate::update::start(&cc.egui_ctx, false);
        }
        app
    }

    pub fn look(&self) -> Look {
        self.settings.look()
    }

    /// Shows the result of an update check once it is in. A check at start-up
    /// only speaks up for a release the reader hasn't skipped.
    fn poll_update(&mut self) {
        if self.dialog.is_some() {
            return;
        }
        let Some((outcome, manual)) = crate::update::take() else { return };
        match outcome {
            crate::update::Outcome::Newer(r) => {
                if manual || self.settings.skipped_version.as_deref() != Some(r.version.as_str()) {
                    self.dialog = Some(Dialog::Update(r));
                }
            }
            crate::update::Outcome::UpToDate if manual => self.toast(tr("TandaiTurun is up to date.")),
            crate::update::Outcome::Failed(e) if manual => {
                self.dialog = Some(Dialog::Error(trf("Couldn't check for updates: {error}", &[("error", &e)])))
            }
            _ => {}
        }
    }

    pub fn toast(&mut self, text: impl Into<String>) {
        self.toasts.push(Toast { text: text.into(), at: Instant::now() });
    }

    pub fn doc(&self) -> Option<&Doc> {
        self.active.and_then(|i| self.docs.get(i))
    }

    pub fn doc_mut(&mut self) -> Option<&mut Doc> {
        self.active.and_then(|i| self.docs.get_mut(i))
    }

    fn add_doc(&mut self, path: Option<PathBuf>, text: String) -> usize {
        let uid = self.next_uid;
        self.next_uid += 1;
        let doc = Doc::new(uid, path, text, &self.settings);
        self.docs.push(doc);
        let i = self.docs.len() - 1;
        self.active = Some(i);
        i
    }

    pub fn new_doc(&mut self) {
        self.untitled_count += 1;
        let i = self.add_doc(None, String::new());
        self.docs[i].untitled = format!("{}-{}.md", tr("Untitled"), self.untitled_count);
        self.docs[i].editing = true;
        self.docs[i].editor.focus_next = true;
    }

    pub fn open_text(&mut self, title: &str, text: &str) {
        let i = self.add_doc(None, text.to_string());
        self.docs[i].untitled = title.to_string();
    }

    pub fn open_path(&mut self, path: &Path) {
        let path = document::canonical(path);
        if let Some(i) = self.docs.iter().position(|d| d.path.as_deref() == Some(&path)) {
            self.active = Some(i);
            return;
        }
        match document::load(&path) {
            Ok(f) => {
                let i = self.add_doc(Some(path.clone()), f.text);
                let d = &mut self.docs[i];
                d.line_ending = f.line_ending;
                d.bom = f.bom;
                d.mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                self.settings.add_recent(&path);
            }
            Err(e) => {
                self.settings.recent.retain(|r| r != &path);
                self.dialog = Some(Dialog::Error(trf(
                    "Could not open {file}: {error}",
                    &[("file", &path.display().to_string()), ("error", &e.to_string())],
                )));
            }
        }
    }

    fn pick_open(&mut self) {
        let dir = self.doc().and_then(|d| d.base_dir());
        let mut dlg = rfd::FileDialog::new()
            .add_filter(tr("Markdown documents"), document::EXTENSIONS)
            .add_filter(tr("All files"), &["*"]);
        if let Some(d) = dir {
            dlg = dlg.set_directory(d);
        }
        if let Some(files) = dlg.pick_files() {
            for f in files {
                self.open_path(&f);
            }
        }
    }

    /// Saves the active document; asks for a name when it has none.
    pub fn save(&mut self, i: usize, save_as: bool) -> bool {
        let path = match (&self.docs[i].path, save_as) {
            (Some(p), false) => p.clone(),
            _ => {
                let d = &self.docs[i];
                let name = if d.path.is_some() {
                    d.name()
                } else {
                    let t = document::title(&d.text, None);
                    if t == "Untitled" || t.is_empty() {
                        d.untitled.clone()
                    } else {
                        format!("{}.md", t.chars().filter(|c| !"/\\:*?\"<>|".contains(*c)).collect::<String>())
                    }
                };
                let mut dlg = rfd::FileDialog::new()
                    .add_filter(tr("Markdown documents"), &["md", "markdown"])
                    .set_file_name(&name);
                if let Some(dir) = d.base_dir() {
                    dlg = dlg.set_directory(dir);
                }
                match dlg.save_file() {
                    Some(p) => document::with_md_extension(p),
                    None => return false,
                }
            }
        };
        let d = &mut self.docs[i];
        match document::save(&path, &d.text, d.line_ending, d.bom) {
            Ok(()) => {
                d.saved = d.text.clone();
                d.path = Some(path.clone());
                d.mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                d.disk_changed = false;
                let name = d.name();
                self.settings.add_recent(&path);
                self.toast(trf("Saved {file}", &[("file", &name)]));
                true
            }
            Err(e) => {
                self.dialog = Some(Dialog::Error(trf(
                    "Could not save {file}: {error}",
                    &[("file", &path.display().to_string()), ("error", &e.to_string())],
                )));
                false
            }
        }
    }

    pub fn reload(&mut self, i: usize) {
        let Some(path) = self.docs[i].path.clone() else { return };
        match document::load(&path) {
            Ok(f) => {
                let d = &mut self.docs[i];
                d.text = f.text.clone();
                d.saved = f.text.clone();
                d.preview_text = f.text;
                d.editor.reset(&d.text);
                d.line_ending = f.line_ending;
                d.bom = f.bom;
                d.mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                d.disk_changed = false;
                d.changed();
                d.preview_rev = d.rev;
            }
            Err(e) => self.dialog = Some(Dialog::Error(e.to_string())),
        }
    }

    fn close(&mut self, uid: u64, force: bool) {
        let Some(i) = self.docs.iter().position(|d| d.uid == uid) else { return };
        if !force && self.docs[i].dirty() {
            self.dialog = Some(Dialog::ConfirmClose { uids: vec![uid], quit: false });
            return;
        }
        self.docs.remove(i);
        self.active = match self.active {
            _ if self.docs.is_empty() => None,
            Some(a) if a > i => Some(a - 1),
            Some(a) if a == i => Some(i.min(self.docs.len() - 1)),
            other => other,
        };
    }

    pub fn close_uids(&mut self, uids: &[u64]) {
        for uid in uids {
            self.close(*uid, true);
        }
    }

    /// Closes the tabs `which` picks; unsaved ones are asked about first.
    fn close_many(&mut self, which: impl Fn(usize, &Doc) -> bool) {
        let picked: Vec<(u64, bool)> =
            self.docs.iter().enumerate().filter(|(i, d)| which(*i, d)).map(|(_, d)| (d.uid, d.dirty())).collect();
        let clean: Vec<u64> = picked.iter().filter(|(_, dirty)| !dirty).map(|(u, _)| *u).collect();
        let dirty: Vec<u64> = picked.iter().filter(|(_, dirty)| *dirty).map(|(u, _)| *u).collect();
        self.close_uids(&clean);
        if !dirty.is_empty() {
            self.dialog = Some(Dialog::ConfirmClose { uids: dirty, quit: false });
        }
    }

    fn export_html(&mut self, i: usize) {
        let d = &self.docs[i];
        let title = document::title(&d.text, d.path.as_deref());
        let name =
            d.path.as_ref().and_then(|p| p.file_stem()).map_or(title.clone(), |s| s.to_string_lossy().into_owned());
        let mut dlg = rfd::FileDialog::new().add_filter("HTML", &["html", "htm"]).set_file_name(format!("{name}.html"));
        if let Some(dir) = d.base_dir() {
            dlg = dlg.set_directory(dir);
        }
        if let Some(p) = dlg.save_file() {
            let html = crate::html_export::document(&d.text, &title, d.base_dir());
            match document::write_atomic(&p, html.as_bytes()) {
                Ok(()) => self.toast(trf("Exported {file}", &[("file", &p.display().to_string())])),
                Err(e) => self.dialog = Some(Dialog::Error(e.to_string())),
            }
        }
    }

    fn pdf_bytes(&self, ctx: &egui::Context, i: usize) -> Vec<u8> {
        let d = &self.docs[i];
        let title = document::title(&d.text, d.path.as_deref());
        let mut look = self.look();
        look.allow_remote |= d.allow_remote;
        crate::pdf::export(ctx, &d.text, &look, d.base_dir(), &title)
    }

    fn export_pdf(&mut self, ctx: &egui::Context, i: usize) {
        let d = &self.docs[i];
        let name = d
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map_or_else(|| document::title(&d.text, None), |s| s.to_string_lossy().into_owned());
        let mut dlg = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_file_name(format!("{name}.pdf"));
        if let Some(dir) = d.base_dir() {
            dlg = dlg.set_directory(dir);
        }
        if let Some(p) = dlg.save_file() {
            let bytes = self.pdf_bytes(ctx, i);
            match document::write_atomic(&p, &bytes) {
                Ok(()) => self.toast(trf("Exported {file}", &[("file", &p.display().to_string())])),
                Err(e) => self.dialog = Some(Dialog::Error(e.to_string())),
            }
        }
    }

    /// Prints by handing a PDF of the document to the system's PDF viewer.
    fn print(&mut self, ctx: &egui::Context, i: usize) {
        let bytes = self.pdf_bytes(ctx, i);
        let dir = std::env::temp_dir().join("tandaiturun-print");
        let _ = std::fs::create_dir_all(&dir);
        let stem = self.docs[i].name().trim_end_matches(".md").to_string();
        let stem: String =
            stem.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
        let path = dir.join(format!("{stem}.pdf"));
        match document::write_atomic(&path, &bytes)
            .map_err(|e| e.to_string())
            .and_then(|_| open::that_detached(&path).map_err(|e| e.to_string()))
        {
            Ok(()) => self.toast(tr("Opened in your PDF viewer: print from there")),
            Err(e) => self.dialog = Some(Dialog::Error(e)),
        }
    }

    pub fn open_link(&mut self, url: &str) {
        let base = self.doc().and_then(|d| d.base_dir());
        if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:") {
            if let Err(e) = webbrowser::open(url) {
                self.dialog = Some(Dialog::Error(e.to_string()));
            }
            return;
        }
        // A relative link to another document: open it in a new tab.
        let (file, anchor) = url.split_once('#').map_or((url, None), |(f, a)| (f, Some(a)));
        let path = Path::new(file);
        let full =
            if path.is_absolute() { path.to_path_buf() } else { base.map_or(path.to_path_buf(), |b| b.join(path)) };
        if full.is_file() {
            if document::is_markdown_path(&full) {
                self.open_path(&full);
                if let (Some(a), Some(d)) = (anchor, self.doc_mut()) {
                    let a = a.to_string();
                    d.view.go_to_anchor(&a);
                }
            } else if let Err(e) = open::that_detached(&full) {
                self.dialog = Some(Dialog::Error(e.to_string()));
            }
        } else {
            self.toast(trf("Link target not found: {target}", &[("target", url)]));
        }
    }

    pub fn toggle_edit(&mut self) {
        if let Some(d) = self.doc_mut() {
            d.editing = !d.editing;
            if d.editing {
                d.preview_text = d.text.clone();
                d.preview_rev = d.rev;
                let y = d.view.top_flow;
                let offset = d.view.layout.as_ref().and_then(|l| l.offset_of_y(y)).unwrap_or(0);
                d.editor.select(offset..offset);
                d.editor.focus_next = true;
                d.preview.go_to_offset(offset);
            } else {
                let at = d.editor.selection(&d.text).start;
                d.view.go_to_offset(at);
            }
        }
    }

    fn apply_actions(&mut self, ctx: &egui::Context) {
        let actions = std::mem::take(&mut self.actions);
        for a in actions {
            match a {
                Action::New => self.new_doc(),
                Action::Open => self.pick_open(),
                Action::OpenPath(p) => self.open_path(&p),
                Action::OpenText { title, text } => self.open_text(&title, &text),
                Action::Save => {
                    if let Some(i) = self.active {
                        self.save(i, false);
                    }
                }
                Action::SaveAs => {
                    if let Some(i) = self.active {
                        self.save(i, true);
                    }
                }
                Action::Reload => {
                    if let Some(i) = self.active {
                        self.reload(i);
                    }
                }
                Action::ExportPdf => {
                    if let Some(i) = self.active {
                        self.export_pdf(ctx, i);
                    }
                }
                Action::ExportHtml => {
                    if let Some(i) = self.active {
                        self.export_html(i);
                    }
                }
                Action::Print => {
                    if let Some(i) = self.active {
                        self.print(ctx, i);
                    }
                }
                Action::Close(uid) => self.close(uid, false),
                Action::CloseAll => self.close_many(|_, _| true),
                Action::CloseOthers(uid) => self.close_many(|_, d| d.uid != uid),
                Action::CloseRight(uid) => {
                    let at = self.docs.iter().position(|d| d.uid == uid).unwrap_or(usize::MAX);
                    self.close_many(|i, _| i > at);
                }
                Action::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Action::ToggleEdit => self.toggle_edit(),
                Action::EditAt(offset) => {
                    if let Some(d) = self.doc_mut() {
                        if !d.editing {
                            d.editing = true;
                            d.preview_text = d.text.clone();
                            d.preview_rev = d.rev;
                        }
                        d.editor.select(offset..offset);
                        d.editor.focus_next = true;
                        d.preview.go_to_offset(offset);
                    }
                }
                Action::Find => {
                    self.find_open = true;
                    self.find_focus = true;
                    if let Some(sel) = self.doc().and_then(|d| {
                        if d.editing {
                            Some(d.text[d.editor.selection(&d.text)].to_string())
                        } else {
                            d.view.selected_text()
                        }
                    }) {
                        if !sel.is_empty() && !sel.contains('\n') {
                            self.find_query = sel;
                        }
                    }
                }
                Action::Format(f) => {
                    if let Some(d) = self.doc_mut() {
                        if d.editing {
                            d.editor.format(&mut d.text, f);
                            d.changed();
                        }
                    }
                }
                Action::MakeDefault => match crate::default_app::make_default() {
                    Ok(crate::default_app::Outcome::Done) => {
                        self.toast(tr("TandaiTurun now opens your Markdown files"));
                    }
                    Ok(crate::default_app::Outcome::OpenedSettings) => {
                        self.toast(tr("Pick TandaiTurun for .md files in the settings that just opened"));
                    }
                    Err(e) => self.dialog = Some(Dialog::Error(e)),
                },
                Action::Undo | Action::Redo => {
                    if let Some(d) = self.doc_mut() {
                        if d.editing {
                            d.editor.focus_next = true;
                            // What a real Ctrl+Z (⌘Z on macOS) looks like.
                            let mut modifiers = if cfg!(target_os = "macos") {
                                egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND
                            } else {
                                egui::Modifiers::CTRL | egui::Modifiers::COMMAND
                            };
                            modifiers.shift = matches!(a, Action::Redo);
                            self.inject_keys.extend([
                                egui::Event::ModifiersChanged(modifiers),
                                egui::Event::Key {
                                    key: egui::Key::Z,
                                    physical_key: None,
                                    pressed: true,
                                    repeat: false,
                                    modifiers,
                                },
                                egui::Event::ModifiersChanged(egui::Modifiers::NONE),
                            ]);
                            ctx.request_repaint();
                        }
                    }
                }
            }
        }
    }

    /// Reloads documents changed on disk (or flags them when they have unsaved edits).
    fn poll_files(&mut self) {
        if self.last_poll.elapsed() < Duration::from_millis(1000) {
            return;
        }
        self.last_poll = Instant::now();
        let mut reload = Vec::new();
        for (i, d) in self.docs.iter_mut().enumerate() {
            let Some(p) = &d.path else { continue };
            let meta = std::fs::metadata(p);
            d.disk_missing = meta.as_ref().is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound);
            let Ok(m) = meta.and_then(|m| m.modified()) else { continue };
            if d.mtime.is_some_and(|old| old != m) {
                d.mtime = Some(m);
                if !d.dirty() && self.settings.auto_reload {
                    reload.push(i);
                } else {
                    d.disk_changed = true;
                }
            }
        }
        for i in reload {
            self.reload(i);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers as M};
        let cmd = M::COMMAND;
        let cs = M::COMMAND | M::SHIFT;
        let look = self.look();
        let editing = self.doc().is_some_and(|d| d.editing);
        let mut act = Vec::new();
        let mut zoom: Option<i32> = None;
        ctx.input_mut(|i| {
            if i.consume_key(cs, Key::S) {
                act.push(Action::SaveAs);
            } else if i.consume_key(cmd, Key::S) {
                act.push(Action::Save);
            }
            if i.consume_key(cmd, Key::N) {
                act.push(Action::New);
            }
            if i.consume_key(cmd, Key::O) {
                act.push(Action::Open);
            }
            if i.consume_key(cmd, Key::W) {
                if let Some(d) = self.active.and_then(|a| self.docs.get(a)) {
                    act.push(Action::Close(d.uid));
                }
            }
            if i.consume_key(cmd, Key::Q) {
                act.push(Action::Quit);
            }
            if i.consume_key(cmd, Key::P) {
                act.push(Action::Print);
            }
            if i.consume_key(cmd, Key::E) {
                act.push(Action::ToggleEdit);
            }
            if i.consume_key(cmd, Key::F) {
                act.push(Action::Find);
            }
            if i.consume_key(cmd, Key::D) {
                self.dialog = Some(Dialog::Properties);
            }
            if i.consume_key(cmd, Key::Comma) {
                self.dialog = Some(Dialog::Preferences);
            }
            if i.consume_key(cmd, Key::H) {
                self.read_mode = !self.read_mode;
            }
            if i.consume_key(M::NONE, Key::F5) {
                act.push(Action::Reload);
            }
            if i.consume_key(M::SHIFT, Key::F4) {
                self.settings.show_tools = !self.settings.show_tools;
            } else if i.consume_key(M::NONE, Key::F4) {
                self.settings.show_nav = !self.settings.show_nav;
            }
            if i.consume_key(M::NONE, Key::F11) {
                self.fullscreen = !self.fullscreen;
            }
            if i.consume_key(M::SHIFT, Key::F3) {
                zoom = Some(100);
            } else if i.consume_key(M::NONE, Key::F3) {
                zoom = Some(101);
            }
            if i.consume_key(cs, Key::Tab) {
                zoom = Some(200);
            } else if i.consume_key(M::CTRL, Key::Tab) {
                zoom = Some(201);
            }
            if i.consume_key(cmd, Key::Plus) || i.consume_key(cmd, Key::Equals) {
                zoom = Some(1);
            }
            if i.consume_key(cmd, Key::Minus) {
                zoom = Some(-1);
            }
            // Ctrl+Alt+digit is a heading in the editor, not a zoom.
            let alt = i.modifiers.alt;
            if !alt && i.consume_key(cmd, Key::Num0) {
                zoom = Some(10);
            }
            if !alt && i.consume_key(cmd, Key::Num1) {
                zoom = Some(11);
            }
            if !alt && i.consume_key(cmd, Key::Num2) {
                zoom = Some(12);
            }
            if i.consume_key(M::NONE, Key::Escape) {
                if self.read_mode {
                    self.read_mode = false;
                } else if self.fullscreen {
                    self.fullscreen = false;
                } else if self.find_open {
                    self.find_open = false;
                }
            }
        });
        self.actions.extend(act);
        if let Some(z) = zoom {
            match z {
                100 | 101 => self.find_step(z == 101),
                200 | 201 => {
                    let n = self.docs.len() + 1;
                    let cur = self.active.map_or(0, |a| a + 1);
                    let next = if z == 201 { (cur + 1) % n } else { (cur + n - 1) % n };
                    self.active = next.checked_sub(1);
                }
                _ => {
                    if let Some(d) = self.doc_mut() {
                        let v = d.active_view();
                        match z {
                            1 => v.zoom_in(&look),
                            -1 => v.zoom_out(&look),
                            10 => v.set_zoom(ZoomMode::FitPage),
                            11 => v.set_zoom(ZoomMode::Fixed(1.0)),
                            12 => v.set_zoom(ZoomMode::FitWidth),
                            _ => {}
                        }
                        if !editing {
                            self.settings.zoom = d.view.zoom;
                        }
                    }
                }
            }
        }
    }

    /// Next / previous search hit, in the reader or in the source.
    pub fn find_step(&mut self, forward: bool) {
        let Some(d) = self.doc_mut() else { return };
        if d.editing {
            let n = d.editor.marks.len();
            if n == 0 {
                return;
            }
            let next = match d.editor.current_mark {
                None => 0,
                Some(c) if forward => (c + 1) % n,
                Some(c) => (c + n - 1) % n,
            };
            d.editor.current_mark = Some(next);
            let r = d.editor.marks[next].clone();
            d.editor.select(r);
        } else {
            d.view.search_step(forward);
        }
    }

    /// Keeps search results in step with the find field.
    fn update_search(&mut self) {
        let query = if self.find_open || self.settings.nav_tab == NavTab::Search {
            self.find_query.clone()
        } else {
            String::new()
        };
        let opts = self.find_opts;
        let Some(d) = self.doc_mut() else { return };
        if d.editing {
            let marks = tandaiturun_core::search::find_all(&d.text, &query, opts);
            if marks != d.editor.marks {
                d.editor.current_mark = if marks.is_empty() {
                    None
                } else {
                    let at = d.editor.selection(&d.text).start;
                    Some(marks.iter().position(|m| m.start >= at).unwrap_or(0))
                };
                d.editor.marks = marks;
            }
            d.preview.search.query.clear();
            d.preview.refresh_search();
        } else {
            d.view.search.query = query;
            d.view.search.opts = opts;
            d.view.refresh_search();
            d.editor.marks.clear();
        }
    }

    pub fn replace_current(&mut self) {
        let replacement = self.replace_text.clone();
        let Some(d) = self.doc_mut() else { return };
        if !d.editing {
            return;
        }
        let Some(r) = d.editor.current_mark.and_then(|c| d.editor.marks.get(c).cloned()) else { return };
        let mut t = d.text.clone();
        t.replace_range(r.clone(), &replacement);
        let at = r.start + replacement.len();
        d.editor.apply(&mut d.text, tandaiturun_core::format::Edit { text: t, selection: at..at });
        d.changed();
        d.editor.marks.clear();
        d.editor.current_mark = None;
        // The recount already makes the first match after the replacement
        // current: select it rather than stepping past it.
        self.update_search();
        if let Some(d) = self.doc_mut() {
            if let Some(r) = d.editor.current_mark.and_then(|c| d.editor.marks.get(c).cloned()) {
                d.editor.select(r);
            }
        }
    }

    pub fn replace_all(&mut self) {
        let (query, replacement, opts) = (self.find_query.clone(), self.replace_text.clone(), self.find_opts);
        let Some(d) = self.doc_mut() else { return };
        if !d.editing || query.is_empty() {
            return;
        }
        let (t, n) = tandaiturun_core::search::replace_all(&d.text, &query, &replacement, opts);
        if n > 0 {
            let at = crate::editor::snap(&t, d.editor.cursor.start..d.editor.cursor.start).start;
            d.editor.apply(&mut d.text, tandaiturun_core::format::Edit { text: t, selection: at..at });
            d.changed();
        }
        self.toast(trf("Replaced {n} occurrences", &[("n", &n.to_string())]));
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            let dirty: Vec<u64> = self.docs.iter().filter(|d| d.dirty()).map(|d| d.uid).collect();
            if !dirty.is_empty() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.dialog = Some(Dialog::ConfirmClose { uids: dirty, quit: true });
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect()
        });
        for p in dropped {
            self.open_path(&p);
        }
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        if hovering {
            let screen = ctx.content_rect();
            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop")));
            painter.rect_filled(screen, 0.0, egui::Color32::from_black_alpha(140));
            painter.text(
                screen.center(),
                egui::Align2::CENTER_CENTER,
                tr("Drop Markdown files to open them"),
                egui::FontId::proportional(22.0),
                egui::Color32::WHITE,
            );
        }
    }

    /// Lays out the visible views for this frame.
    fn update_layouts(&mut self, ctx: &egui::Context) {
        let mut look = self.look();
        let Some(d) = self.active.and_then(|i| self.docs.get_mut(i)) else { return };
        look.allow_remote |= d.allow_remote;
        let base = d.base_dir();
        if d.editing {
            // Lay the preview out again shortly after typing stops (at once for short texts).
            if d.preview_rev != d.rev {
                let quiet = d.last_edit.elapsed() > Duration::from_millis(300) || d.text.len() < 20_000;
                if quiet {
                    d.preview_text = d.text.clone();
                    d.preview_rev = d.rev;
                } else {
                    ctx.request_repaint_after(Duration::from_millis(320));
                }
            }
            d.preview.update_layout(ctx, &d.preview_text, d.preview_rev, &look, base);
        } else {
            d.view.update_layout(ctx, &d.text, d.rev, &look, base);
        }
    }

    pub fn handle_view_events(&mut self, events: Vec<ViewEvent>) {
        for e in events {
            match e {
                ViewEvent::Link(url) => self.open_link(&url),
                ViewEvent::EditAt(offset) => self.actions.push(Action::EditAt(offset)),
                ViewEvent::AllowRemote => {
                    if let Some(d) = self.doc_mut() {
                        d.allow_remote = true;
                    }
                }
            }
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_files();
        ctx.request_repaint_after(Duration::from_millis(1000));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // Scripts the bundled fonts lack: borrow fonts from the system.
        let mut new_fonts = false;
        for d in &mut self.docs {
            if d.fonts_rev != d.rev {
                d.fonts_rev = d.rev;
                new_fonts |= crate::fonts::cover(&d.text);
            }
        }
        if new_fonts {
            ctx.set_fonts(crate::fonts::definitions());
            ctx.request_repaint();
        }
        let dark = match self.settings.ui_theme {
            UiTheme::Light => false,
            UiTheme::Dark => true,
            UiTheme::System => ctx.system_theme().is_some_and(|t| t == egui::Theme::Dark),
        };
        let lang = self.settings.lang.unwrap_or_else(Lang::from_system);
        if self.applied != Some((dark, lang)) {
            self.palette = Palette::of(dark);
            theme::apply(&ctx, &self.palette);
            i18n::set(lang);
            self.applied = Some((dark, lang));
        }
        if self.dialog.is_none() {
            self.handle_shortcuts(&ctx);
        }
        self.handle_close_request(&ctx);
        self.update_layouts(&ctx);
        self.update_search();
        // Page keys go to the pages unless a text field (editor, find box,
        // page number) is being typed in.
        let typing = ctx.text_edit_focused();
        let dialog_open = self.dialog.is_some();
        if let Some(d) = self.doc_mut() {
            if !typing && !dialog_open {
                d.active_view().handle_keys(&ctx);
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen || self.read_mode && false));

        if self.read_mode && self.doc().is_some() {
            crate::chrome::read_mode(self, ui);
        } else {
            crate::chrome::menu_bar(self, ui);
            crate::chrome::tab_strip(self, ui);
            if self.doc().is_some() {
                crate::chrome::toolbar(self, ui);
                if self.doc().is_some_and(|d| d.editing) {
                    crate::chrome::format_bar(self, ui);
                }
                if self.find_open {
                    crate::chrome::find_bar(self, ui);
                }
                crate::chrome::disk_banner(self, ui);
                crate::chrome::missing_banner(self, ui);
                crate::chrome::remote_banner(self, ui);
                crate::chrome::status_bar(self, ui);
                crate::panes::nav_rail(self, ui);
                if self.settings.show_nav {
                    crate::panes::nav_pane(self, ui);
                }
                if self.settings.show_tools {
                    crate::panes::tools_pane(self, ui);
                }
                crate::panes::document(self, ui);
            } else {
                crate::panes::home(self, ui);
            }
        }
        crate::dialogs::show(self, &ctx);
        crate::dialogs::toasts(self, &ctx);
        self.handle_dropped_files(&ctx);
        self.apply_actions(&ctx);
        self.poll_update();
        let title = match self.doc() {
            Some(d) => format!("{}{} - TandaiTurun", if d.dirty() { "● " } else { "" }, d.name()),
            None => "TandaiTurun".to_string(),
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        crate::scene::tick(self, &ctx);
        crate::script::tick(self, &ctx);
    }

    /// Events the app sends itself (menu Undo/Redo, scripted scenes), fed in
    /// as real input so egui handles them like the user's own.
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        raw_input.events.append(&mut self.inject_keys);
    }

    /// Widget state (scroll positions…) is keyed by tab numbers, which mean
    /// other documents in the next session: don't keep it.
    fn persist_egui_memory(&self) -> bool {
        false
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.settings.session = self.docs.iter().filter_map(|d| d.path.clone()).collect();
        if crate::scene::active() || crate::script::active() {
            return;
        }
        eframe::set_value(storage, "settings", &self.settings);
    }
}
