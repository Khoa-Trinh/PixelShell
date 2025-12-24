// use anyhow::{bail, Context, Result};
// use crossbeam_channel::{bounded, Receiver, Sender};
// use dialoguer::{theme::ColorfulTheme, MultiSelect, Select};
// use indicatif::{ProgressBar, ProgressStyle};
// use ps_core::PixelRect;
// use rayon::iter::{ParallelBridge, ParallelIterator};
// use std::{
//     cell::RefCell,
//     collections::HashMap,
//     env,
//     fs::{self, File},
//     io::{BufWriter, Read, Write},
//     path::{Path, PathBuf},
//     process::{Command, Stdio},
//     thread,
//     time::Instant,
// };

// // ==================================================================================
// // 1. DATA STRUCTURES (Shared)
// // ==================================================================================

// /// Defines a single conversion task (One video -> One .bin file)
// #[derive(Debug, Clone)]
// pub struct ConvertJob {
//     pub input_path: PathBuf,
//     pub output_path: PathBuf,
//     pub width: u32,
//     pub height: u32,
//     pub fps: u16,
//     pub use_gpu: bool,
// }

// /// Status updates sent from the Core Logic to the CLI or GUI
// #[derive(Debug, Clone)]
// pub enum ConverterStatus {
//     Starting,
//     Analyzing(String), // "Detecting FPS..."
//     Processing {
//         current_frame: u64,
//         total_frames: u64,
//         fps_speed: f64, // Processing speed
//     },
//     Finished,
//     Error(String),
// }

// pub struct ConvertArgs {
//     pub project_name: Option<String>,
//     pub resolutions: Option<String>,
//     pub use_gpu: bool,
// }

// // Internal structures for the pipeline
// struct RawFrame {
//     id: u64,
//     data: Vec<u8>,
// }
// struct ProcessedFrame {
//     id: u64,
//     rects: Vec<PixelRect>,
//     recycled_buffer: Vec<u8>,
// }

// // Thread-local scratch buffer for Snowplow algorithm
// thread_local! {
//     static SCRATCH_BUFFER: RefCell<Vec<isize>> = RefCell::new(Vec::new());
// }

// // ==================================================================================
// // 2. CORE LOGIC (Interface Agnostic)
// // ==================================================================================

// /// The high-performance engine.
// /// Accepts a generic `callback` to report progress.
// pub fn process_conversion<F>(job: ConvertJob, callback: F) -> Result<()>
// where
//     F: Fn(ConverterStatus) + Send + Clone + 'static,
// {
//     callback(ConverterStatus::Starting);

//     // 1. Analyze Video
//     callback(ConverterStatus::Analyzing("Detecting Metadata...".into()));
//     let total_frames = get_frame_count(&job.input_path).unwrap_or(0);

//     // 2. FFmpeg Setup
//     let filters = format!(
//         "scale={w}:{h},format=gray,gblur=sigma=1.0:steps=1,eq=contrast=1000:saturation=0",
//         w = job.width,
//         h = job.height
//     );

//     let mut cmd = Command::new("ffmpeg");
//     if job.use_gpu {
//         cmd.arg("-hwaccel").arg("cuda");
//     }

//     cmd.arg("-i")
//         .arg(&job.input_path)
//         .arg("-vf")
//         .arg(filters)
//         .arg("-f")
//         .arg("rawvideo")
//         .arg("-pix_fmt")
//         .arg("gray")
//         .arg("-");

//     let mut child = cmd
//         .stdout(Stdio::piped())
//         .stderr(Stdio::null())
//         .spawn()
//         .context("Failed to spawn ffmpeg")?;

//     let mut stdout = child.stdout.take().context("Failed to open stdout")?;

//     // 3. Channel Setup
//     let queue_size = 64;
//     let (tx_raw, rx_raw): (Sender<RawFrame>, Receiver<RawFrame>) = bounded(queue_size);
//     let (tx_processed, rx_processed): (Sender<ProcessedFrame>, Receiver<ProcessedFrame>) =
//         bounded(queue_size);
//     let (tx_recycle, rx_recycle): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = bounded(queue_size);

//     let frame_size = (job.width * job.height) as usize;

//     // Pre-fill Recycle Bin
//     for _ in 0..queue_size {
//         let _ = tx_recycle.send(vec![0u8; frame_size]);
//     }

//     // 4. Writer Thread (Handles Disk I/O + Reporting)
//     let output_path = job.output_path.clone();
//     let cb_writer = callback.clone();
//     let fps = job.fps;

//     let write_handle = thread::spawn(move || -> Result<()> {
//         let mut file_out = BufWriter::with_capacity(4 * 1024 * 1024, File::create(output_path)?);
//         file_out.write_all(&fps.to_le_bytes())?;

//         let mut next_needed_id = 0;
//         let mut reorder_buffer: HashMap<u64, Vec<PixelRect>> = HashMap::new();

//         let start_time = Instant::now();
//         let mut last_report = Instant::now();

//         for frame in rx_processed {
//             reorder_buffer.insert(frame.id, frame.rects);
//             let _ = tx_recycle.send(frame.recycled_buffer); // Return buffer immediately

//             while let Some(rects) = reorder_buffer.remove(&next_needed_id) {
//                 // Bulk write rects (unsafe cast is virtually free)
//                 let rect_bytes = unsafe {
//                     std::slice::from_raw_parts(
//                         rects.as_ptr() as *const u8,
//                         rects.len() * std::mem::size_of::<PixelRect>(),
//                     )
//                 };
//                 file_out.write_all(rect_bytes)?;

//                 let eos = PixelRect::EOS_MARKER;
//                 let eos_bytes = unsafe {
//                     std::slice::from_raw_parts(
//                         &eos as *const PixelRect as *const u8,
//                         std::mem::size_of::<PixelRect>(),
//                     )
//                 };
//                 file_out.write_all(eos_bytes)?;

//                 next_needed_id += 1;

//                 // Report Progress (throttled to ~10 times/sec to save CPU)
//                 if next_needed_id % 30 == 0 || last_report.elapsed().as_millis() > 100 {
//                     let elapsed = start_time.elapsed().as_secs_f64();
//                     let speed = if elapsed > 0.0 {
//                         next_needed_id as f64 / elapsed
//                     } else {
//                         0.0
//                     };

//                     cb_writer(ConverterStatus::Processing {
//                         current_frame: next_needed_id,
//                         total_frames,
//                         fps_speed: speed,
//                     });
//                     last_report = Instant::now();
//                 }
//             }
//         }
//         cb_writer(ConverterStatus::Finished);
//         Ok(())
//     });

//     // 5. Reader Thread
//     thread::spawn(move || {
//         let mut frame_id = 0;
//         loop {
//             let mut buffer = rx_recycle.recv().unwrap_or_else(|_| vec![0u8; frame_size]);
//             if stdout.read_exact(&mut buffer).is_err() {
//                 break;
//             } // EOF
//             if tx_raw
//                 .send(RawFrame {
//                     id: frame_id,
//                     data: buffer,
//                 })
//                 .is_err()
//             {
//                 break;
//             }
//             frame_id += 1;
//         }
//     });

//     // 6. Parallel Compute (Main Thread Logic)
//     let width = job.width;
//     let height = job.height;

//     rx_raw.into_iter().par_bridge().for_each(|raw| {
//         SCRATCH_BUFFER.with(|cell| {
//             let mut indices = cell.borrow_mut();
//             if indices.len() != width as usize {
//                 *indices = vec![-1isize; width as usize];
//             }

//             let rects = extract_rects_optimized(&raw.data, width, height, 127, &mut indices);

//             let _ = tx_processed.send(ProcessedFrame {
//                 id: raw.id,
//                 rects,
//                 recycled_buffer: raw.data,
//             });
//         });
//     });

//     drop(tx_processed);
//     write_handle.join().expect("Writer panic")?;
//     Ok(())
// }

// // ==================================================================================
// // 3. CLI HANDLER (Interactive + Flags)
// // ==================================================================================

// pub fn run_cli(args: ConvertArgs) -> Result<()> {
//     // 1. Setup Environment
//     let current_exe = env::current_exe().context("Failed to get exe path")?;
//     let exe_dir = current_exe
//         .parent()
//         .context("Failed to get exe directory")?;
//     let assets_root = exe_dir.join("assets");

//     // 2. Select Project
//     let project_name = match args.project_name {
//         Some(name) => name,
//         None => {
//             if !assets_root.exists() {
//                 bail!("'assets' folder missing.");
//             }
//             let entries = fs::read_dir(&assets_root)?
//                 .filter_map(|e| e.ok())
//                 .filter(|e| e.path().is_dir())
//                 .map(|e| e.file_name().to_string_lossy().to_string())
//                 .collect::<Vec<_>>();
//             if entries.is_empty() {
//                 bail!("No projects found.");
//             }

//             let idx = Select::with_theme(&ColorfulTheme::default())
//                 .with_prompt("Select Project")
//                 .items(&entries)
//                 .default(0)
//                 .interact()?;
//             entries[idx].clone()
//         }
//     };

//     // 3. Select Resolutions
//     let resolution_names: Vec<String> = match args.resolutions {
//         Some(s) => s.split(',').map(|x| x.trim().to_string()).collect(),
//         None => {
//             let options = vec!["720p", "1080p", "1440p", "2160p"];
//             let defaults = vec![true, true, false, false];
//             let selections = MultiSelect::with_theme(&ColorfulTheme::default())
//                 .with_prompt("Select Output Resolutions")
//                 .items(&options)
//                 .defaults(&defaults)
//                 .interact()?;
//             if selections.is_empty() {
//                 bail!("Select at least one resolution.");
//             }
//             selections.iter().map(|&i| options[i].to_string()).collect()
//         }
//     };

//     // 4. Locate Source Video
//     let project_dir = assets_root.join(&project_name);
//     let vid_path = ["mkv", "mp4", "avi", "mov", "webm"]
//         .iter()
//         .map(|ext| project_dir.join(format!("{}.{}", project_name, ext)))
//         .find(|p| p.exists())
//         .context("No video found in project folder")?;

//     let fps = detect_fps(&vid_path).unwrap_or(30);
//     println!("Detected: {} FPS", fps);

//     // 5. Process Loop
//     for res_name in resolution_names {
//         let width = match res_name.as_str() {
//             "720p" => 1280,
//             "1080p" => 1920,
//             "1440p" => 2560,
//             "2160p" => 3840,
//             _ => continue,
//         };
//         let height = width * 9 / 16;
//         let out_path = project_dir.join(format!("{}_{}.bin", project_name, res_name));

//         println!("\n--- Processing {} ({}x{}) ---", res_name, width, height);

//         let job = ConvertJob {
//             input_path: vid_path.clone(),
//             output_path: out_path,
//             width,
//             height,
//             fps,
//             use_gpu: args.use_gpu,
//         };

//         // SETUP CLI PROGRESS BAR
//         let pb = ProgressBar::new(100);
//         pb.set_style(ProgressStyle::with_template(
//             "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({per_sec}) {eta} {msg}"
//         ).unwrap().progress_chars("#>-"));

//         // RUN LOGIC with CLI Callback
//         let pb_clone = pb.clone();
//         process_conversion(job, move |status| match status {
//             ConverterStatus::Starting => pb_clone.set_message("Starting..."),
//             ConverterStatus::Analyzing(_) => pb_clone.set_message("Analyzing..."),
//             ConverterStatus::Processing {
//                 current_frame,
//                 total_frames,
//                 ..
//             } => {
//                 pb_clone.set_length(total_frames);
//                 pb_clone.set_position(current_frame);
//             }
//             ConverterStatus::Finished => pb_clone.finish_with_message("Done!"),
//             ConverterStatus::Error(e) => pb_clone.abandon_with_message(format!("Error: {}", e)),
//         })?;
//     }

//     println!("\n--- ALL TASKS COMPLETE ---");
//     Ok(())
// }

// // ==================================================================================
// // 4. GUI / ASYNC HANDLER (Bridge)
// // ==================================================================================

// pub fn run_async(job: ConvertJob, sender: std::sync::mpsc::Sender<ConverterStatus>) -> Result<()> {
//     // We spawn a new thread so the GUI doesn't freeze
//     thread::spawn(move || {
//         // FIX: Create a CLONE for the callback to use
//         let tx_callback = sender.clone();

//         let result = process_conversion(job, move |status| {
//             // Use the CLONE inside the loop
//             let _ = tx_callback.send(status);
//         });

//         // If an error happens, we still have the ORIGINAL 'sender' to report it
//         if let Err(e) = result {
//             let _ = sender.send(ConverterStatus::Error(e.to_string()));
//         }
//     });
//     Ok(())
// }

// // ==================================================================================
// // 5. HELPER FUNCTIONS & ALGORITHMS
// // ==================================================================================

// // Unsafe Snowplow Algorithm (Kept exactly as optimized)
// fn extract_rects_optimized(
//     buffer: &[u8],
//     width: u32,
//     height: u32,
//     threshold: u8,
//     active_indices: &mut Vec<isize>,
// ) -> Vec<PixelRect> {
//     let w = width as usize;
//     let h = height as usize;
//     active_indices.fill(-1);
//     let mut boxes: Vec<PixelRect> = Vec::with_capacity(4000);

//     for y in 0..h {
//         let row_start = y * w;
//         let row = unsafe { buffer.get_unchecked(row_start..row_start + w) };
//         let mut x = 0;

//         while x < w {
//             let pixel = unsafe { *row.get_unchecked(x) };
//             if pixel < threshold {
//                 unsafe {
//                     let idx = active_indices.get_unchecked_mut(x);
//                     if *idx != -1 {
//                         *idx = -1;
//                     }
//                 }
//                 x += 1;
//                 continue;
//             }

//             let start_x = x;
//             while x < w {
//                 let p = unsafe { *row.get_unchecked(x) };
//                 if p < threshold {
//                     break;
//                 }
//                 x += 1;
//             }

//             let run_width = (x - start_x) as u16;
//             let current_start_x = start_x as u16;
//             let active_idx = unsafe { *active_indices.get_unchecked(start_x) };
//             let mut merged = false;

//             if active_idx != -1 {
//                 let idx = active_idx as usize;
//                 if idx < boxes.len() {
//                     let b = unsafe { boxes.get_unchecked_mut(idx) };
//                     if b.y + b.h == (y as u16) && b.x == current_start_x && b.w == run_width {
//                         b.h += 1;
//                         merged = true;
//                     }
//                 }
//             }

//             if !merged {
//                 let new_idx = boxes.len() as isize;
//                 boxes.push(PixelRect {
//                     x: current_start_x,
//                     y: y as u16,
//                     w: run_width,
//                     h: 1,
//                 });
//                 unsafe {
//                     *active_indices.get_unchecked_mut(start_x) = new_idx;
//                 }
//             }
//         }
//     }
//     boxes
// }

// fn detect_fps(path: &Path) -> Option<u16> {
//     let output = Command::new("ffprobe")
//         .args(&[
//             "-v",
//             "error",
//             "-select_streams",
//             "v:0",
//             "-show_entries",
//             "stream=r_frame_rate",
//             "-of",
//             "default=noprint_wrappers=1:nokey=1",
//         ])
//         .arg(path)
//         .output()
//         .ok()?;
//     let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
//     if out.contains('/') {
//         let parts: Vec<&str> = out.split('/').collect();
//         Some((parts[0].parse::<f64>().ok()? / parts[1].parse::<f64>().ok()?).round() as u16)
//     } else {
//         out.parse::<f64>().ok().map(|f| f.round() as u16)
//     }
// }

// fn get_frame_count(path: &Path) -> Option<u64> {
//     let output = Command::new("ffprobe")
//         .args(&[
//             "-v",
//             "error",
//             "-select_streams",
//             "v:0",
//             "-show_entries",
//             "stream=nb_frames",
//             "-of",
//             "default=noprint_wrappers=1:nokey=1",
//         ])
//         .arg(path)
//         .output()
//         .ok()?;
//     if let Ok(c) = String::from_utf8_lossy(&output.stdout)
//         .trim()
//         .parse::<u64>()
//     {
//         return Some(c);
//     }
//     let d = Command::new("ffprobe")
//         .args(&[
//             "-v",
//             "error",
//             "-show_entries",
//             "format=duration",
//             "-of",
//             "default=noprint_wrappers=1:nokey=1",
//         ])
//         .arg(path)
//         .output()
//         .ok()?;
//     let dur: f64 = String::from_utf8_lossy(&d.stdout).trim().parse().ok()?;
//     Some((dur * detect_fps(path)? as f64).round() as u64)
// }
