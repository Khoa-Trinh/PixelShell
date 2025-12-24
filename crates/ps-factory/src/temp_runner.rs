// use anyhow::{bail, Context, Result};
// use dialoguer::{theme::ColorfulTheme, Select};
// use std::{
//     env, fs,
//     path::{Path, PathBuf},
//     process::Command,
//     sync::mpsc::Sender,
//     thread,
//     time::Duration,
// };

// // ==================================================================================
// // 1. DATA STRUCTURES (Shared)
// // ==================================================================================

// #[derive(Debug, Clone, PartialEq)]
// pub enum RunnerMode {
//     Watchdog, // Keeps restarting the process (Normal/Silent)
//     Detach,   // Starts process and exits (Fire & Forget)
// }

// #[derive(Debug, Clone)]
// pub enum RunnerStatus {
//     Starting(String),
//     Running(u32), // PID
//     Restarting,
//     Detached,
//     Error(String),
// }

// pub struct RunArgs {
//     pub target: Option<String>,
//     pub silent: bool,
//     pub detach: bool,
// }

// // ==================================================================================
// // 2. CORE LOGIC (Process Management)
// // ==================================================================================

// /// Manages the lifecycle of the target executable.
// pub fn process_runner<F>(exe_path: &Path, mode: RunnerMode, callback: F) -> Result<()>
// where
//     F: Fn(RunnerStatus) + Send + Clone + 'static,
// {
//     let exe_name = exe_path.file_name().unwrap().to_string_lossy().to_string();
//     callback(RunnerStatus::Starting(exe_name.clone()));

//     match mode {
//         RunnerMode::Detach => {
//             Command::new(exe_path)
//                 .spawn()
//                 .context("Failed to detach process")?;
//             callback(RunnerStatus::Detached);
//             Ok(())
//         }
//         RunnerMode::Watchdog => {
//             // Infinite Loop
//             loop {
//                 let mut child = Command::new(exe_path)
//                     .spawn()
//                     .context("Failed to spawn process")?;

//                 callback(RunnerStatus::Running(child.id()));

//                 // Block until process exits
//                 let _ = child.wait();

//                 callback(RunnerStatus::Restarting);

//                 // Wait before restart to prevent CPU thrashing if immediate crash
//                 thread::sleep(Duration::from_secs(2));
//             }
//         }
//     }
// }

// // ==================================================================================
// // 3. CLI HANDLER (Interactive + Window Hiding)
// // ==================================================================================

// pub fn run_cli(args: RunArgs) -> Result<()> {
//     // 1. Setup Paths
//     let current_exe = env::current_exe()?;
//     let exe_dir = current_exe.parent().unwrap();
//     let dist_dir = exe_dir.join("dist");

//     // 2. Select Executable
//     let selected_path = resolve_target(&dist_dir, args.target.as_deref())?;

//     // 3. Determine Mode
//     let (mode, is_silent) = if args.detach {
//         (RunnerMode::Detach, false)
//     } else if args.silent {
//         (RunnerMode::Watchdog, true)
//     } else {
//         // Interactive Selection
//         let modes = vec![
//             "Normal (Watchdog visible)",
//             "Silent (Watchdog hidden, auto-restart)",
//             "Detach (Run once & exit CLI)",
//         ];
//         let selection = Select::with_theme(&ColorfulTheme::default())
//             .with_prompt("Select Run Mode")
//             .default(0)
//             .items(&modes)
//             .interact()?;

//         match selection {
//             0 => (RunnerMode::Watchdog, false),
//             1 => (RunnerMode::Watchdog, true),
//             2 => (RunnerMode::Detach, false),
//             _ => (RunnerMode::Watchdog, false),
//         }
//     };

//     // 4. Handle "Silent" Mode (CLI Specific)
//     // This logic belongs here, NOT in the core, because a GUI shouldn't hide itself.
//     if is_silent {
//         #[cfg(target_os = "windows")]
//         unsafe {
//             use windows::Win32::System::Console::GetConsoleWindow;
//             use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

//             let hwnd = GetConsoleWindow();
//             if hwnd.0 != 0 {
//                 println!("👻 Going Silent! (Check Task Manager to stop 'ps-cli')");
//                 thread::sleep(Duration::from_millis(500));
//                 ShowWindow(hwnd, SW_HIDE);
//             }
//         }
//     } else if mode == RunnerMode::Watchdog {
//         println!(
//             "✅ Watchdog Active for: {:?}",
//             selected_path.file_name().unwrap()
//         );
//         println!("   (Press Ctrl+C to stop)");
//     }

//     // 5. Run Logic
//     process_runner(&selected_path, mode, |status| match status {
//         RunnerStatus::Starting(name) => println!("🚀 Starting: {}", name),
//         RunnerStatus::Running(pid) => println!("   -> PID: {}", pid),
//         RunnerStatus::Restarting => println!("🔄 Process exited. Restarting in 2s..."),
//         RunnerStatus::Detached => println!("👋 Detached successfully."),
//         RunnerStatus::Error(e) => eprintln!("❌ Error: {}", e),
//     })?;

//     Ok(())
// }

// // ==================================================================================
// // 4. ASYNC HANDLER (GUI Bridge)
// // ==================================================================================

// pub fn run_async(target_name: Option<String>, sender: Sender<RunnerStatus>) -> Result<()> {
//     // 1. Resolve Path (Needs to replicate logic or reuse helper)
//     let current_exe = env::current_exe()?;
//     let exe_dir = current_exe.parent().unwrap();
//     let dist_dir = exe_dir.join("dist");

//     // Non-interactive resolution
//     let selected_path = match resolve_target(&dist_dir, target_name.as_deref()) {
//         Ok(p) => p,
//         Err(e) => {
//             let _ = sender.send(RunnerStatus::Error(e.to_string()));
//             return Ok(());
//         }
//     };

//     // 2. Spawn Background Thread
//     thread::spawn(move || {
//         let tx_callback = sender.clone();

//         // GUI usually implies "Watchdog" mode (keeping it alive)
//         // If you want "Detach", you'd call a different function.
//         let result = process_runner(&selected_path, RunnerMode::Watchdog, move |status| {
//             let _ = tx_callback.send(status);
//         });

//         if let Err(e) = result {
//             let _ = sender.send(RunnerStatus::Error(e.to_string()));
//         }
//     });

//     Ok(())
// }

// // ==================================================================================
// // 5. HELPER FUNCTIONS
// // ==================================================================================

// fn resolve_target(dist_dir: &Path, target_hint: Option<&str>) -> Result<PathBuf> {
//     if !dist_dir.exists() {
//         bail!("'dist' folder not found. Run 'build' first.");
//     }

//     let mut executables: Vec<(String, PathBuf)> = fs::read_dir(dist_dir)?
//         .filter_map(|e| e.ok())
//         .filter(|e| {
//             let p = e.path();
//             p.is_file() && (p.extension().map_or(false, |ext| ext == "exe") || cfg!(unix))
//         })
//         .map(|e| (e.file_name().to_string_lossy().to_string(), e.path()))
//         .collect();

//     executables.sort_by(|a, b| a.0.cmp(&b.0));

//     if executables.is_empty() {
//         bail!("No executables found in 'dist/'.");
//     }

//     if let Some(name) = target_hint {
//         let matches: Vec<&PathBuf> = executables
//             .iter()
//             .filter(|(n, _)| n.contains(name))
//             .map(|(_, p)| p)
//             .collect();

//         match matches.len() {
//             0 => bail!("No match for '{}'", name),
//             1 => Ok(matches[0].clone()),
//             _ => {
//                 // In CLI, we could ask. In logic, we must bail or pick first.
//                 // For safety in shared logic, we'll pick the first but warn.
//                 println!("Ambiguous match for '{}', picking: {:?}", name, matches[0]);
//                 Ok(matches[0].clone())
//             }
//         }
//     } else {
//         // Interactive Selection (Only valid if TTY, handled by CLI wrapper usually)
//         // If this helper is called by GUI with None, we can't interact.
//         // So we just return list or error?
//         // For CLI specifically:
//         let options: Vec<String> = executables.iter().map(|(n, _)| n.clone()).collect();
//         let idx = Select::with_theme(&ColorfulTheme::default())
//             .with_prompt("Select Executable")
//             .items(&options)
//             .default(0)
//             .interact()?;
//         Ok(executables[idx].1.clone())
//     }
// }
