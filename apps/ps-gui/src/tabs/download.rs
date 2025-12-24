use crate::{app::PsApp, theme};
use eframe::egui;
use ps_factory::downloader;
use std::thread;

pub fn show(app: &mut PsApp, ui: &mut egui::Ui) {
    ui.heading("Download Video");
    ui.label("Fetch videos from YouTube and prepare them for conversion.");
    ui.add_space(15.0);

    // CARD 1: SOURCE
    theme::card(ui, "1. Source Configuration", None, |ui| {
        ui.label("YouTube URL:");
        theme::text_input(ui, &mut app.dl_url, "https://youtube.com/watch?v=...");

        ui.add_space(10.0);

        ui.label("Project Name:");
        theme::text_input(ui, &mut app.dl_project, "my_cool_video");
    });

    ui.add_space(15.0);

    // CARD 2: SETTINGS (Now with a Reset button in the header)
    let (_, reset_clicked) = theme::card(
        ui,
        "2. Output Settings",
        Some(("↺ Reset", theme::ButtonVariant::Secondary)),
        |ui| {
            ui.label("Target Resolution:");

            let res_options = [("1080p (FHD)", "1080p"), ("720p (HD)", "720p")];
            let current_res = app.dl_res.clone();

            // 2. Pass &app.dl_res directly. Do NOT clone.
            // Cloning here creates a temporary string that disappears immediately.
            theme::combo_box(ui, "dl_res", &current_res, |ui| {
                for (label, value) in res_options {
                    // We only convert to String here because app.dl_res is likely a String.
                    // This is unavoidable unless you change app.dl_res to an Enum (the ideal Rust way).
                    ui.selectable_value(&mut app.dl_res, value.to_string(), label);
                }
            });

            ui.label("Target FPS:");
            let current_fps = app.dl_fps.clone();

            let fps_options = [("30 FPS", "30"), ("60 FPS", "60")];

            theme::combo_box(ui, "dl_fps", &current_fps, |ui| {
                for (label, value) in fps_options {
                    ui.selectable_value(&mut app.dl_fps, value.to_string(), label);
                }
            });
        },
    );

    // Handle Reset Action
    if reset_clicked {
        app.dl_res = "1080p".into();
        app.dl_fps = "30".into();
    }

    ui.add_space(20.0);

    // ACTION BUTTON
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // [FIX] Use add_enabled_ui to handle the styling wrapper correctly
            ui.add_enabled_ui(!app.is_busy, |ui| {
                if theme::styled_button(ui, "🚀 Start Download", theme::ButtonVariant::Primary)
                    .clicked()
                {
                    start_download(app);
                }
            });
        });
    });
}

fn start_download(app: &mut PsApp) {
    app.is_busy = true;
    let tx = app.status_tx.clone();

    let job = downloader::DownloadJob {
        url: app.dl_url.clone(),
        project_name: app.dl_project.clone(),
        width: if app.dl_res == "1080p" { 1920 } else { 1280 },
        height: if app.dl_res == "1080p" { 1080 } else { 720 },
        fps: app.dl_fps.parse().unwrap_or(30),
        use_gpu: false,
    };

    thread::spawn(move || {
        let (internal_tx, internal_rx) = std::sync::mpsc::channel();
        let _ = downloader::run_async(job, internal_tx);
        while let Ok(status) = internal_rx.recv() {
            let msg = match status {
                downloader::DownloadStatus::Downloading(p) => {
                    format!("[Downloading] {:.0}%", p * 100.0)
                }
                downloader::DownloadStatus::ProcessingVideo(p) => {
                    format!("[Processing] {:.0}%", p * 100.0)
                }
                downloader::DownloadStatus::ExtractingAudio(p) => {
                    format!("[Extracting] {:.0}%", p * 100.0)
                }
                downloader::DownloadStatus::Finished(_) => "Download Complete!".into(),
                downloader::DownloadStatus::Error(e) => format!("Error: {}", e),
                _ => format!("{:?}", status),
            };
            tx.send(msg).unwrap();
        }
    });
}
