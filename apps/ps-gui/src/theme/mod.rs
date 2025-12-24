pub mod palette;
pub mod style;
pub mod widgets;

// --- RE-EXPORTS ---
// This allows you to import everything from `theme::` in your main code.
pub use palette::ACCENT;

// From style.rs
pub use style::apply_settings;

// From widgets.rs
pub use widgets::{
    card,          // theme::card(...)
    combo_box,     // theme::combo_box(...)
    styled_button, // theme::styled_button(...)
    text_input,    // theme::text_input(...)
    ButtonVariant, // theme::ButtonVariant::Primary
};

#[derive(PartialEq, Clone, Copy)]
pub enum Theme {
    Dark,
    Light,
    System,
}
