use super::types::{DownloadJob, DownloadStatus};
use super::utils::{get_video_info, parse_ffmpeg_time};
use anyhow::{bail, Context, Result};
use std::{
    env, fs,
    io::{BufRead, BufReader, Read},
    path::PathBuf,
    process::{Command, Stdio},
};

fn run_ffmpeg_progress<F, S>(
    mut cmd: Command,
    total_duration: f64,
    callback: &F,
    status_ctor: S,
) -> Result<()>
where
    F: Fn(DownloadStatus),
    S: Fn(f32) -> DownloadStatus,
{
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null()).stderr(Stdio::piped());

    let mut child = cmd.spawn().context("Failed to spawn ffmpeg")?;

    if let Some(stderr) = child.stderr.take() {
        let mut reader = BufReader::new(stderr);
        let mut buffer = Vec::new();
        let mut byte = [0u8; 1];

        // Custom loop to read until \r or \n
        while reader.read(&mut byte).unwrap_or(0) > 0 {
            let ch = byte[0] as char;

            if ch == '\r' || ch == '\n' {
                if !buffer.is_empty() {
                    let line = String::from_utf8_lossy(&buffer);

                    // Parse "time=00:00:00.00"
                    if let Some(idx) = line.find("time=") {
                        let remainder = &line[idx + 5..];
                        let time_str = remainder.split_whitespace().next().unwrap_or("");
                        if let Some(current_seconds) = parse_ffmpeg_time(time_str) {
                            if total_duration > 0.0 {
                                let pct = (current_seconds / total_duration) as f32;
                                callback(status_ctor(pct.clamp(0.0, 1.0)));
                            }
                        }
                    }
                    buffer.clear();
                }
            } else {
                buffer.push(byte[0]);
            }
        }
    }

    let status = child.wait()?;
    if !status.success() {
        bail!("FFmpeg command failed");
    }
    Ok(())
}

pub fn process_download<F>(job: DownloadJob, callback: F) -> Result<PathBuf>
where
    F: Fn(DownloadStatus),
{
    callback(DownloadStatus::Starting);

    // 1. Setup Paths
    let current_exe = env::current_exe().context("Failed to get exe path")?;
    let exe_dir = current_exe
        .parent()
        .context("Failed to get exe directory")?;
    let assets_root = exe_dir.join("assets");
    let output_dir = assets_root.join(&job.project_name);

    fs::create_dir_all(&output_dir)?;

    let temp_raw = output_dir.join("temp_raw.mp4");
    let final_video = output_dir.join(format!("{}.mkv", job.project_name));
    let final_audio = output_dir.join(format!("{}.ogg", job.project_name));

    // 2. Download (yt-dlp)
    callback(DownloadStatus::Downloading(0.0));

    let mut child = Command::new("yt-dlp")
        .args(&[
            "-f",
            "bestvideo+bestaudio/best",
            &job.url,
            "-o",
            temp_raw.to_str().unwrap(),
            "--merge-output-format",
            "mp4",
            "--no-playlist",
            "--newline", // Crucial for parsing
            "--progress",
            "--no-warnings",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to spawn yt-dlp")?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                // Parse: "[download]  45.0% of..."
                if line.starts_with("[download]") && line.contains('%') {
                    if let Some(start) = line.find(']') {
                        let remainder = &line[start + 1..];
                        if let Some(end) = remainder.find('%') {
                            let pct_str = remainder[..end].trim();
                            if let Ok(pct) = pct_str.parse::<f32>() {
                                callback(DownloadStatus::Downloading(pct / 100.0));
                            }
                        }
                    }
                }
            }
        }
    }

    let status = child.wait()?;
    if !status.success() {
        bail!("yt-dlp download failed.");
    }
    callback(DownloadStatus::Downloading(1.0));

    // 3. Inspect Source (Get Duration)
    let (cw, ch, cfps, duration) = get_video_info(&temp_raw)?;

    // 4. Build Filters
    let mut filters = Vec::new();
    if cw != job.width || ch != job.height {
        filters.push(format!(
            "scale={}:{}:force_original_aspect_ratio=increase:flags=lanczos",
            job.width, job.height
        ));
        filters.push(format!("crop={}:{}", job.width, job.height));
        filters.push("setsar=1".to_string());
    }
    if cfps != job.fps {
        filters.push(format!("fps={}", job.fps));
    }
    filters.push("format=gray".to_string());
    filters.push("gblur=sigma=1.5:steps=1".to_string());
    filters.push("eq=contrast=1000:saturation=0".to_string());

    let filter_str = filters.join(",");

    // 5. Process Video (Upscale/Filter)
    callback(DownloadStatus::ProcessingVideo(0.0));

    let mut ffmpeg_vid = Command::new("ffmpeg");
    ffmpeg_vid
        .arg("-i")
        .arg(&temp_raw)
        .arg("-vf")
        .arg(&filter_str)
        .args(&[
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-qp",
            "0",
            "-an",
            "-y",
        ])
        .arg(&final_video);

    run_ffmpeg_progress(
        ffmpeg_vid,
        duration,
        &callback,
        DownloadStatus::ProcessingVideo,
    )
    .context("Video processing failed")?;

    // Force 100% update
    callback(DownloadStatus::ProcessingVideo(1.0));

    // 6. Extract Audio
    callback(DownloadStatus::ExtractingAudio(0.0));

    let mut ffmpeg_audio = Command::new("ffmpeg");
    ffmpeg_audio
        .arg("-i")
        .arg(&temp_raw)
        .args(&["-vn", "-acodec", "libvorbis", "-q:a", "5", "-y"])
        .arg(&final_audio);

    run_ffmpeg_progress(
        ffmpeg_audio,
        duration,
        &callback,
        DownloadStatus::ExtractingAudio,
    )
    .context("Audio extraction failed")?;

    // Force 100% update
    callback(DownloadStatus::ExtractingAudio(1.0));

    // 7. Cleanup
    if temp_raw.exists() {
        let _ = fs::remove_file(temp_raw);
    }

    callback(DownloadStatus::Finished(output_dir.clone()));
    Ok(output_dir)
}
