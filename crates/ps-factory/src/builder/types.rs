use std::path::PathBuf;

/// The "Magic Footer" that the Runner looks for at the end of the file.
/// MUST match the struct definition in `ps-runner`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PayloadFooter {
    pub video_offset: u64,
    pub video_len: u64,
    pub audio_offset: u64,
    pub audio_len: u64,
    pub width: u16,
    pub height: u16,
    pub magic: [u8; 8],
}

#[derive(Debug, Clone)]
pub struct BuildTarget {
    pub project: String,
    pub resolution: String,
    pub width: u16,
    pub height: u16,
    pub bin_path: PathBuf,
    pub audio_path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum BuildStatus {
    Starting,
    Building(String),  // "Building my_project_1080p.exe..."
    Finished(PathBuf), // Returns path to the new .exe
    Error(String),
}

pub struct BuildArgs {
    pub project_name: Option<String>,
    pub resolutions: Option<String>,
    pub build_all: bool,
}
