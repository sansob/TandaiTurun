//! Colours and the egui style. The window chrome follows the Acrobat layout:
//! white bars around a grey canvas, with the document pages as white paper.

use egui::epaint::Shadow;
use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Visuals};
use serde::{Deserialize, Serialize};

/// The TandaiTurun red (the brand mark and a few highlights).
pub const BRAND: Color32 = Color32::from_rgb(0xE0, 0x30, 0x2A);
/// Selection and focus blue.
pub const ACCENT: Color32 = Color32::from_rgb(0x14, 0x73, 0xE6);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UiTheme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub dark: bool,
    /// Menu bar and toolbar.
    pub chrome: Color32,
    /// The tab strip behind the document tabs.
    pub strip: Color32,
    /// Behind the pages.
    pub canvas: Color32,
    /// Side panes.
    pub pane: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub hover: Color32,
    pub selected: Color32,
    pub input: Color32,
}

impl Palette {
    pub fn of(dark: bool) -> Palette {
        if dark {
            Palette {
                dark,
                chrome: Color32::from_rgb(0x2B, 0x2B, 0x2B),
                strip: Color32::from_rgb(0x1F, 0x1F, 0x1F),
                canvas: Color32::from_rgb(0x3A, 0x3A, 0x3A),
                pane: Color32::from_rgb(0x26, 0x26, 0x26),
                border: Color32::from_rgb(0x44, 0x44, 0x44),
                text: Color32::from_rgb(0xE6, 0xE6, 0xE6),
                muted: Color32::from_rgb(0xA0, 0xA0, 0xA0),
                hover: Color32::from_rgb(0x3A, 0x3A, 0x3A),
                selected: Color32::from_rgb(0x1D, 0x3F, 0x66),
                input: Color32::from_rgb(0x1E, 0x1E, 0x1E),
            }
        } else {
            Palette {
                dark,
                chrome: Color32::WHITE,
                strip: Color32::from_rgb(0xF0, 0xF0, 0xF0),
                canvas: Color32::from_rgb(0xD9, 0xD9, 0xD9),
                pane: Color32::from_rgb(0xF7, 0xF7, 0xF7),
                border: Color32::from_rgb(0xD6, 0xD6, 0xD6),
                text: Color32::from_rgb(0x2C, 0x2C, 0x2C),
                muted: Color32::from_rgb(0x6E, 0x6E, 0x6E),
                hover: Color32::from_rgb(0xEA, 0xEA, 0xEA),
                selected: Color32::from_rgb(0xDD, 0xEB, 0xFB),
                input: Color32::WHITE,
            }
        }
    }
}

/// Colours of the paper itself ("Page display" in the View menu).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PageTheme {
    #[default]
    Paper,
    Sepia,
    Night,
}

impl PageTheme {
    pub const ALL: [PageTheme; 3] = [PageTheme::Paper, PageTheme::Sepia, PageTheme::Night];

    pub fn label(self) -> &'static str {
        match self {
            PageTheme::Paper => "Paper",
            PageTheme::Sepia => "Sepia",
            PageTheme::Night => "Night",
        }
    }

    pub fn colors(self) -> PageColors {
        match self {
            PageTheme::Paper => PageColors {
                paper: Color32::WHITE,
                text: Color32::from_rgb(0x1F, 0x1F, 0x1F),
                heading: Color32::from_rgb(0x11, 0x11, 0x11),
                muted: Color32::from_rgb(0x5F, 0x63, 0x68),
                link: ACCENT,
                code_bg: Color32::from_rgb(0xF5, 0xF6, 0xF8),
                code_border: Color32::from_rgb(0xE1, 0xE4, 0xE8),
                rule: Color32::from_rgb(0xD5, 0xD8, 0xDC),
                quote_bar: Color32::from_rgb(0xD0, 0xD4, 0xD9),
                table_head: Color32::from_rgb(0xF2, 0xF3, 0xF5),
                mark: Color32::from_rgba_unmultiplied(0xFF, 0xD3, 0x3D, 150),
                mark_current: Color32::from_rgba_unmultiplied(0xFF, 0x8A, 0x00, 170),
                selection: Color32::from_rgba_unmultiplied(0x3D, 0x8B, 0xF2, 90),
                dark: false,
            },
            PageTheme::Sepia => PageColors {
                paper: Color32::from_rgb(0xF6, 0xEE, 0xDA),
                text: Color32::from_rgb(0x3B, 0x2F, 0x1E),
                heading: Color32::from_rgb(0x2C, 0x22, 0x14),
                muted: Color32::from_rgb(0x7A, 0x68, 0x4E),
                link: Color32::from_rgb(0x8A, 0x4B, 0x08),
                code_bg: Color32::from_rgb(0xEC, 0xE2, 0xCA),
                code_border: Color32::from_rgb(0xDD, 0xD0, 0xB2),
                rule: Color32::from_rgb(0xD8, 0xC9, 0xA8),
                quote_bar: Color32::from_rgb(0xD2, 0xBF, 0x98),
                table_head: Color32::from_rgb(0xEC, 0xE2, 0xCA),
                mark: Color32::from_rgba_unmultiplied(0xFF, 0xC8, 0x3D, 150),
                mark_current: Color32::from_rgba_unmultiplied(0xFF, 0x7A, 0x00, 170),
                selection: Color32::from_rgba_unmultiplied(0x8A, 0x6A, 0x2A, 70),
                dark: false,
            },
            PageTheme::Night => PageColors {
                paper: Color32::from_rgb(0x1E, 0x1F, 0x22),
                text: Color32::from_rgb(0xD6, 0xD6, 0xD6),
                heading: Color32::from_rgb(0xF0, 0xF0, 0xF0),
                muted: Color32::from_rgb(0x9A, 0x9E, 0xA5),
                link: Color32::from_rgb(0x6C, 0xB2, 0xFF),
                code_bg: Color32::from_rgb(0x2A, 0x2C, 0x30),
                code_border: Color32::from_rgb(0x3A, 0x3D, 0x42),
                rule: Color32::from_rgb(0x44, 0x47, 0x4D),
                quote_bar: Color32::from_rgb(0x4A, 0x4E, 0x55),
                table_head: Color32::from_rgb(0x2A, 0x2C, 0x30),
                mark: Color32::from_rgba_unmultiplied(0xC8, 0x9B, 0x00, 140),
                mark_current: Color32::from_rgba_unmultiplied(0xFF, 0x8A, 0x00, 170),
                selection: Color32::from_rgba_unmultiplied(0x3D, 0x8B, 0xF2, 110),
                dark: true,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageColors {
    pub paper: Color32,
    pub text: Color32,
    pub heading: Color32,
    pub muted: Color32,
    pub link: Color32,
    pub code_bg: Color32,
    pub code_border: Color32,
    pub rule: Color32,
    pub quote_bar: Color32,
    pub table_head: Color32,
    pub mark: Color32,
    pub mark_current: Color32,
    pub selection: Color32,
    pub dark: bool,
}

/// Height of toolbar buttons and inputs.
pub const CONTROL_HEIGHT: f32 = 28.0;

pub fn apply(ctx: &egui::Context, p: &Palette) {
    ctx.set_theme(if p.dark { egui::Theme::Dark } else { egui::Theme::Light });
    let mut visuals = if p.dark { Visuals::dark() } else { Visuals::light() };
    let radius = CornerRadius::same(4);
    visuals.panel_fill = p.chrome;
    visuals.window_fill = p.chrome;
    visuals.window_stroke = Stroke::new(1.0, p.border);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.menu_corner_radius = CornerRadius::same(6);
    visuals.window_shadow =
        Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(if p.dark { 120 } else { 45 }) };
    visuals.popup_shadow =
        Shadow { offset: [0, 4], blur: 12, spread: 0, color: Color32::from_black_alpha(if p.dark { 100 } else { 35 }) };
    visuals.extreme_bg_color = p.input;
    visuals.text_edit_bg_color = Some(p.input);
    visuals.faint_bg_color = p.pane;
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = if p.dark {
        Color32::from_rgba_unmultiplied(0x3D, 0x8B, 0xF2, 110)
    } else {
        Color32::from_rgba_unmultiplied(0x3D, 0x8B, 0xF2, 80)
    };
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.striped = true;

    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill = p.chrome;
    w.noninteractive.weak_bg_fill = p.chrome;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = radius;
    w.inactive.bg_fill = p.input;
    w.inactive.weak_bg_fill = p.hover;
    w.inactive.bg_stroke = Stroke::new(1.0, p.border);
    w.inactive.fg_stroke = Stroke::new(1.0, p.text);
    w.inactive.corner_radius = radius;
    w.hovered.bg_fill = p.hover;
    w.hovered.weak_bg_fill = p.hover;
    w.hovered.bg_stroke = Stroke::new(1.0, p.border);
    w.hovered.fg_stroke = Stroke::new(1.5, p.text);
    w.hovered.corner_radius = radius;
    w.hovered.expansion = 0.0;
    w.active.bg_fill = p.selected;
    w.active.weak_bg_fill = p.selected;
    w.active.bg_stroke = Stroke::new(1.0, ACCENT);
    w.active.fg_stroke = Stroke::new(2.0, p.text);
    w.active.corner_radius = radius;
    w.active.expansion = 0.0;
    w.open.bg_fill = p.selected;
    w.open.weak_bg_fill = p.selected;
    w.open.bg_stroke = Stroke::new(1.0, ACCENT);
    w.open.fg_stroke = Stroke::new(1.0, p.text);
    w.open.corner_radius = radius;
    ctx.set_visuals(visuals);

    ctx.global_style_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, FontId::new(18.0, FontFamily::Name("semibold".into()))),
            (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(13.5, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(13.5, FontFamily::Monospace)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.interact_size.y = CONTROL_HEIGHT;
        style.spacing.window_margin = Margin::same(14);
        style.spacing.menu_margin = Margin::same(6);
        style.spacing.combo_width = 90.0;
    });
}
