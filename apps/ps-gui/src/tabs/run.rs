use crate::{app::PsApp, theme, utils};
use eframe::egui;
use ps_factory::runner;
use std::thread;

pub fn show(app: &mut PsApp, ui: &mut egui::Ui) {
    ui.heading("Runner");
    ui.label("Launch and manage your built Pixel Shell overlays.");
    ui.add_space(15.0);

    // CARD 1: TARGET SELECTION
    // Header Action: Refresh List
    let (_, refresh_clicked) = theme::card(
        ui,
        "1. Target Executable",
        Some(("🔄 Refresh", theme::ButtonVariant::Secondary)),
        |ui| {
            // -- Row 1: System Browse --
            ui.horizontal(|ui| {
                if ui.button("📂 Browse System...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Executable", &["exe"])
                        .pick_file()
                    {
                        app.rn_manual_path = Some(path);
                    }
                }

                if let Some(_path) = &app.rn_manual_path {
                    if ui.small_button("❌ Clear Selection").clicked() {
                        app.rn_manual_path = None;
                    }
                }
            });

            ui.add_space(5.0);

            // -- Logic: Determine Display Text --
            let selected_text = if let Some(p) = &app.rn_manual_path {
                format!("📄 {}", p.file_name().unwrap().to_string_lossy())
            } else if !app.rn_files.is_empty() {
                if app.rn_selected_idx >= app.rn_files.len() {
                    app.rn_selected_idx = 0;
                }
                format!(
                    "🚀 {}",
                    app.rn_files[app.rn_selected_idx]
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                )
            } else {
                "⚠️ No executables found".to_string()
            };

            // Clone for borrowing safety
            let manual_opt = app.rn_manual_path.clone();
            let files_clone = app.rn_files.clone();
            let sel_idx = app.rn_selected_idx;

            // -- Row 2: Dropdown --
            ui.horizontal(|ui| {
                ui.label("Selected App:");

                theme::combo_box(ui, "rn_target_dropdown", &selected_text, |ui| {
                    // Option A: Manual
                    if let Some(p) = manual_opt {
                        ui.selectable_value(
                            &mut app.rn_manual_path,
                            Some(p.clone()),
                            format!("📄 {}", p.file_name().unwrap().to_string_lossy()),
                        );
                    }

                    // Option B: Scanned List
                    for (i, file) in files_clone.iter().enumerate() {
                        let name = file.file_name().unwrap().to_string_lossy();
                        // Is selected if manual is None AND index matches
                        let is_selected = app.rn_manual_path.is_none() && sel_idx == i;

                        if ui.selectable_label(is_selected, name).clicked() {
                            app.rn_selected_idx = i;
                            app.rn_manual_path = None;
                        }
                    }
                });
            });
        },
    );

    if refresh_clicked {
        app.rn_files = utils::scan_dist_files();
    }

    ui.add_space(15.0);

    // CARD 2: OPTIONS
    theme::card(ui, "2. Run Options", None, |ui| {
        ui.checkbox(&mut app.rn_detach, "Detach Process (Fire & Forget)");
        ui.label(
            egui::RichText::new(
                "If checked, the GUI will stop monitoring the process immediately after launch.",
            )
            .small()
            .weak(),
        );
    });

    ui.add_space(20.0);

    // ACTION BUTTON
    let has_file = app.rn_manual_path.is_some() || !app.rn_files.is_empty();

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // [FIX] Use add_enabled_ui wrapper
            ui.add_enabled_ui(!app.is_busy && has_file, |ui| {
                if theme::styled_button(ui, "🚀 Launch Overlay", theme::ButtonVariant::Primary)
                    .clicked()
                {
                    start_runner(app);
                }
            });
        });
    });
}

fn start_runner(app: &mut PsApp) {
    app.is_busy = true;
    let tx = app.status_tx.clone();
    let target_path = app
        .rn_manual_path
        .clone()
        .unwrap_or_else(|| app.rn_files[app.rn_selected_idx].clone());
    let detach = app.rn_detach;

    thread::spawn(move || {
        tx.send(format!("Launching: {:?}", target_path.file_name().unwrap()))
            .ok();
        let mode = if detach {
            runner::RunnerMode::Detach
        } else {
            runner::RunnerMode::Watchdog
        };

        let (internal_tx, internal_rx) = std::sync::mpsc::channel();
        let tx_for_callback = internal_tx.clone();

        let _ = thread::spawn(move || {
            let res = runner::process_runner(&target_path, mode, move |status| {
                let _ = tx_for_callback.send(status);
            });
            if let Err(e) = res {
                let _ = internal_tx.send(runner::RunnerStatus::Error(e.to_string()));
            }
        });

        while let Ok(status) = internal_rx.recv() {
            match status {
                runner::RunnerStatus::Starting(name) => {
                    tx.send(format!("Starting: {}", name)).ok();
                }
                runner::RunnerStatus::Running(pid) => {
                    tx.send(format!("Running (PID: {})", pid)).ok();
                    if detach {
                        break;
                    }
                }
                runner::RunnerStatus::Restarting => {
                    tx.send("Process exited. Restarting...".into()).ok();
                }
                runner::RunnerStatus::Detached => {
                    tx.send("Process Detached Successfully.".into()).ok();
                    break;
                }
                runner::RunnerStatus::Error(e) => {
                    tx.send(format!("Error: {}", e)).ok();
                    break;
                }
            }
        }
        if detach {
            tx.send("Runner Finished.".into()).ok();
        }
    });
}
