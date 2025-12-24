// use anyhow::{bail, Context, Result};
// use byteorder::{LittleEndian, ReadBytesExt};
// use dialoguer::{theme::ColorfulTheme, Select};
// use minifb::{Key, Window, WindowOptions};
// use std::{
//     env,
//     fs::{self, File},
//     io::{BufReader, Read, Seek, SeekFrom},
//     mem,
//     path::PathBuf,
//     sync::mpsc::Sender,
//     thread,
//     time::{Duration, Instant},
// };

// // ==================================================================================
// // 1. DATA STRUCTURES
// // ==================================================================================

// #[derive(Debug, Clone)]
// pub struct DebugJob {
//     pub file_path: PathBuf,
// }

// #[derive(Debug, Clone)]
// pub enum DebugStatus {
//     Starting,
//     Playing { frame: usize, rect_count: usize },
//     Finished,
//     Error(String),
// }

// pub struct DebugArgs {
//     pub project_name: Option<String>,
//     pub file_name: Option<String>,
// }

// // Struct matching the one in Builder/Runner
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

// // ==================================================================================
// // 2. CORE LOGIC
// // ==================================================================================

// pub fn process_debug_session<F>(job: DebugJob, callback: F) -> Result<()>
// where
//     F: Fn(DebugStatus),
// {
//     callback(DebugStatus::Starting);

//     let mut f = File::open(&job.file_path).context("Failed to open file")?;

//     // DETECT MODE: .bin (Raw) vs .exe (Container)
//     let file_size = f.metadata()?.len();
//     let is_exe = job.file_path.extension().map_or(false, |e| e == "exe");

//     let (mut reader, width, height, fps) = if is_exe {
//         // --- EXE MODE: READ FOOTER ---
//         if file_size < mem::size_of::<PayloadFooter>() as u64 {
//             bail!("File too small to be a valid Pixel Shell EXE");
//         }

//         f.seek(SeekFrom::End(-(mem::size_of::<PayloadFooter>() as i64)))?;
//         let mut footer_buf = [0u8; mem::size_of::<PayloadFooter>()];
//         f.read_exact(&mut footer_buf)?;

//         let footer: PayloadFooter = unsafe { std::mem::transmute(footer_buf) };

//         if &footer.magic != b"PS_PATCH" {
//             bail!("Invalid EXE: Magic signature 'PS_PATCH' not found.");
//         }

//         // Seek to Video Data
//         f.seek(SeekFrom::Start(footer.video_offset))?;

//         // Read FPS Header from the video blob
//         let fps = f.read_u16::<LittleEndian>()?;

//         (
//             BufReader::new(f),
//             footer.width as usize,
//             footer.height as usize,
//             fps,
//         )
//     } else {
//         // --- BIN MODE: RAW READ ---
//         let fps = f.read_u16::<LittleEndian>()?;
//         (BufReader::new(f), 1920, 1080, fps) // Default res for raw bin if unknown
//     };

//     // SETUP WINDOW
//     let mut window = Window::new(
//         &format!("Debug View - {} FPS ({}x{})", fps, width, height),
//         width,
//         height,
//         WindowOptions {
//             resize: true,
//             ..WindowOptions::default()
//         },
//     )
//     .context("Unable to create debug window")?;

//     let mut buffer: Vec<u32> = vec![0; width * height];
//     let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
//     let mut frame_idx = 0;
//     let mut rect_buf = [0u8; 8];

//     // RENDER LOOP
//     while window.is_open() && !window.is_key_down(Key::Escape) {
//         let start_time = Instant::now();
//         buffer.fill(0xFF000000);
//         let mut rect_count = 0;

//         loop {
//             if reader.read_exact(&mut rect_buf).is_err() {
//                 callback(DebugStatus::Finished);
//                 return Ok(());
//             }

//             let x = u16::from_le_bytes(rect_buf[0..2].try_into().unwrap());
//             let y = u16::from_le_bytes(rect_buf[2..4].try_into().unwrap());
//             let w = u16::from_le_bytes(rect_buf[4..6].try_into().unwrap());
//             let h = u16::from_le_bytes(rect_buf[6..8].try_into().unwrap());

//             if w == 0 && h == 0 {
//                 break;
//             } // Frame End

//             rect_count += 1;
//             draw_rect(
//                 &mut buffer,
//                 width,
//                 height,
//                 x as usize,
//                 y as usize,
//                 w as usize,
//                 h as usize,
//             );
//         }

//         if frame_idx % fps as usize == 0 {
//             callback(DebugStatus::Playing {
//                 frame: frame_idx,
//                 rect_count,
//             });
//         }

//         window.update_with_buffer(&buffer, width, height)?;
//         frame_idx += 1;

//         let elapsed = start_time.elapsed();
//         if elapsed < frame_duration {
//             thread::sleep(frame_duration - elapsed);
//         }
//     }

//     callback(DebugStatus::Finished);
//     Ok(())
// }

// fn draw_rect(
//     buffer: &mut [u32],
//     screen_w: usize,
//     screen_h: usize,
//     x: usize,
//     y: usize,
//     w: usize,
//     h: usize,
// ) {
//     let right = (x + w).min(screen_w);
//     let bottom = (y + h).min(screen_h);
//     // Bounds check to prevent crashes on bad data
//     if x >= screen_w || y >= screen_h {
//         return;
//     }

//     for r in y..bottom {
//         let row_start = r * screen_w;
//         buffer[row_start + x..row_start + right].fill(0xFFFFFFFF);
//     }
// }

// // ==================================================================================
// // 3. CLI HANDLER
// // ==================================================================================

// pub fn run_cli(_args: DebugArgs) -> Result<()> {
//     let current_exe = env::current_exe()?;
//     let exe_dir = current_exe.parent().unwrap();
//     let assets_dir = exe_dir.join("assets");
//     let dist_dir = exe_dir.join("dist");

//     // 1. Select Mode: Check Project (Bin) OR Check Build (Exe)
//     let modes = vec!["Inspect Project (.bin)", "Inspect Build (.exe)"];
//     let mode_idx = Select::with_theme(&ColorfulTheme::default())
//         .with_prompt("Select Debug Mode")
//         .items(&modes)
//         .default(0)
//         .interact()?;

//     let target_path = if mode_idx == 0 {
//         // --- SELECT .BIN ---
//         if !assets_dir.exists() {
//             bail!("Assets folder missing.");
//         }

//         // Pick Project
//         let projects: Vec<_> = fs::read_dir(&assets_dir)?.filter_map(|e| e.ok()).collect();
//         let p_names: Vec<String> = projects
//             .iter()
//             .map(|p| p.file_name().to_string_lossy().into())
//             .collect();
//         let p_idx = Select::with_theme(&ColorfulTheme::default())
//             .with_prompt("Select Project")
//             .items(&p_names)
//             .interact()?;
//         let p_path = projects[p_idx].path();

//         // Pick File
//         let bins: Vec<_> = fs::read_dir(&p_path)?
//             .filter_map(|e| e.ok())
//             .filter(|e| e.path().extension().map_or(false, |ext| ext == "bin"))
//             .collect();
//         let b_names: Vec<String> = bins
//             .iter()
//             .map(|b| b.file_name().to_string_lossy().into())
//             .collect();
//         let b_idx = Select::with_theme(&ColorfulTheme::default())
//             .with_prompt("Select File")
//             .items(&b_names)
//             .interact()?;
//         bins[b_idx].path()
//     } else {
//         // --- SELECT .EXE ---
//         if !dist_dir.exists() {
//             bail!("Dist folder missing. Build something first.");
//         }

//         let exes: Vec<_> = fs::read_dir(&dist_dir)?
//             .filter_map(|e| e.ok())
//             .filter(|e| e.path().extension().map_or(false, |ext| ext == "exe"))
//             .collect();

//         if exes.is_empty() {
//             bail!("No executables found in dist.");
//         }

//         let e_names: Vec<String> = exes
//             .iter()
//             .map(|e| e.file_name().to_string_lossy().into())
//             .collect();
//         let e_idx = Select::with_theme(&ColorfulTheme::default())
//             .with_prompt("Select Executable")
//             .items(&e_names)
//             .interact()?;
//         exes[e_idx].path()
//     };

//     println!("Debugging: {:?}", target_path);
//     let job = DebugJob {
//         file_path: target_path,
//     };

//     process_debug_session(job, |status| {
//         if let DebugStatus::Error(e) = status {
//             eprintln!("Error: {}", e);
//         }
//     })?;

//     Ok(())
// }

// // ==================================================================================
// // 4. ASYNC HANDLER
// // ==================================================================================

// pub fn run_async(file_path: PathBuf, sender: Sender<DebugStatus>) -> Result<()> {
//     thread::spawn(move || {
//         let tx = sender.clone();
//         let job = DebugJob { file_path };
//         let result = process_debug_session(job, move |status| {
//             let _ = tx.send(status);
//         });
//         if let Err(e) = result {
//             let _ = sender.send(DebugStatus::Error(e.to_string()));
//         }
//     });
//     Ok(())
// }
