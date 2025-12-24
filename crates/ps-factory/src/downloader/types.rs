use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DownloadJob {
    pub url: String,
    pub project_name: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub use_gpu: bool,
}

#[derive(Debug, Clone)]
pub enum DownloadStatus {
    Starting,
    Downloading(f32),     // 0.0 - 1.0 (from yt-dlp)
    ProcessingVideo(f32), // 0.0 - 1.0 (from ffmpeg time / duration)
    ExtractingAudio(f32), // 0.0 - 1.0 (from ffmpeg time / duration)
    Finished(PathBuf),
    Error(String),
}

#[derive(Deserialize)]
pub struct FFProbeOutput {
    pub streams: Vec<FFProbeStream>,
    pub format: Option<FFProbeFormat>,
}

#[derive(Deserialize)]
pub struct FFProbeStream {
    pub codec_type: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub avg_frame_rate: String,
}

#[derive(Deserialize)]
pub struct FFProbeFormat {
    pub duration: String,
}

pub struct DownloadArgs {
    pub url: Option<String>,
    pub resolution: Option<String>,
    pub fps: Option<u32>,
    pub project_name: Option<String>,
}
