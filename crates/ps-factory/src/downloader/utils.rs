use super::types::FFProbeOutput;
use anyhow::{bail, Context, Result};
use dialoguer::{theme::ColorfulTheme, Select};
use std::path::Path;
use std::process::Command;

pub fn check_dependencies() -> Result<()> {
    let deps = ["yt-dlp", "ffmpeg", "ffprobe"];
    for dep in deps {
        which::which(dep).with_context(|| format!("Error: '{}' not found in PATH.", dep))?;
    }
    Ok(())
}

pub fn parse_ffmpeg_time(time_str: &str) -> Option<f64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: f64 = parts[0].parse().ok()?;
    let m: f64 = parts[1].parse().ok()?;
    let s: f64 = parts[2].parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

pub fn get_video_info(path: &Path) -> Result<(u32, u32, u32, f64)> {
    let output = Command::new("ffprobe")
        .args(&[
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_streams",
            "-show_format",
        ])
        .arg(path)
        .output()?;

    let parsed: FFProbeOutput =
        serde_json::from_slice(&output.stdout).context("Failed to parse ffprobe output")?;

    let duration: f64 = parsed
        .format
        .as_ref()
        .and_then(|f| f.duration.parse().ok())
        .unwrap_or(0.0);

    for stream in parsed.streams {
        if stream.codec_type == "video" {
            let w = stream.width.unwrap_or(0);
            let h = stream.height.unwrap_or(0);
            let fps = if stream.avg_frame_rate.contains('/') {
                let parts: Vec<&str> = stream.avg_frame_rate.split('/').collect();
                let num: f64 = parts[0].parse().unwrap_or(0.0);
                let den: f64 = parts[1].parse().unwrap_or(1.0);
                if den == 0.0 {
                    0
                } else {
                    (num / den).round() as u32
                }
            } else {
                stream.avg_frame_rate.parse::<f64>().unwrap_or(0.0).round() as u32
            };
            return Ok((w, h, fps, duration));
        }
    }
    bail!("No video stream found in downloaded file.")
}

pub fn resolve_resolution(arg: Option<String>) -> Result<(u32, u32)> {
    match arg.as_deref() {
        Some("720p") => Ok((1280, 720)),
        Some("1080p") => Ok((1920, 1080)),
        Some("1440p") => Ok((2560, 1440)),
        Some("2160p") => Ok((3840, 2160)),
        Some(other) => {
            println!(
                "Warning: Unknown resolution '{}', defaulting to 1080p",
                other
            );
            Ok((1920, 1080))
        }
        None => {
            let resolutions = vec!["720p", "1080p", "1440p", "2160p"];
            let idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select Target Resolution")
                .default(1)
                .items(&resolutions)
                .interact()?;
            match idx {
                0 => Ok((1280, 720)),
                1 => Ok((1920, 1080)),
                2 => Ok((2560, 1440)),
                3 => Ok((3840, 2160)),
                _ => Ok((1920, 1080)),
            }
        }
    }
}

pub fn resolve_fps(arg: Option<u32>) -> Result<u32> {
    match arg {
        Some(f) => Ok(f),
        None => {
            let options = vec!["30 FPS", "60 FPS", "120 FPS", "144 FPS"];
            let idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select FPS Strategy")
                .default(1)
                .items(&options)
                .interact()?;
            match idx {
                0 => Ok(30),
                1 => Ok(60),
                2 => Ok(120),
                3 => Ok(144),
                _ => Ok(30),
            }
        }
    }
}
