use ps_core::PixelRect;
use std::cell::RefCell;
use std::path::PathBuf; // Ensure ps_core is in your dependencies

/// Defines a single conversion task (One video -> One .bin file)
#[derive(Debug, Clone)]
pub struct ConvertJob {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub fps: u16,
    pub use_gpu: bool,
}

/// Status updates sent from the Core Logic to the CLI or GUI
#[derive(Debug, Clone)]
pub enum ConverterStatus {
    Starting,
    Analyzing(String), // "Detecting FPS..."
    Processing {
        current_frame: u64,
        total_frames: u64,
        fps_speed: f64, // Processing speed
    },
    Finished,
    Error(String),
}

pub struct ConvertArgs {
    pub project_name: Option<String>,
    pub resolutions: Option<String>,
    pub use_gpu: bool,
}

// Internal structures for the pipeline
pub struct RawFrame {
    pub id: u64,
    pub data: Vec<u8>,
}

pub struct ProcessedFrame {
    pub id: u64,
    pub rects: Vec<PixelRect>,
    pub recycled_buffer: Vec<u8>,
}

// Thread-local scratch buffer for Snowplow algorithm
thread_local! {
    pub static SCRATCH_BUFFER: RefCell<Vec<isize>> = RefCell::new(Vec::new());
}
