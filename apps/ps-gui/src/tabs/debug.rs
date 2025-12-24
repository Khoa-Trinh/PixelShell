use crate::{app::PsApp, theme, utils};
use eframe::egui;
use ps_factory::debugger;
use std::thread;

pub fn show(app: &mut PsApp, ui: &mut egui::Ui) {
    ui.heading("Debugger");
    ui.label("Inspect overlay data structures or verify built executables.");
    ui.add_space(15.0);

    // CARD 1: MODE SELECTION
    theme::card(ui, "1. Debug Mode", None, |ui| {
        ui.horizontal(|ui| {
            // Mode: Project File (.bin)
            if ui
                .selectable_label(!app.db_mode_exe, "📄 Project File (.bin)")
                .clicked()
            {
                app.db_mode_exe = false;
                app.db_files = utils::scan_bin_files();
                app.db_selected_idx = 0;
                app.db_manual_path = None;
            }

            // Mode: Built Executable (.exe)
            if ui
                .selectable_label(app.db_mode_exe, "📦 Built Executable (.exe)")
                .clicked()
            {
                app.db_mode_exe = true;
                app.db_files = utils::scan_dist_files();
                app.db_selected_idx = 0;
                app.db_manual_path = None;
            }
        });

        ui.add_space(5.0);
        ui.label(
            egui::RichText::new(if app.db_mode_exe {
                "Debug final packaged executables before distribution."
            } else {
                "Debug raw binary overlay files directly from conversion."
            })
            .small()
            .weak(),
        );
    });

    ui.add_space(15.0);

    // CARD 2: FILE SELECTION
    // Header Action: Refresh List
    let (_, refresh_clicked) = theme::card(
        ui,
        "2. Target File",
        Some(("🔄 Refresh", theme::ButtonVariant::Secondary)),
        |ui| {
            // -- Row 1: Browse --
            ui.horizontal(|ui| {
                if ui.button("📂 Browse System...").clicked() {
                    let filter = if app.db_mode_exe { "exe" } else { "bin" };
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Target", &[filter])
                        .pick_file()
                    {
                        app.db_manual_path = Some(path);
                    }
                }

                if let Some(_path) = &app.db_manual_path {
                    if ui.small_button("❌ Clear Selection").clicked() {
                        app.db_manual_path = None;
                    }
                }
            });

            ui.add_space(5.0);

            // -- Logic: Determine Display Text --
            let selected_text = if let Some(p) = &app.db_manual_path {
                format!("📄 {}", p.file_name().unwrap().to_string_lossy())
            } else if !app.db_files.is_empty() {
                if app.db_selected_idx >= app.db_files.len() {
                    app.db_selected_idx = 0;
                }
                format!(
                    "🔍 {}",
                    app.db_files[app.db_selected_idx]
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                )
            } else {
                "⚠️ No files found".to_string()
            };

            // Clone for borrowing safety
            let manual_opt = app.db_manual_path.clone();
            let files_clone = app.db_files.clone();
            let sel_idx = app.db_selected_idx;

            // -- Row 2: Dropdown --
            ui.horizontal(|ui| {
                ui.label("Selected:");

                theme::combo_box(ui, "db_file_dropdown", &selected_text, |ui| {
                    // Option A: Manual
                    if let Some(p) = manual_opt {
                        ui.selectable_value(
                            &mut app.db_manual_path,
                            Some(p.clone()),
                            format!("📄 {}", p.file_name().unwrap().to_string_lossy()),
                        );
                    }

                    // Option B: Scanned List
                    for (i, file) in files_clone.iter().enumerate() {
                        let name = file.file_name().unwrap().to_string_lossy();
                        // Is selected if manual is None AND index matches
                        let is_selected = app.db_manual_path.is_none() && sel_idx == i;

                        if ui.selectable_label(is_selected, name).clicked() {
                            app.db_selected_idx = i;
                            app.db_manual_path = None;
                        }
                    }
                });
            });
        },
    );

    // Handle Refresh
    if refresh_clicked {
        if app.db_mode_exe {
            app.db_files = utils::scan_dist_files();
        } else {
            app.db_files = utils::scan_bin_files();
        }
    }

    ui.add_space(20.0);

    // ACTION BUTTON
    let has_file = app.db_manual_path.is_some() || !app.db_files.is_empty();

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // [FIX] Use add_enabled_ui wrapper
            ui.add_enabled_ui(!app.is_busy && has_file, |ui| {
                if theme::styled_button(ui, "🐞 Launch Debugger", theme::ButtonVariant::Primary)
                    .clicked()
                {
                    start_debug(app);
                }
            });
        });
    });
}

fn start_debug(app: &mut PsApp) {
    app.is_busy = true;
    let tx = app.status_tx.clone();
    let target_path = app
        .db_manual_path
        .clone()
        .unwrap_or_else(|| app.db_files[app.db_selected_idx].clone());

    thread::spawn(move || {
        tx.send(format!("Debugging: {:?}", target_path.file_name().unwrap()))
            .ok();
        let (internal_tx, internal_rx) = std::sync::mpsc::channel();
        let _ = debugger::run_async(target_path, internal_tx);

        while let Ok(status) = internal_rx.recv() {
            match status {
                debugger::DebugStatus::Starting => {
                    tx.send("Debugger Window Opening...".into()).ok();
                }
                debugger::DebugStatus::Playing { frame, rect_count } => {
                    if frame % 60 == 0 {
                        tx.send(format!("Playing Frame: {} ({} rects)", frame, rect_count))
                            .ok();
                    }
                }
                debugger::DebugStatus::Finished => {
                    tx.send("Debug Session Finished.".into()).ok();
                    break;
                }
                debugger::DebugStatus::Error(e) => {
                    tx.send(format!("Error: {}", e)).ok();
                    break;
                }
            }
        }
    });
}
