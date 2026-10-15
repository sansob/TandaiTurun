//! TandaiTurun: a native Markdown reader and editor with the look of a PDF reader.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod chrome;
mod default_app;
mod dialogs;
mod editor;
mod fonts;
mod i18n;
mod i18n_id;
mod layout;
mod markdown;
mod math;
mod mermaid;
mod panes;
mod pdf;
mod remote;
mod scene;
mod theme;
mod view;

use std::path::PathBuf;

fn load_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("../../../assets/icon-256.png")).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData { rgba: img.into_raw(), width, height })
}

const HELP: &str = "TandaiTurun - read and edit Markdown like a PDF

Usage:
  tandaiturun [FILE.md ...]               open documents
  tandaiturun --export-pdf IN.md [OUT]    convert to PDF (A4, same layout as the app);
                                          add --allow-remote-images to include internet images
  tandaiturun --export-html IN.md [OUT]   convert to a standalone HTML file
  tandaiturun --version | --help
";

/// Command-line conversions that don't need a window. Returns the exit code
/// when the arguments asked for one.
fn run_cli(args: &[String]) -> Option<i32> {
    match args.first().map(String::as_str) {
        Some("--version" | "-V") => {
            println!("tandaiturun {}", env!("CARGO_PKG_VERSION"));
            Some(0)
        }
        Some("--help" | "-h") => {
            print!("{HELP}");
            Some(0)
        }
        Some("--export-pdf") => {
            let allow_remote = args.iter().any(|a| a == "--allow-remote-images");
            let args: Vec<String> = args.iter().filter(|a| *a != "--allow-remote-images").cloned().collect();
            let Some(input) = args.get(1) else {
                eprint!("{HELP}");
                return Some(2);
            };
            let input = PathBuf::from(input);
            let out = args.get(2).map(PathBuf::from).unwrap_or_else(|| input.with_extension("pdf"));
            let f = match tandaiturun_core::document::load(&input) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("{}: {e}", input.display());
                    return Some(1);
                }
            };
            let bytes = headless_pdf(&f.text, &input, allow_remote);
            match tandaiturun_core::document::write_atomic(&out, &bytes) {
                Ok(()) => {
                    println!("{}", out.display());
                    Some(0)
                }
                Err(e) => {
                    eprintln!("{}: {e}", out.display());
                    Some(1)
                }
            }
        }
        Some("--export-html") => {
            let Some(input) = args.get(1) else {
                eprint!("{HELP}");
                return Some(2);
            };
            let input = PathBuf::from(input);
            let out = args.get(2).map(PathBuf::from).unwrap_or_else(|| input.with_extension("html"));
            let code = match tandaiturun_core::document::load(&input) {
                Ok(f) => {
                    let title = tandaiturun_core::document::title(&f.text, Some(&input));
                    let html = tandaiturun_core::html::document(&f.text, &title);
                    match tandaiturun_core::document::write_atomic(&out, html.as_bytes()) {
                        Ok(()) => {
                            println!("{}", out.display());
                            0
                        }
                        Err(e) => {
                            eprintln!("{}: {e}", out.display());
                            1
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}: {e}", input.display());
                    1
                }
            };
            Some(code)
        }
        Some(a) if a.starts_with("--") => {
            eprintln!("unknown option {a}\n");
            eprint!("{HELP}");
            Some(2)
        }
        _ => None,
    }
}

/// Lays a document out without a window and renders it to PDF.
fn headless_pdf(text: &str, path: &std::path::Path, allow_remote: bool) -> Vec<u8> {
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    ctx.set_fonts(fonts::definitions());
    let look = view::Look {
        page: layout::PageSize::A4,
        font: layout::DocFont::Sans,
        font_size: app::Settings::default().font_size,
        theme: theme::PageTheme::Paper,
        allow_remote,
    };
    let base = std::fs::canonicalize(path).ok().and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let title = tandaiturun_core::document::title(text, Some(path));
    let mut out = Vec::new();
    // Images load in the background: lay out again until they are all there.
    let started = std::time::Instant::now();
    loop {
        let mut pending = false;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let params = layout::Params {
                page: look.page,
                font: look.font,
                font_size: look.font_size,
                theme: look.theme,
                zoom: 1.0,
                base_dir: base.clone(),
                pixels_per_point: ui.ctx().pixels_per_point(),
                allow_remote,
            };
            let l = layout::layout(ui.ctx(), text, &params);
            pending = l.pending_images;
            if !pending || started.elapsed() > std::time::Duration::from_secs(30) {
                let ctx = ui.ctx().clone();
                let fetch = move |uri: &str| match ctx.try_load_bytes(uri) {
                    Ok(egui::load::BytesPoll::Ready { bytes, .. }) => Some(bytes.to_vec()),
                    _ => None,
                };
                out = pdf::render(&l, text, &title, &fetch);
            }
        });
        output.textures_delta.clear();
        if !out.is_empty() {
            return out;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = run_cli(&args) {
        std::process::exit(code);
    }
    scene::init_from_env();
    let files: Vec<PathBuf> = args.iter().map(PathBuf::from).collect();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("TandaiTurun")
        .with_app_id("io.github.sansob.TandaiTurun")
        .with_inner_size([1360.0, 880.0])
        .with_min_inner_size([760.0, 520.0])
        .with_drag_and_drop(true);
    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }
    let scripted = scene::active();
    let options = eframe::NativeOptions {
        viewport,
        // Scripted screenshot scenes must not reuse the user's session.
        persist_window: !scripted,
        persistence_path: scripted.then(|| std::env::temp_dir().join("tandaiturun-scene.ron")),
        ..Default::default()
    };
    eframe::run_native(
        "TandaiTurun",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            cc.egui_ctx.set_fonts(fonts::definitions());
            Ok(Box::new(app::App::new(cc, files)))
        }),
    )
}
