use eframe::egui::Color32;

// --- GLOBAL ACCENT ---
pub const ACCENT: Color32 = Color32::from_rgb(0, 122, 255);
// pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0, 110, 230);

// --- DARK MODE CONSTANTS ---
pub const DARK_BG_BASE: Color32 = Color32::from_rgb(32, 32, 35);
pub const DARK_BG_HEADER: Color32 = Color32::from_rgb(40, 40, 45);
pub const DARK_BG_INPUT: Color32 = Color32::from_rgb(45, 45, 50);
pub const DARK_BORDER: Color32 = Color32::from_gray(60);
// pub const DARK_TEXT_WEAK: Color32 = Color32::from_gray(140);
pub const DARK_TEXT_STRONG: Color32 = Color32::from_gray(240);
pub const DARK_OVERLAY_HOVER: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 85);
// --- LIGHT MODE CONSTANTS ---
pub const LIGHT_BG_BASE: Color32 = Color32::from_rgb(255, 255, 255);
pub const LIGHT_BG_HEADER: Color32 = Color32::from_gray(248);
pub const LIGHT_BG_INPUT: Color32 = Color32::from_rgb(240, 240, 245);
pub const LIGHT_BORDER: Color32 = Color32::from_gray(220);
// pub const LIGHT_TEXT_WEAK: Color32 = Color32::from_gray(100);
pub const LIGHT_TEXT_STRONG: Color32 = Color32::from_gray(40);
pub const LIGHT_OVERLAY_HOVER: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 15);
// --- THE STRUCT (The single source of truth) ---
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg_base: Color32,
    pub bg_header: Color32,
    pub bg_input: Color32,
    pub border: Color32,

    pub text_strong: Color32,
    // pub text_weak: Color32,
    pub accent: Color32,
    // pub accent_hover: Color32,

    // For lists/dropdown items hover state
    pub overlay_hover: Color32,
}

pub fn get_colors(is_dark: bool) -> Palette {
    if is_dark {
        Palette {
            bg_base: DARK_BG_BASE,
            bg_header: DARK_BG_HEADER,
            bg_input: DARK_BG_INPUT,
            border: DARK_BORDER,
            text_strong: DARK_TEXT_STRONG,
            // text_weak: DARK_TEXT_WEAK,
            accent: ACCENT,
            // accent_hover: ACCENT_HOVER,
            overlay_hover: DARK_OVERLAY_HOVER,
        }
    } else {
        Palette {
            bg_base: LIGHT_BG_BASE,
            bg_header: LIGHT_BG_HEADER,
            bg_input: LIGHT_BG_INPUT,
            border: LIGHT_BORDER,
            text_strong: LIGHT_TEXT_STRONG,
            // text_weak: LIGHT_TEXT_WEAK,
            accent: ACCENT,
            // accent_hover: ACCENT_HOVER,
            overlay_hover: LIGHT_OVERLAY_HOVER,
        }
    }
}
