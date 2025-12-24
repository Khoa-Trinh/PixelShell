// use anyhow::{bail, Context, Result};
// use dialoguer::{theme::ColorfulTheme, Input, Select};
// use serde::Deserialize;
// use std::{
//     cell::RefCell,
//     env, fs,
//     io::{BufRead, BufReader, Read},
//     path::{Path, PathBuf},
//     process::{Command, Stdio},
//     sync::mpsc::Sender,
//     thread,
// };

// // ==================================================================================
// // 1. DATA STRUCTURES (Unchanged)
// // ==================================================================================

// #[derive(Debug, Clone)]
// pub struct DownloadJob {
//     pub url: String,
//     pub project_name: String,
//     pub width: u32,
//     pub height: u32,
//     pub fps: u32,
//     pub use_gpu: bool,
// }

// #[derive(Debug, Clone)]
// pub enum DownloadStatus {
//     Starting,
//     Downloading(f32),     // 0.0 - 1.0
//     ProcessingVideo(f32), // 0.0 - 1.0
//     ExtractingAudio(f32), // 0.0 - 1.0
//     Finished(PathBuf),
//     Error(String),
// }

// #[derive(Deserialize)]
// struct FFProbeOutput {
//     streams: Vec<FFProbeStream>,
//     format: Option<FFProbeFormat>,
// }

// #[derive(Deserialize)]
// struct FFProbeStream {
//     codec_type: String,
//     width: Option<u32>,
//     height: Option<u32>,
//     avg_frame_rate: String,
// }

// #[derive(Deserialize)]
// struct FFProbeFormat {
//     duration: String,
// }

// pub struct DownloadArgs {
//     pub url: Option<String>,
//     pub resolution: Option<String>,
//     pub fps: Option<u32>,
//     pub project_name: Option<String>,
// }

// // ==================================================================================
// // 2. CORE LOGIC (Unchanged)
// // ==================================================================================

// pub fn check_dependencies() -> Result<()> {
//     let deps = ["yt-dlp", "ffmpeg", "ffprobe"];
//     for dep in deps {
//         which::which(dep).with_context(|| format!("Error: '{}' not found in PATH.", dep))?;
//     }
//     Ok(())
// }

// fn parse_ffmpeg_time(time_str: &str) -> Option<f64> {
//     let parts: Vec<&str> = time_str.split(':').collect();
//     if parts.len() != 3 {
//         return None;
//     }
//     let h: f64 = parts[0].parse().ok()?;
//     let m: f64 = parts[1].parse().ok()?;
//     let s: f64 = parts[2].parse().ok()?;
//     Some(h * 3600.0 + m * 60.0 + s)
// }

// fn run_ffmpeg_progress<F, S>(
//     mut cmd: Command,
//     total_duration: f64,
//     callback: &F,
//     status_ctor: S,
// ) -> Result<()>
// where
//     F: Fn(DownloadStatus),
//     S: Fn(f32) -> DownloadStatus,
// {
//     cmd.stdin(Stdio::null());
//     cmd.stdout(Stdio::null()).stderr(Stdio::piped());

//     let mut child = cmd.spawn().context("Failed to spawn ffmpeg")?;

//     if let Some(stderr) = child.stderr.take() {
//         let mut reader = BufReader::new(stderr);
//         let mut buffer = Vec::new();
//         let mut byte = [0u8; 1];

//         while reader.read(&mut byte).unwrap_or(0) > 0 {
//             let ch = byte[0] as char;
//             if ch == '\r' || ch == '\n' {
//                 if !buffer.is_empty() {
//                     let line = String::from_utf8_lossy(&buffer);
//                     if let Some(idx) = line.find("time=") {
//                         let remainder = &line[idx + 5..];
//                         let time_str = remainder.split_whitespace().next().unwrap_or("");
//                         if let Some(current_seconds) = parse_ffmpeg_time(time_str) {
//                             if total_duration > 0.0 {
//                                 let pct = (current_seconds / total_duration) as f32;
//                                 callback(status_ctor(pct.clamp(0.0, 1.0)));
//                             }
//                         }
//                     }
//                     buffer.clear();
//                 }
//             } else {
//                 buffer.push(byte[0]);
//             }
//         }
//     }

//     let status = child.wait()?;
//     if !status.success() {
//         bail!("FFmpeg command failed");
//     }
//     Ok(())
// }

// pub fn process_download<F>(job: DownloadJob, callback: F) -> Result<PathBuf>
// where
//     F: Fn(DownloadStatus),
// {
//     callback(DownloadStatus::Starting);

//     // 1. Setup Paths
//     let current_exe = env::current_exe()?;
//     let exe_dir = current_exe.parent().context("Failed to get exe dir")?;
//     let assets_root = exe_dir.join("assets");
//     let output_dir = assets_root.join(&job.project_name);
//     fs::create_dir_all(&output_dir)?;

//     let temp_raw = output_dir.join("temp_raw.mp4");
//     let final_video = output_dir.join(format!("{}.mkv", job.project_name));
//     let final_audio = output_dir.join(format!("{}.ogg", job.project_name));

//     // 2. Download (yt-dlp)
//     callback(DownloadStatus::Downloading(0.0));

//     let mut child = Command::new("yt-dlp")
//         .args(&[
//             "-f",
//             "bestvideo+bestaudio/best",
//             &job.url,
//             "-o",
//             temp_raw.to_str().unwrap(),
//             "--merge-output-format",
//             "mp4",
//             "--no-playlist",
//             "--newline",
//             "--progress",
//             "--no-warnings",
//         ])
//         .stdout(Stdio::piped())
//         .stderr(Stdio::null())
//         .spawn()?;

//     if let Some(stdout) = child.stdout.take() {
//         let reader = BufReader::new(stdout);
//         for line in reader.lines() {
//             if let Ok(line) = line {
//                 if line.starts_with("[download]") && line.contains('%') {
//                     if let Some(start) = line.find(']') {
//                         let remainder = &line[start + 1..];
//                         if let Some(end) = remainder.find('%') {
//                             let pct_str = remainder[..end].trim();
//                             if let Ok(pct) = pct_str.parse::<f32>() {
//                                 callback(DownloadStatus::Downloading(pct / 100.0));
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//     }

//     if !child.wait()?.success() {
//         bail!("yt-dlp download failed.");
//     }
//     // We send 1.0 here, but we rely on the NEXT stage starting to print the newline
//     callback(DownloadStatus::Downloading(1.0));

//     // 3. Inspect Source
//     let (cw, ch, cfps, duration) = get_video_info(&temp_raw)?;

//     // 4. Build Filters
//     let mut filters = Vec::new();
//     if cw != job.width || ch != job.height {
//         filters.push(format!(
//             "scale={}:{}:force_original_aspect_ratio=increase:flags=lanczos",
//             job.width, job.height
//         ));
//         filters.push(format!("crop={}:{}", job.width, job.height));
//         filters.push("setsar=1".to_string());
//     }
//     if cfps != job.fps {
//         filters.push(format!("fps={}", job.fps));
//     }
//     filters.push("format=gray,gblur=sigma=1.5:steps=1,eq=contrast=1000:saturation=0".to_string());
//     let filter_str = filters.join(",");

//     // 5. Process Video
//     callback(DownloadStatus::ProcessingVideo(0.0));
//     let mut ffmpeg_vid = Command::new("ffmpeg");
//     ffmpeg_vid
//         .arg("-i")
//         .arg(&temp_raw)
//         .arg("-vf")
//         .arg(&filter_str)
//         .args(&[
//             "-c:v",
//             "libx264",
//             "-preset",
//             "ultrafast",
//             "-qp",
//             "0",
//             "-an",
//             "-y",
//         ])
//         .arg(&final_video);

//     run_ffmpeg_progress(
//         ffmpeg_vid,
//         duration,
//         &callback,
//         DownloadStatus::ProcessingVideo,
//     )?;
//     callback(DownloadStatus::ProcessingVideo(1.0));

//     // 6. Extract Audio
//     callback(DownloadStatus::ExtractingAudio(0.0));
//     let mut ffmpeg_audio = Command::new("ffmpeg");
//     ffmpeg_audio
//         .arg("-i")
//         .arg(&temp_raw)
//         .args(&["-vn", "-acodec", "libvorbis", "-q:a", "5", "-y"])
//         .arg(&final_audio);

//     run_ffmpeg_progress(
//         ffmpeg_audio,
//         duration,
//         &callback,
//         DownloadStatus::ExtractingAudio,
//     )?;
//     callback(DownloadStatus::ExtractingAudio(1.0));

//     // 7. Cleanup
//     if temp_raw.exists() {
//         let _ = fs::remove_file(temp_raw);
//     }

//     callback(DownloadStatus::Finished(output_dir.clone()));
//     Ok(output_dir)
// }

// // ==================================================================================
// // 3. CLI HANDLER (Fixed Formatting)
// // ==================================================================================

// pub fn run_cli(args: DownloadArgs) -> Result<()> {
//     check_dependencies()?;

//     let url = match args.url {
//         Some(u) => u,
//         None => Input::with_theme(&ColorfulTheme::default())
//             .with_prompt("Enter YouTube URL")
//             .interact_text()?,
//     };
//     let (width, height) = resolve_resolution(args.resolution)?;
//     let fps = resolve_fps(args.fps)?;
//     let project_name = match args.project_name {
//         Some(name) => name,
//         None => Input::with_theme(&ColorfulTheme::default())
//             .with_prompt("Enter Project Name")
//             .default("my_project".into())
//             .interact_text()?,
//     };

//     let job = DownloadJob {
//         url,
//         project_name,
//         width,
//         height,
//         fps,
//         use_gpu: false,
//     };

//     println!(
//         "\n🚀 Starting Job: {} [{}x{} @ {}fps]",
//         job.project_name, width, height, fps
//     );

//     // STATE TRACKER: 0=Start, 1=DL, 2=Proc, 3=Audio
//     // This allows us to print a newline ONLY when the stage changes.
//     let current_stage = RefCell::new(0);

//     process_download(job, |status| {
//         let mut stage = current_stage.borrow_mut();

//         match status {
//             DownloadStatus::Starting => {
//                 println!("   [1/4] Initializing...");
//                 *stage = 1;
//             }
//             DownloadStatus::Downloading(pct) => {
//                 use std::io::Write;
//                 print!("\r   [2/4] Downloading... {:.0}%      ", pct * 100.0);
//                 std::io::stdout().flush().ok();
//                 // Never print newline here.
//             }
//             DownloadStatus::ProcessingVideo(pct) => {
//                 // If this is the FIRST update for processing, close the previous line
//                 if *stage == 1 {
//                     println!(); // Finish the "Downloading... 100%" line
//                     *stage = 2;
//                 }

//                 use std::io::Write;
//                 print!("\r   [3/4] Processing... {:.0}%       ", pct * 100.0);
//                 std::io::stdout().flush().ok();
//             }
//             DownloadStatus::ExtractingAudio(pct) => {
//                 if *stage == 2 {
//                     println!(); // Finish the "Processing... 100%" line
//                     *stage = 3;
//                 }

//                 use std::io::Write;
//                 print!("\r   [4/4] Audio... {:.0}%            ", pct * 100.0);
//                 std::io::stdout().flush().ok();
//             }
//             DownloadStatus::Finished(path) => {
//                 if *stage == 3 {
//                     println!();
//                 } // Finish the Audio line
//                 println!("✅ Success! Project saved at: {:?}", path);
//             }
//             DownloadStatus::Error(e) => eprintln!("\n❌ Error: {}", e),
//         }
//     })?;

//     Ok(())
// }

// // ==================================================================================
// // 4. ASYNC & HELPERS (Unchanged)
// // ==================================================================================

// pub fn run_async(job: DownloadJob, sender: Sender<DownloadStatus>) -> Result<()> {
//     check_dependencies()?;
//     thread::spawn(move || {
//         let result = process_download(job, |status| {
//             let _ = sender.send(status);
//         });
//         if let Err(e) = result {
//             let _ = sender.send(DownloadStatus::Error(e.to_string()));
//         }
//     });
//     Ok(())
// }

// fn resolve_resolution(arg: Option<String>) -> Result<(u32, u32)> {
//     match arg.as_deref() {
//         Some("720p") => Ok((1280, 720)),
//         Some("1080p") => Ok((1920, 1080)),
//         Some("1440p") => Ok((2560, 1440)),
//         Some("2160p") => Ok((3840, 2160)),
//         Some(_) => Ok((1920, 1080)),
//         None => {
//             let idx = Select::with_theme(&ColorfulTheme::default())
//                 .with_prompt("Select Target Resolution")
//                 .items(&vec!["720p", "1080p", "1440p", "2160p"])
//                 .default(1)
//                 .interact()?;
//             match idx {
//                 0 => Ok((1280, 720)),
//                 1 => Ok((1920, 1080)),
//                 2 => Ok((2560, 1440)),
//                 3 => Ok((3840, 2160)),
//                 _ => Ok((1920, 1080)),
//             }
//         }
//     }
// }
// fn resolve_fps(arg: Option<u32>) -> Result<u32> {
//     match arg {
//         Some(f) => Ok(f),
//         None => {
//             let idx = Select::with_theme(&ColorfulTheme::default())
//                 .with_prompt("Select FPS Strategy")
//                 .items(&vec!["30 FPS", "60 FPS"])
//                 .default(1)
//                 .interact()?;
//             match idx {
//                 0 => Ok(30),
//                 1 => Ok(60),
//                 _ => Ok(30),
//             }
//         }
//     }
// }
// fn get_video_info(path: &Path) -> Result<(u32, u32, u32, f64)> {
//     let output = Command::new("ffprobe")
//         .args(&[
//             "-v",
//             "quiet",
//             "-print_format",
//             "json",
//             "-show_streams",
//             "-show_format",
//         ])
//         .arg(path)
//         .output()?;
//     let parsed: FFProbeOutput = serde_json::from_slice(&output.stdout)?;
//     let duration = parsed
//         .format
//         .as_ref()
//         .and_then(|f| f.duration.parse().ok())
//         .unwrap_or(0.0);
//     for stream in parsed.streams {
//         if stream.codec_type == "video" {
//             let fps = stream
//                 .avg_frame_rate
//                 .split('/')
//                 .map(|s| s.parse::<f64>().unwrap_or(0.0))
//                 .collect::<Vec<_>>();
//             let f = if fps.len() == 2 && fps[1] != 0.0 {
//                 (fps[0] / fps[1]).round() as u32
//             } else {
//                 30
//             };
//             return Ok((
//                 stream.width.unwrap_or(0),
//                 stream.height.unwrap_or(0),
//                 f,
//                 duration,
//             ));
//         }
//     }
//     bail!("No video")
// }
