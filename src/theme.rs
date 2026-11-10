//! Palette colorata (niente bianco, nero o grigio) e stile dell'interfaccia.

use eframe::egui::{self, Color32, CornerRadius, Stroke};

/// Sfondo: azzurro lavanda tenue.
pub const BG: Color32 = Color32::from_rgb(0xDD, 0xE4, 0xFF);
/// Pannelli e finestre: rosa tenue.
pub const SURFACE: Color32 = Color32::from_rgb(0xFF, 0xE4, 0xF2);
/// Riga alternata delle tabelle: lilla chiaro.
pub const STRIPE: Color32 = Color32::from_rgb(0xEE, 0xE6, 0xFF);
pub const PRIMARY: Color32 = Color32::from_rgb(0x2B, 0x3B, 0xB8);
pub const PRIMARY_LIGHT: Color32 = Color32::from_rgb(0xC9, 0xD3, 0xFF);
/// Testo: blu notte.
pub const TEXT: Color32 = Color32::from_rgb(0x14, 0x1E, 0x6B);
/// Testo secondario: blu petrolio.
pub const MUTED: Color32 = Color32::from_rgb(0x5A, 0x4F, 0xA8);
pub const BORDER: Color32 = Color32::from_rgb(0xB8, 0x9C, 0xE8);
pub const OK: Color32 = Color32::from_rgb(0x0F, 0x8A, 0x4F);
pub const WARN: Color32 = Color32::from_rgb(0xC2, 0x5B, 0x00);
pub const ERR: Color32 = Color32::from_rgb(0xC6, 0x28, 0x4D);
/// Pulsanti principali.
pub const SCAN: Color32 = Color32::from_rgb(0x2B, 0x3B, 0xB8);
pub const DOWNLOAD: Color32 = Color32::from_rgb(0xC7, 0x2A, 0x93);
pub const INSTALL: Color32 = Color32::from_rgb(0xC7, 0x2A, 0x93);
/// Testo sui pulsanti colorati: giallo chiaro.
pub const ON_COLOR: Color32 = Color32::from_rgb(0xFF, 0xF0, 0xFA);
pub const LOG_BG: Color32 = Color32::from_rgb(0x14, 0x1E, 0x6B);
pub const LOG_INFO: Color32 = Color32::from_rgb(0x9F, 0xD4, 0xFF);
pub const LOG_OK: Color32 = Color32::from_rgb(0x7B, 0xF0, 0xB0);
pub const LOG_WARN: Color32 = Color32::from_rgb(0xFF, 0xD9, 0x7A);
pub const LOG_ERR: Color32 = Color32::from_rgb(0xFF, 0x9A, 0xB0);

pub fn apply(ctx: &egui::Context) {
    let mut v = egui::Visuals::light();
    v.panel_fill = BG;
    v.window_fill = SURFACE;
    v.extreme_bg_color = SURFACE;
    v.faint_bg_color = STRIPE;
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = PRIMARY_LIGHT;
    v.selection.stroke = Stroke::new(1.0, PRIMARY);
    v.hyperlink_color = PRIMARY;
    let r = CornerRadius::same(8);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
    ] {
        w.corner_radius = r;
    }
    v.widgets.noninteractive.bg_fill = SURFACE;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.inactive.bg_fill = PRIMARY_LIGHT;
    v.widgets.inactive.weak_bg_fill = PRIMARY_LIGHT;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, PRIMARY);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0xB3, 0xC0, 0xFF);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(0x9F, 0xAE, 0xFF);
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
    });
}
