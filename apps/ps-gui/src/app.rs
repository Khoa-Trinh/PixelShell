use crate::{tabs, theme, utils};
use eframe::egui;
use ps_factory::builder;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(PartialEq)]
pub enum Tab {
    Download,
    Convert,
    Build,
    Runner,
    Debug,
}

pub struct PsApp {
    pub current_tab: Tab,
    pub theme_preference: theme::Theme,

    pub logs: Vec<String>,
    pub status_rx: Receiver<String>,
    pub status_tx: Sender<String>,

    pub is_busy: bool,
    pub progress: f32,
    pub current_task: String,

    // Inputs (Download)
    pub dl_url: String,
    pub dl_res: String,
    pub dl_fps: String,
    pub dl_project: String,

    // Inputs (Convert)
    pub cv_gpu: bool,
    pub cv_res_720: bool,
    pub cv_res_1080: bool,
    pub cv_res_1440: bool,
    pub cv_res_2160: bool,
    pub cv_files: Vec<PathBuf>,
    pub cv_selected_idx: usize,
    pub cv_manual_path: Option<PathBuf>,

    // Inputs (Build)
    pub bd_targets: Vec<builder::BuildTarget>,
    pub bd_selected_idx: usize,
    pub bd_manual_path: Option<PathBuf>,

    // Inputs (Runner)
    pub rn_files: Vec<PathBuf>,
    pub rn_selected_idx: usize,
    pub rn_manual_path: Option<PathBuf>,
    pub rn_detach: bool,

    // Inputs (Debug)
    pub db_mode_exe: bool,
    pub db_files: Vec<PathBuf>,
    pub db_selected_idx: usize,
    pub db_manual_path: Option<PathBuf>,
}

impl Default for PsApp {
    fn default() -> Self {
        let (tx, rx) = channel::<String>();

        Self {
            current_tab: Tab::Download,
            theme_preference: theme::Theme::Dark, // Default to Dark

            logs: vec!["Ready.".into()],
            status_rx: rx,
            status_tx: tx,
            is_busy: false,
            progress: 0.0,
            current_task: "Idle".into(),

            dl_url: "".into(),
            dl_res: "1080p".into(),
            dl_fps: "30".into(),
            dl_project: "new_project".into(),

            cv_gpu: false,
            cv_res_720: false,
            cv_res_1080: true,
            cv_res_1440: false,
            cv_res_2160: false,
            cv_files: utils::scan_video_files(),
            cv_selected_idx: 0,
            cv_manual_path: None,

            bd_targets: utils::scan_build_targets(),
            bd_selected_idx: 0,
            bd_manual_path: None,

            rn_files: utils::scan_dist_files(),
            rn_selected_idx: 0,
            rn_manual_path: None,
            rn_detach: false,

            db_mode_exe: false,
            db_files: utils::scan_bin_files(),
            db_selected_idx: 0,
            db_manual_path: None,
        }
    }
}

impl eframe::App for PsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Apply Theme first
        let is_dark = match self.theme_preference {
            theme::Theme::Dark => true,
            theme::Theme::Light => false,
            theme::Theme::System => {
                // Query eframe for the system theme. Default to Dark if unknown.
                match _frame.info().system_theme {
                    Some(eframe::Theme::Light) => false,
                    _ => true,
                }
            }
        };

        theme::apply_settings(ctx, is_dark);

        // 2. Poll Messages
        self.handle_messages();

        // 3. Draw Layout
        // ORDER MATTERS: Side -> Bottom -> Central (Fill)

        self.render_sidebar(ctx);
        self.render_bottom_panel(ctx); // <--- FIX: Moved BEFORE render_content
        self.render_content(ctx);

        if self.is_busy {
            ctx.request_repaint();
        }
    }
}

impl PsApp {
    fn handle_messages(&mut self) {
        while let Ok(msg) = self.status_rx.try_recv() {
            if let Some(pct_str) = utils::extract_percentage(&msg) {
                self.progress = pct_str / 100.0;

                // Smart label update
                let new_label = if msg.contains("[Downloading]") {
                    Some("Downloading...")
                } else if msg.contains("[Processing]") {
                    Some("Processing...")
                } else if msg.contains("[Extracting]") {
                    Some("Extracting Audio...")
                } else {
                    None
                };

                if let Some(lbl) = new_label {
                    if self.current_task != lbl {
                        self.current_task = lbl.into();
                        self.logs.push(format!(">> {}", lbl));
                    }
                }
            } else {
                // AUTO-NAV LOGIC
                if msg.contains("Download Complete") {
                    self.is_busy = false;
                    self.current_task = "Done".into();
                    self.logs
                        .push("Download finished. Switching to Convert tab.".into());

                    // Auto-Switch
                    self.current_tab = Tab::Convert;
                    self.cv_files = utils::scan_video_files(); // Refresh Convert list
                } else if msg.contains("Conversion Batch Complete") {
                    self.is_busy = false;
                    self.current_task = "Done".into();
                    self.logs
                        .push("Conversion finished. Switching to Build tab.".into());

                    // Auto-Switch
                    self.current_tab = Tab::Build;
                    self.bd_targets = utils::scan_build_targets(); // Refresh Build list
                } else if msg.contains("Build Finished") {
                    self.is_busy = false;
                    self.current_task = "Done".into();
                    self.logs
                        .push("Build finished. Switching to Runner tab.".into());

                    // Auto-Switch
                    self.current_tab = Tab::Runner;
                    self.rn_files = utils::scan_dist_files(); // Refresh Runner list
                } else if msg.contains("Success")
                    || msg.contains("Done")
                    || msg.contains("Finished")
                {
                    self.is_busy = false;
                    self.progress = 1.0;
                    self.current_task = "Done".into();
                } else if msg.contains("Starting") {
                    self.progress = 0.0;
                    self.current_task = "Working...".into();
                } else if msg.contains("Error") {
                    self.is_busy = false;
                    self.current_task = "Error".into();
                }

                if !msg.starts_with("[") && self.current_task != msg {
                    self.logs.push(msg);
                }
            }
        }
    }

    fn render_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .exact_width(220.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("PIXEL SHELL")
                            .strong()
                            .size(18.0)
                            .color(theme::ACCENT),
                    );
                    ui.label(egui::RichText::new("FACTORY").weak().size(10.0));
                });
                ui.add_space(30.0);

                let nav_btn = |ui: &mut egui::Ui, label: &str, tab: Tab, current: &Tab| {
                    let selected = *current == tab;
                    let text = if selected {
                        egui::RichText::new(label)
                            .strong()
                            .color(egui::Color32::WHITE)
                    } else {
                        egui::RichText::new(label)
                    };

                    let btn = egui::Button::new(text)
                        .min_size(egui::vec2(ui.available_width(), 40.0))
                        .rounding(6.0)
                        .fill(if selected {
                            theme::ACCENT
                        } else {
                            egui::Color32::TRANSPARENT
                        });

                    if ui.add(btn).clicked() {
                        return Some(tab);
                    }
                    None
                };

                if let Some(t) = nav_btn(ui, "⬇  Download", Tab::Download, &self.current_tab) {
                    self.current_tab = t;
                }
                if let Some(t) = nav_btn(ui, "🔄  Convert", Tab::Convert, &self.current_tab) {
                    self.current_tab = t;
                }
                if let Some(t) = nav_btn(ui, "📦  Build", Tab::Build, &self.current_tab) {
                    self.current_tab = t;
                }
                if let Some(t) = nav_btn(ui, "🏃  Runner", Tab::Runner, &self.current_tab) {
                    self.current_tab = t;
                }
                if let Some(t) = nav_btn(ui, "🐞  Debugger", Tab::Debug, &self.current_tab) {
                    self.current_tab = t;
                }

                // THEME SELECTOR
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(20.0);

                    // [FIX] Use new theme::combo_box component
                    let theme_text = match self.theme_preference {
                        theme::Theme::Light => "☀ Light",
                        theme::Theme::Dark => "🌙 Dark",
                        theme::Theme::System => "💻 System",
                    };

                    theme::combo_box(ui, "theme_select", theme_text, |ui| {
                        ui.selectable_value(
                            &mut self.theme_preference,
                            theme::Theme::Light,
                            "☀ Light",
                        );
                        ui.selectable_value(
                            &mut self.theme_preference,
                            theme::Theme::Dark,
                            "🌙 Dark",
                        );
                        ui.selectable_value(
                            &mut self.theme_preference,
                            theme::Theme::System,
                            "💻 System",
                        );
                    });

                    ui.add_space(5.0);
                    ui.separator();
                });
            });
    }

    fn render_bottom_panel(&mut self, ctx: &egui::Context) {
        // 1. Calculate style BEFORE creating the panel
        let is_dark = ctx.style().visuals.dark_mode;
        let bg = if is_dark {
            egui::Color32::from_rgb(20, 20, 23)
        } else {
            egui::Color32::from_rgb(240, 240, 245)
        };

        // 2. Define the Frame for the panel
        // This ensures the background color fills the ENTIRE panel, including margins.
        let panel_frame = egui::Frame::none().fill(bg).inner_margin(15.0);

        // 3. Create Panel with .frame()
        egui::TopBottomPanel::bottom("bottom_bar")
            .resizable(false)
            .min_height(140.0)
            .frame(panel_frame) // <--- FIX: Apply frame here
            .show(ctx, |ui| {
                // 1. Header (Status)
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("TERMINAL").strong().small());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.is_busy {
                            ui.spinner();
                            ui.label(
                                egui::RichText::new(&self.current_task)
                                    .strong()
                                    .color(theme::ACCENT),
                            );
                        } else {
                            let status_color = if self.current_task == "Error" {
                                egui::Color32::RED
                            } else {
                                egui::Color32::from_gray(100)
                            };
                            ui.label(
                                egui::RichText::new(&self.current_task)
                                    .strong()
                                    .color(status_color),
                            );
                        }
                    });
                });

                ui.add_space(5.0);
                ui.separator();
                ui.add_space(5.0);

                // 2. Console Output (FIXED HEIGHT)
                let console_height = 100.0;

                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .max_height(console_height)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for log in self.logs.iter().rev().take(100).rev() {
                            ui.label(
                                egui::RichText::new(log)
                                    .font(egui::FontId::monospace(12.0))
                                    .color(ui.visuals().weak_text_color()),
                            );
                        }
                    });

                // 3. Progress Bar (BELOW Console, Hidden when idle)
                if self.is_busy || self.progress > 0.0 {
                    ui.add_space(10.0);

                    let bar_height = 6.0;
                    let rounding = 3.0;

                    let (rect, _response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), bar_height),
                        egui::Sense::hover(),
                    );

                    ui.painter().rect_filled(
                        rect,
                        rounding,
                        if is_dark {
                            egui::Color32::from_gray(40)
                        } else {
                            egui::Color32::from_gray(200)
                        },
                    );

                    if self.progress > 0.0 {
                        let fill_width = rect.width() * self.progress;
                        let fill_rect =
                            egui::Rect::from_min_size(rect.min, egui::vec2(fill_width, bar_height));
                        ui.painter().rect_filled(fill_rect, rounding, theme::ACCENT);
                    }
                }
            });
    }

    fn render_content(&mut self, ctx: &egui::Context) {
        // CentralPanel fills the REMAINING space (after Side and Bottom panels)
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Frame::none()
                    .inner_margin(egui::Margin {
                        left: 5.0,
                        right: 17.5, // <--- Padding to prevent scrollbar overlap
                        top: 17.5,
                        bottom: 1.0,
                    })
                    .show(ui, |ui| match self.current_tab {
                        Tab::Download => tabs::download::show(self, ui),
                        Tab::Convert => tabs::convert::show(self, ui),
                        Tab::Build => tabs::build::show(self, ui),
                        Tab::Runner => tabs::run::show(self, ui),
                        Tab::Debug => tabs::debug::show(self, ui),
                    });
            });
        });
    }
}
