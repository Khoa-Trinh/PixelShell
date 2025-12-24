use crate::{app::PsApp, theme, utils};
use eframe::egui;
use ps_factory::builder::{self, utils as builder_utils};
use std::thread;

pub fn show(app: &mut PsApp, ui: &mut egui::Ui) {
    ui.heading("Build Executable");
    ui.label("Package your .bin overlays into standalone .exe files for distribution.");
    ui.add_space(15.0);

    // CARD 1: SELECTION
    // We place the "Refresh" button in the card header
    let (_, refresh_clicked) = theme::card(
        ui,
        "1. Project Selection",
        Some(("🔄 Refresh", theme::ButtonVariant::Secondary)),
        |ui| {
            // -- Row 1: Source Control --
            ui.horizontal(|ui| {
                if ui.button("📂 Browse System...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Pixel Shell Bin", &["bin"])
                        .pick_file()
                    {
                        app.bd_manual_path = Some(path);
                    }
                }

                if let Some(_path) = &app.bd_manual_path {
                    if ui.small_button("❌ Clear Manual Selection").clicked() {
                        app.bd_manual_path = None;
                    }
                }
            });

            ui.add_space(5.0);

            // -- Logic: Determine Display Text --
            let selected_text = if let Some(p) = &app.bd_manual_path {
                format!("📄 {}", p.file_name().unwrap().to_string_lossy())
            } else if !app.bd_targets.is_empty() {
                // Safety check for index out of bounds
                if app.bd_selected_idx >= app.bd_targets.len() {
                    app.bd_selected_idx = 0;
                }
                let t = &app.bd_targets[app.bd_selected_idx];
                format!("📦 {} ({})", t.project, t.resolution)
            } else {
                "⚠️ No valid builds found".to_string()
            };

            // Clone state for borrowing inside the closure
            let manual_opt = app.bd_manual_path.clone();
            let targets_clone = app.bd_targets.clone();
            let sel_idx = app.bd_selected_idx;

            // -- Row 2: Dropdown --
            ui.horizontal(|ui| {
                ui.label("Target Project:");

                theme::combo_box(ui, "bd_project_dropdown", &selected_text, |ui| {
                    // Option A: Manual Selection (if exists)
                    if let Some(p) = manual_opt {
                        ui.selectable_value(
                            &mut app.bd_manual_path,
                            Some(p.clone()),
                            format!("📄 {}", p.file_name().unwrap().to_string_lossy()),
                        );
                    }

                    // Option B: Scanned Targets
                    for (i, target) in targets_clone.iter().enumerate() {
                        let label = format!("{} ({})", target.project, target.resolution);
                        // We check if this item is "Selected" if:
                        // 1. Manual path is NONE
                        // 2. The index matches
                        let is_selected = app.bd_manual_path.is_none() && sel_idx == i;

                        if ui.selectable_label(is_selected, label).clicked() {
                            app.bd_selected_idx = i;
                            app.bd_manual_path = None; // Switch back to auto-list mode
                        }
                    }
                });
            });
        },
    );

    // Handle Refresh Action
    if refresh_clicked {
        app.bd_targets = utils::scan_build_targets();
    }

    ui.add_space(20.0);

    // MAIN ACTION BUTTON
    let has_selection = app.bd_manual_path.is_some() || !app.bd_targets.is_empty();

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // [FIX] Use add_enabled_ui instead of add_enabled
            ui.add_enabled_ui(!app.is_busy && has_selection, |ui| {
                if theme::styled_button(
                    ui,
                    "🔨 Build Selected Project",
                    theme::ButtonVariant::Primary,
                )
                .clicked()
                {
                    start_build(app);
                }
            });
        });
    });
}

fn start_build(app: &mut PsApp) {
    app.is_busy = true;
    let tx = app.status_tx.clone();

    // 1. Construct the Target
    // We either use the manually selected path OR the one from the scanned list.
    let target = if let Some(path) = &app.bd_manual_path {
        let filename = path.file_stem().unwrap().to_string_lossy().to_string();

        // Use shared utils to detect metadata
        let audio_path = builder_utils::detect_audio_path(path);
        let (res_str, w, h) = builder_utils::detect_resolution(&filename);

        builder::BuildTarget {
            project: filename,
            resolution: res_str,
            width: w,
            height: h,
            bin_path: path.clone(),
            audio_path,
        }
    } else {
        // Use the pre-scanned target directly
        app.bd_targets[app.bd_selected_idx].clone()
    };

    // 2. Spawn Build Thread
    thread::spawn(move || {
        tx.send(format!("Building target: {}", target.project)).ok();

        let targets = vec![target];
        let (internal_tx, internal_rx) = std::sync::mpsc::channel();

        // Run the async builder logic
        let _ = builder::run_async(targets, internal_tx);

        // Forward status updates to the main app
        while let Ok(status) = internal_rx.recv() {
            match status {
                builder::BuildStatus::Building(name) => {
                    tx.send(format!("Building: {}", name)).ok();
                }
                builder::BuildStatus::Finished(path) => {
                    tx.send(format!("Success: {:?}", path.file_name().unwrap()))
                        .ok();
                }
                builder::BuildStatus::Error(e) => {
                    tx.send(format!("Error: {}", e)).ok();
                }
                _ => {}
            }
        }
        tx.send("Build Finished.".into()).ok();
    });
}
