// use anyhow::{bail, Context, Result};
// use dialoguer::{theme::ColorfulTheme, MultiSelect};
// use std::{
//     env,
//     fs::{self, File},
//     io::Write,
//     mem,
//     path::{Path, PathBuf},
//     slice,
//     sync::mpsc::Sender,
//     thread,
// };

// // ==================================================================================
// // 1. DATA STRUCTURES (Shared)
// // ==================================================================================

// /// The "Magic Footer" that the Runner looks for at the end of the file.
// /// MUST match the struct definition in `ps-runner`.
// #[repr(C)]
// #[derive(Debug, Clone, Copy)]
// struct PayloadFooter {
//     video_offset: u64,
//     video_len: u64,
//     audio_offset: u64,
//     audio_len: u64,
//     width: u16,
//     height: u16,
//     magic: [u8; 8],
// }

// #[derive(Debug, Clone)]
// pub struct BuildTarget {
//     pub project: String,
//     pub resolution: String,
//     pub width: u16,
//     pub height: u16,
//     pub bin_path: PathBuf,
//     pub audio_path: PathBuf,
// }

// #[derive(Debug, Clone)]
// pub enum BuildStatus {
//     Starting,
//     Building(String),  // "Building my_project_1080p.exe..."
//     Finished(PathBuf), // Returns path to the new .exe
//     Error(String),
// }

// pub struct BuildArgs {
//     pub project_name: Option<String>,
//     pub resolutions: Option<String>,
//     pub build_all: bool,
// }

// // ==================================================================================
// // 2. CORE LOGIC (Pure Function)
// // ==================================================================================

// /// Reads the template, appends data, and writes the final standalone EXE.
// pub fn build_single_target(
//     target: &BuildTarget,
//     template_path: &Path,
//     output_dir: &Path,
// ) -> Result<PathBuf> {
//     // 1. Validation
//     if !template_path.exists() {
//         bail!("Template not found at {:?}", template_path);
//     }

//     let exe_name = format!("{}_{}.exe", target.project, target.resolution);
//     let output_path = output_dir.join(&exe_name);

//     // 2. Load Data
//     // We read these into memory. For massive files, we might stream them,
//     // but for <100MB overlays, memory is fine and faster.
//     let template_bytes = fs::read(template_path).context("Failed to read template exe")?;
//     let video_data = fs::read(&target.bin_path).context("Failed to read video bin")?;
//     let audio_data = fs::read(&target.audio_path).context("Failed to read audio ogg")?;

//     // 3. Calculate Offsets
//     let template_len = template_bytes.len() as u64;
//     let video_len = video_data.len() as u64;
//     let audio_len = audio_data.len() as u64;

//     // Layout: [Template EXE] + [Video Data] + [Audio Data] + [Footer]
//     let video_offset = template_len;
//     let audio_offset = template_len + video_len;

//     let footer = PayloadFooter {
//         video_offset,
//         video_len,
//         audio_offset,
//         audio_len,
//         width: target.width,
//         height: target.height,
//         magic: *b"PS_PATCH", // Magic bytes to verify integrity
//     };

//     // 4. Write Output File
//     let mut file = File::create(&output_path).context("Failed to create output file")?;

//     file.write_all(&template_bytes)?;
//     file.write_all(&video_data)?;
//     file.write_all(&audio_data)?;

//     // Serialize Footer (Unsafe transmutation to bytes)
//     let footer_bytes = unsafe {
//         slice::from_raw_parts(
//             &footer as *const PayloadFooter as *const u8,
//             mem::size_of::<PayloadFooter>(),
//         )
//     };
//     file.write_all(footer_bytes)?;

//     Ok(output_path)
// }

// // ==================================================================================
// // 3. CLI HANDLER (Interactive)
// // ==================================================================================

// pub fn run_cli(args: BuildArgs) -> Result<()> {
//     // 1. Setup Paths
//     let current_exe = env::current_exe().context("Failed to get exe path")?;
//     let exe_dir = current_exe
//         .parent()
//         .context("Failed to get exe directory")?;
//     let assets_dir = exe_dir.join("assets");
//     let dist_dir = exe_dir.join("dist");
//     let template_path = exe_dir.join("ps-runner.exe");

//     if !template_path.exists() {
//         bail!("❌ Missing Template!\nCould not find 'ps-runner.exe' at:\n{:?}\n\nPlease build ps-runner first.", template_path);
//     }

//     // 2. Discover Targets
//     let all_targets = get_available_builds(&assets_dir)?;
//     if all_targets.is_empty() {
//         bail!("No assets found. Run 'convert' first.");
//     }

//     // 3. Filter / Select Targets
//     let selected_targets: Vec<BuildTarget> =
//         if args.build_all || args.project_name.is_some() || args.resolutions.is_some() {
//             // Flag Mode
//             println!("Filtering targets based on flags...");
//             let req_res: Option<Vec<String>> = args
//                 .resolutions
//                 .as_ref()
//                 .map(|s| s.split(',').map(|r| r.trim().to_string()).collect());

//             all_targets
//                 .into_iter()
//                 .filter(|t| {
//                     if let Some(p) = &args.project_name {
//                         if &t.project != p {
//                             return false;
//                         }
//                     }
//                     if let Some(r_list) = &req_res {
//                         if !r_list.contains(&t.resolution) {
//                             return false;
//                         }
//                     }
//                     true
//                 })
//                 .collect()
//         } else {
//             // Interactive Mode
//             let options: Vec<String> = all_targets
//                 .iter()
//                 .map(|t| format!("{} @ {}", t.project, t.resolution))
//                 .collect();

//             let selection = MultiSelect::with_theme(&ColorfulTheme::default())
//                 .with_prompt("Select Executables to Build")
//                 .items(&options)
//                 .interact()?;

//             if selection.is_empty() {
//                 println!("No targets selected.");
//                 return Ok(());
//             }

//             selection
//                 .into_iter()
//                 .map(|i| all_targets[i].clone())
//                 .collect()
//         };

//     if selected_targets.is_empty() {
//         bail!("No matching targets found.");
//     }

//     // 4. Execution Loop
//     fs::create_dir_all(&dist_dir)?;
//     println!("📂 Output directory: {:?}", dist_dir);

//     for target in selected_targets {
//         print!("📦 Building {}... ", target.project);
//         // Flush stdout to show the text before the heavy work starts
//         let _ = std::io::stdout().flush();

//         match build_single_target(&target, &template_path, &dist_dir) {
//             Ok(path) => println!("✅ Created: {:?}", path.file_name().unwrap()),
//             Err(e) => println!("❌ Failed: {}", e),
//         }
//     }

//     println!("\n✨ All tasks complete.");
//     Ok(())
// }

// // ==================================================================================
// // 4. GUI / ASYNC HANDLER (Bridge)
// // ==================================================================================

// pub fn run_async(targets: Vec<BuildTarget>, sender: Sender<BuildStatus>) -> Result<()> {
//     // 1. Locate Template (Assume standard layout relative to factory exe)
//     let current_exe = env::current_exe()?;
//     let exe_dir = current_exe.parent().unwrap();
//     let template_path = exe_dir.join("ps-runner.exe");
//     let dist_dir = exe_dir.join("dist");

//     if !template_path.exists() {
//         let _ = sender.send(BuildStatus::Error("Missing ps-runner.exe template".into()));
//         return Ok(());
//     }

//     fs::create_dir_all(&dist_dir)?;

//     thread::spawn(move || {
//         let _ = sender.send(BuildStatus::Starting);

//         for target in targets {
//             let name = format!("{}_{}", target.project, target.resolution);
//             let _ = sender.send(BuildStatus::Building(name));

//             match build_single_target(&target, &template_path, &dist_dir) {
//                 Ok(path) => {
//                     let _ = sender.send(BuildStatus::Finished(path));
//                 }
//                 Err(e) => {
//                     let _ = sender.send(BuildStatus::Error(e.to_string()));
//                 }
//             }
//         }
//     });

//     Ok(())
// }

// // ==================================================================================
// // 5. HELPER FUNCTIONS
// // ==================================================================================

// pub fn get_available_builds(assets_dir: &Path) -> Result<Vec<BuildTarget>> {
//     if !assets_dir.exists() {
//         return Ok(vec![]);
//     }
//     let mut targets = Vec::new();
//     let resolutions = vec![
//         ("720p", 1280, 720),
//         ("1080p", 1920, 1080),
//         ("1440p", 2560, 1440),
//         ("2160p", 3840, 2160),
//     ];

//     for entry in fs::read_dir(assets_dir)? {
//         let entry = entry?;
//         if entry.path().is_dir() {
//             let project_name = entry.file_name().to_string_lossy().to_string();
//             let project_dir = entry.path();
//             let audio_path = project_dir.join(format!("{}.ogg", project_name));

//             // Audio is required
//             if !audio_path.exists() {
//                 continue;
//             }

//             for (res_name, w, h) in &resolutions {
//                 let bin_name = format!("{}_{}.bin", project_name, res_name);
//                 let bin_path = project_dir.join(&bin_name);

//                 if bin_path.exists() {
//                     targets.push(BuildTarget {
//                         project: project_name.clone(),
//                         resolution: res_name.to_string(),
//                         width: *w,
//                         height: *h,
//                         bin_path: fs::canonicalize(&bin_path)?,
//                         audio_path: fs::canonicalize(&audio_path)?,
//                     });
//                 }
//             }
//         }
//     }
//     Ok(targets)
// }
