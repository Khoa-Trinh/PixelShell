use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DebugJob {
    pub file_path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum DebugStatus {
    Starting,
    Playing { frame: usize, rect_count: usize },
    Finished,
    Error(String),
}

pub struct DebugArgs {
    pub project_name: Option<String>,
    pub file_name: Option<String>,
}

// Struct matching the one in Builder/Runner
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
