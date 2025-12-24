use super::palette::{self, Palette};
use eframe::egui;

// ============================================================================
// BUTTONS
// ============================================================================

#[derive(PartialEq, Clone, Copy)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    // Destructive,
}

impl ButtonVariant {
    fn get_colors(&self, colors: &Palette) -> (egui::Color32, egui::Color32) {
        match self {
            ButtonVariant::Primary => (colors.accent, egui::Color32::WHITE),
            ButtonVariant::Secondary => (colors.bg_input, colors.text_strong),
            // ButtonVariant::Destructive => {
            // (egui::Color32::from_rgb(200, 50, 50), egui::Color32::WHITE)
            // }
        }
    }
}

pub fn styled_button(ui: &mut egui::Ui, text: &str, variant: ButtonVariant) -> egui::Response {
    let colors = palette::get_colors(ui.visuals().dark_mode);
    let (bg_color, text_color) = variant.get_colors(&colors);

    ui.add(
        egui::Button::new(egui::RichText::new(text).color(text_color).strong())
            .fill(bg_color)
            .rounding(6.0)
            .min_size(egui::vec2(0.0, 32.0)), // Standard Height
    )
}

// ============================================================================
// WRAPPER: box32
// ============================================================================

/// Creates a dedicated block that is forced to be 32px high.
/// It centers whatever content you put inside it vertically.
fn box_32<R>(
    ui: &mut egui::Ui,
    width: f32,
    align_right: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let layout = if align_right {
        egui::Layout::right_to_left(egui::Align::Center)
    } else {
        egui::Layout::left_to_right(egui::Align::Center)
    };

    ui.allocate_ui_with_layout(egui::vec2(width, 32.0), layout, |ui| add_contents(ui))
        .inner
}

// ============================================================================
// CARD COMPONENT
// ============================================================================

pub fn card<R>(
    ui: &mut egui::Ui,
    title: &str,
    action: Option<(&str, ButtonVariant)>,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> (R, bool) {
    let colors = palette::get_colors(ui.visuals().dark_mode);
    let mut action_clicked = false;

    let inner_result = egui::Frame::none()
        .fill(colors.bg_base)
        .rounding(8.0)
        .stroke(egui::Stroke::new(1.0, colors.border))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;

            // --- HEADER ---
            egui::Frame::none()
                .fill(colors.bg_header)
                .rounding(egui::Rounding {
                    nw: 8.0,
                    ne: 8.0,
                    sw: 0.0,
                    se: 0.0,
                })
                .inner_margin(egui::Margin::symmetric(16.0, 8.0))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    // Parent Horizontal Row
                    ui.horizontal(|ui| {
                        // CHILD 1: TITLE WRAPPER
                        // Takes available width but leaves room for button if it exists
                        // Note: We use allocate_ui to control the width specifically for the title part
                        // so it pushes against the button correctly in a horizontal layout.
                        let title_width = if action.is_some() {
                            ui.available_width() - 80.0 // Reserve space for button
                        } else {
                            ui.available_width()
                        };

                        box_32(ui, title_width, false, |ui| {
                            ui.label(
                                egui::RichText::new(title)
                                    .strong()
                                    .size(14.0)
                                    .color(colors.text_strong),
                            );
                        });

                        // CHILD 2: ACTION BUTTON WRAPPER
                        if let Some((label, variant)) = action {
                            // Separate box32 for the button
                            box_32(ui, ui.available_width(), true, |ui| {
                                let (bg, fg) = variant.get_colors(&colors);
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(label).size(12.0).color(fg),
                                        )
                                        .fill(bg)
                                        .rounding(4.0)
                                        .min_size(egui::vec2(60.0, 24.0)),
                                    )
                                    .clicked()
                                {
                                    action_clicked = true;
                                }
                            });
                        }
                    });
                });

            // --- SEPARATOR ---
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0.0, colors.border);

            // --- BODY ---
            egui::Frame::none()
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
                    add_contents(ui)
                })
                .inner
        })
        .inner;

    (inner_result, action_clicked)
}

// ============================================================================
// COMBO BOX
// ============================================================================

pub fn combo_box(
    ui: &mut egui::Ui,
    id: &str,
    selected_text: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let colors = palette::get_colors(ui.visuals().dark_mode);

    ui.scope(|ui| {
        // [FIX] Same 32px height fix
        ui.spacing_mut().button_padding = egui::vec2(10.0, 7.0);
        let v = ui.visuals_mut();

        // Styling (Same as your other combo box)
        v.widgets.inactive.rounding = 6.0.into();
        v.widgets.inactive.weak_bg_fill = colors.bg_input;
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, colors.border);
        v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, colors.text_strong);

        v.widgets.hovered = v.widgets.inactive;
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, colors.accent);

        v.widgets.open = v.widgets.inactive;
        v.widgets.open.bg_stroke = egui::Stroke::new(1.5, colors.accent);

        egui::ComboBox::from_id_source(id)
            .selected_text(selected_text)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                ui.spacing_mut().button_padding = egui::vec2(10.0, 8.0);

                // Dropdown Item Styles
                let lv = ui.visuals_mut();
                lv.widgets.active.rounding = 6.0.into();
                lv.widgets.active.weak_bg_fill = colors.accent;
                lv.widgets.active.bg_fill = colors.accent;
                lv.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

                lv.widgets.hovered.rounding = 6.0.into();
                lv.widgets.hovered.weak_bg_fill = colors.overlay_hover;
                lv.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                lv.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, colors.text_strong);

                lv.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                lv.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                lv.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, colors.text_strong);

                // Run your custom logic here
                add_contents(ui);
            });
    });
} // ============================================================================
  // TEXT INPUT
  // ============================================================================

pub fn text_input(ui: &mut egui::Ui, value: &mut String, hint: &str) -> egui::Response {
    let colors = palette::get_colors(ui.visuals().dark_mode);
    let padding = egui::Margin::symmetric(12.0, 10.0);
    let rounding = egui::Rounding::same(6.0);

    let response = egui::Frame::none()
        .inner_margin(padding)
        .fill(colors.bg_input)
        .rounding(rounding)
        .stroke(egui::Stroke::new(1.0, colors.border))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(value)
                    .hint_text(hint)
                    .desired_width(f32::INFINITY)
                    .frame(false)
                    .text_color(colors.text_strong)
                    .vertical_align(egui::Align::Center),
            )
        })
        .inner;

    let visual_rect = response.rect.expand2(egui::vec2(12.0, 10.0));

    if response.has_focus() {
        ui.painter()
            .rect_stroke(visual_rect, rounding, egui::Stroke::new(1.5, colors.accent));
    } else if response.hovered() {
        ui.painter()
            .rect_stroke(visual_rect, rounding, egui::Stroke::new(1.0, colors.accent));
    }

    response
}
