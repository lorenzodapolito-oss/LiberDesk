//! Palette colorata (niente bianco, nero o grigio) e stile dell'interfaccia.

use eframe::egui::{self, Color32, CornerRadius, Stroke};

/// Sfondo: verde acqua tenue.
pub const BG: Color32 = Color32::from_rgb(0xCF, 0xF1, 0xE6);
/// Pannelli e finestre: crema dorata.
pub const SURFACE: Color32 = Color32::from_rgb(0xFF, 0xEF, 0xC2);
/// Riga alternata delle tabelle: albicocca.
pub const STRIPE: Color32 = Color32::from_rgb(0xFF, 0xE0, 0xB0);
pub const PRIMARY: Color32 = Color32::from_rgb(0x6D, 0x3F, 0xD1);
pub const PRIMARY_LIGHT: Color32 = Color32::from_rgb(0xD9, 0xC8, 0xFB);
/// Testo: blu notte.
pub const TEXT: Color32 = Color32::from_rgb(0x14, 0x2B, 0x5E);
/// Testo secondario: blu petrolio.
pub const MUTED: Color32 = Color32::from_rgb(0x2F, 0x6F, 0x8F);
pub const BORDER: Color32 = Color32::from_rgb(0x5F, 0xB8, 0xA0);
pub const OK: Color32 = Color32::from_rgb(0x0F, 0x8A, 0x4F);
pub const WARN: Color32 = Color32::from_rgb(0xC2, 0x5B, 0x00);
pub const ERR: Color32 = Color32::from_rgb(0xC6, 0x28, 0x4D);
/// Pulsanti principali.
pub const SCAN: Color32 = Color32::from_rgb(0x1E, 0x6F, 0xE0);
pub const DOWNLOAD: Color32 = Color32::from_rgb(0x12, 0x9B, 0x62);
pub const INSTALL: Color32 = Color32::from_rgb(0xE8, 0x6A, 0x0C);
/// Testo sui pulsanti colorati: giallo chiaro.
pub const ON_COLOR: Color32 = Color32::from_rgb(0xFF, 0xF4, 0xB8);
pub const LOG_BG: Color32 = Color32::from_rgb(0x1B, 0x2A, 0x6B);
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
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0xC4, 0xAE, 0xF7);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(0xB0, 0x96, 0xF2);
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
    });
}
