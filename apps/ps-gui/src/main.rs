use eframe::egui;

mod app;
mod tabs;
mod theme;
mod utils;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_title("Pixel Shell Factory"),
        ..Default::default()
    };
    eframe::run_native(
        "Pixel Shell Factory",
        options,
        Box::new(|_cc| Box::new(app::PsApp::default())),
    )
}
