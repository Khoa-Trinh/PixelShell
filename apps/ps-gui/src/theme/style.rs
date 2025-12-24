use super::palette;
use eframe::egui;

pub fn apply_settings(ctx: &egui::Context, is_dark: bool) {
    let colors = palette::get_colors(is_dark);

    // We construct the visuals dynamically from the Palette struct
    let visuals = if is_dark {
        generate_visuals(egui::Visuals::dark(), &colors)
    } else {
        generate_visuals(egui::Visuals::light(), &colors)
    };

    let mut style = (*ctx.style()).clone();
    style.visuals = visuals;

    // Typography & Spacing (remains standard)
    style.text_styles = [
        (
            egui::TextStyle::Heading,
            egui::FontId::new(24.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Body,
            egui::FontId::new(15.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Monospace,
            egui::FontId::new(13.0, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Button,
            egui::FontId::new(15.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Small,
            egui::FontId::new(11.0, egui::FontFamily::Proportional),
        ),
    ]
    .into();

    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.window_margin = egui::Margin::same(16.0);
    style.spacing.button_padding = egui::vec2(16.0, 8.0);

    ctx.set_style(style);
}

// Helper to map Palette -> egui::Visuals
fn generate_visuals(mut v: egui::Visuals, colors: &palette::Palette) -> egui::Visuals {
    v.window_rounding = egui::Rounding::same(8.0);
    v.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 4.0),
        blur: 15.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(90),
    };

    // Map palette fields to Global Visuals
    v.widgets.noninteractive.bg_fill = colors.bg_base;
    v.panel_fill = colors.bg_base;

    // Global text color defaults
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, colors.text_strong);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, colors.text_strong);

    v
}
