//! Palette colorata (niente bianco, nero o grigio) e stile dell'interfaccia.

use eframe::egui::{self, Color32, CornerRadius, Stroke};

/// Sfondo: celeste pastello.
pub const BG: Color32 = Color32::from_rgb(0xC9, 0xE8, 0xFA);
/// Pannelli e finestre: celeste un po' più chiaro (mai bianco).
pub const SURFACE: Color32 = Color32::from_rgb(0xDA, 0xF0, 0xFC);
/// Riga alternata delle tabelle: lilla chiaro.
pub const STRIPE: Color32 = Color32::from_rgb(0xB9, 0xDF, 0xF5);
pub const PRIMARY: Color32 = Color32::from_rgb(0x2B, 0x3B, 0xB8);
pub const PRIMARY_LIGHT: Color32 = Color32::from_rgb(0xAF, 0xD6, 0xF2);
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

/// Colori dell'interfaccia: mai bianco né grigio, nemmeno nei dettagli (testo, ombre, bordi).
pub fn visuals() -> egui::Visuals {
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
    harmonize(&mut v);
    v
}

/// Sostituisce i grigi e i bianchi di egui (testo dei pulsanti, ombre, cursore, bordi).
fn harmonize(v: &mut egui::Visuals) {
    v.weak_text_color = Some(MUTED);
    v.code_bg_color = SURFACE;
    v.faint_bg_color = STRIPE;
    v.extreme_bg_color = SURFACE;
    v.window_stroke = Stroke::new(1.0, BORDER);
    let shadow = Color32::from_rgba_unmultiplied(0x2B, 0x4A, 0x7A, 60);
    v.window_shadow.color = shadow;
    v.popup_shadow.color = shadow;
    v.text_cursor.stroke.color = PRIMARY;
    v.widgets.noninteractive.weak_bg_fill = SURFACE;
    v.widgets.open.bg_fill = PRIMARY_LIGHT;
    v.widgets.open.weak_bg_fill = PRIMARY_LIGHT;
    v.widgets.open.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.hovered.bg_fill = v.widgets.hovered.weak_bg_fill;
    v.widgets.active.bg_fill = v.widgets.active.weak_bg_fill;
    v.widgets.active.bg_stroke = Stroke::new(1.5, PRIMARY);
    v.text_edit_bg_color = Some(SURFACE);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.fg_stroke.color = TEXT;
    }
}

pub fn apply(ctx: &egui::Context) {
    let v = visuals();
    ctx.set_theme(egui::ThemePreference::Light);
    ctx.set_visuals_of(egui::Theme::Light, v.clone());
    ctx.set_visuals_of(egui::Theme::Dark, v);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn neutral(c: Color32) -> bool {
        let [r, g, b, _] = c.to_srgba_unmultiplied().map(i32::from);
        r.max(g).max(b) - r.min(g).min(b) < 24
    }

    #[test]
    fn no_white_or_gray_anywhere_in_the_visuals() {
        let v = visuals();
        let mut named: Vec<(&str, Color32)> = vec![
            ("panel", v.panel_fill),
            ("window", v.window_fill),
            ("faint", v.faint_bg_color),
            ("extreme", v.extreme_bg_color),
            ("code", v.code_bg_color),
            ("window_stroke", v.window_stroke.color),
            ("window_shadow", v.window_shadow.color),
            ("popup_shadow", v.popup_shadow.color),
            ("selection", v.selection.bg_fill),
            ("selection_stroke", v.selection.stroke.color),
            ("hyperlink", v.hyperlink_color),
            ("cursor", v.text_cursor.stroke.color),
            ("warn", v.warn_fg_color),
            ("error", v.error_fg_color),
        ];
        if let Some(c) = v.override_text_color {
            named.push(("text", c));
        }
        if let Some(c) = v.weak_text_color {
            named.push(("weak_text", c));
        }
        if let Some(c) = v.text_edit_bg_color {
            named.push(("text_edit", c));
        }
        for (n, w) in [
            ("noninteractive", v.widgets.noninteractive),
            ("inactive", v.widgets.inactive),
            ("hovered", v.widgets.hovered),
            ("active", v.widgets.active),
            ("open", v.widgets.open),
        ] {
            named.push((n, w.bg_fill));
            named.push((n, w.weak_bg_fill));
            named.push((n, w.bg_stroke.color));
            named.push((n, w.fg_stroke.color));
        }
        for (n, c) in named {
            assert!(!neutral(c), "{n}: {c:?} è bianco o grigio");
        }
    }
}
