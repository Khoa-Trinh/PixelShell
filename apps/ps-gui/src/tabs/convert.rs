use crate::{app::PsApp, theme, utils};
use eframe::egui;
use ps_factory::converter;
use std::thread;

pub fn show(app: &mut PsApp, ui: &mut egui::Ui) {
    ui.heading("Convert to Binary");
    ui.label("Process video files into the optimized .bin format for Pixel Shell.");
    ui.add_space(15.0);

    // CARD 1: SOURCE SELECTION
    // Header Action: Refresh List
    let (_, refresh_clicked) = theme::card(
        ui,
        "1. Source Video",
        Some(("🔄 Refresh", theme::ButtonVariant::Secondary)),
        |ui| {
            // -- Row 1: System Browse --
            ui.horizontal(|ui| {
                if ui.button("📂 Browse System...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Video", &["mp4", "mkv", "mov"])
                        .pick_file()
                    {
                        app.cv_manual_path = Some(path);
                    }
                }

                if let Some(_path) = &app.cv_manual_path {
                    if ui.small_button("❌ Clear Selection").clicked() {
                        app.cv_manual_path = None;
                    }
                }
            });

            ui.add_space(5.0);

            // -- Logic: Determine Display Text --
            let selected_text = if let Some(p) = &app.cv_manual_path {
                format!("📄 {}", p.file_name().unwrap().to_string_lossy())
            } else if !app.cv_files.is_empty() {
                if app.cv_selected_idx >= app.cv_files.len() {
                    app.cv_selected_idx = 0;
                }
                format!(
                    "🎞️  {}",
                    app.cv_files[app.cv_selected_idx]
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                )
            } else {
                "⚠️ No video files found".to_string()
            };

            // Clone for borrowing safety
            let manual_opt = app.cv_manual_path.clone();
            let files_clone = app.cv_files.clone();
            let sel_idx = app.cv_selected_idx;

            // -- Row 2: Dropdown --
            ui.horizontal(|ui| {
                ui.label("Selected File:");

                theme::combo_box(ui, "cv_source_dropdown", &selected_text, |ui| {
                    // Option A: Manual
                    if let Some(p) = manual_opt {
                        ui.selectable_value(
                            &mut app.cv_manual_path,
                            Some(p.clone()),
                            format!("📄 {}", p.file_name().unwrap().to_string_lossy()),
                        );
                    }

                    // Option B: Scanned List
                    for (i, file) in files_clone.iter().enumerate() {
                        let name = file.file_name().unwrap().to_string_lossy();
                        // Is selected if manual is None AND index matches
                        let is_selected = app.cv_manual_path.is_none() && sel_idx == i;

                        if ui.selectable_label(is_selected, name).clicked() {
                            app.cv_selected_idx = i;
                            app.cv_manual_path = None;
                        }
                    }
                });
            });
        },
    );

    if refresh_clicked {
        app.cv_files = utils::scan_video_files();
    }

    ui.add_space(15.0);

    // CARD 2: OUTPUT SETTINGS
    theme::card(ui, "2. Output Configuration", None, |ui| {
        ui.label("Target Resolutions:");

        egui::Grid::new("cv_resolutions_grid")
            .spacing([40.0, 10.0])
            .show(ui, |ui| {
                ui.checkbox(&mut app.cv_res_720, "720p (HD)");
                ui.checkbox(&mut app.cv_res_1080, "1080p (FHD)");
                ui.end_row();

                ui.checkbox(&mut app.cv_res_1440, "1440p (2K)");
                ui.checkbox(&mut app.cv_res_2160, "2160p (4K)");
                ui.end_row();
            });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(5.0);

        ui.checkbox(&mut app.cv_gpu, "Use GPU Acceleration (NVENC/CUDA)");
        ui.label(
            egui::RichText::new("Requires compatible NVIDIA hardware.")
                .small()
                .weak(),
        );
    });

    ui.add_space(20.0);

    // ACTION BUTTON
    let has_res = app.cv_res_720 || app.cv_res_1080 || app.cv_res_1440 || app.cv_res_2160;
    let has_file = app.cv_manual_path.is_some() || !app.cv_files.is_empty();

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // [FIX] Use add_enabled_ui wrapper
            ui.add_enabled_ui(!app.is_busy && has_res && has_file, |ui| {
                if theme::styled_button(
                    ui,
                    "🔄 Start Batch Conversion",
                    theme::ButtonVariant::Primary,
                )
                .clicked()
                {
                    start_conversion(app);
                }
            });
        });
    });
}

fn start_conversion(app: &mut PsApp) {
    app.is_busy = true;
    let tx = app.status_tx.clone();

    // Determine Input Path
    let input_path = app
        .cv_manual_path
        .clone()
        .unwrap_or_else(|| app.cv_files[app.cv_selected_idx].clone());

    // Prepare Jobs
    let mut jobs = Vec::new();
    if app.cv_res_720 {
        jobs.push((1280, 720, "720p"));
    }
    if app.cv_res_1080 {
        jobs.push((1920, 1080, "1080p"));
    }
    if app.cv_res_1440 {
        jobs.push((2560, 1440, "1440p"));
    }
    if app.cv_res_2160 {
        jobs.push((3840, 2160, "2160p"));
    }
    let gpu = app.cv_gpu;

    thread::spawn(move || {
        let fps = 30; // Ideally detected via ffprobe, currently hardcoded fallback

        for (w, h, label) in jobs {
            tx.send(format!("Starting {} conversion...", label)).ok();
            let (internal_tx, internal_rx) = std::sync::mpsc::channel();

            let out_name = format!(
                "{}_{}.bin",
                input_path.file_stem().unwrap().to_string_lossy(),
                label
            );
            let output_path = input_path.parent().unwrap().join(out_name);

            let job = converter::ConvertJob {
                input_path: input_path.clone(),
                output_path,
                width: w,
                height: h,
                fps,
                use_gpu: gpu,
            };

            let _ = converter::run_async(job, internal_tx);

            while let Ok(status) = internal_rx.recv() {
                match status {
                    converter::ConverterStatus::Processing {
                        current_frame,
                        total_frames,
                        ..
                    } => {
                        let p = current_frame as f32 / total_frames as f32;
                        tx.send(format!("[Processing] {:.0}%", p * 100.0)).ok();
                    }
                    converter::ConverterStatus::Finished => {}
                    converter::ConverterStatus::Error(e) => {
                        tx.send(format!("Error: {}", e)).ok();
                    }
                    _ => {}
                }
            }
        }
        tx.send("Conversion Batch Complete!".into()).ok();
    });
}
