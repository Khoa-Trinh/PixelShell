use ps_factory::builder;
use std::path::PathBuf;

pub fn extract_percentage(msg: &str) -> Option<f32> {
    if let Some(start) = msg.rfind(' ') {
        let potential_num = &msg[start + 1..].trim_end_matches('%');
        return potential_num.parse::<f32>().ok();
    }
    None
}

pub fn scan_build_targets() -> Vec<builder::BuildTarget> {
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let assets_dir = exe_dir.join("assets");
            if let Ok(targets) = builder::get_available_builds(&assets_dir) {
                return targets;
            }
        }
    }
    vec![]
}

pub fn scan_video_files() -> Vec<PathBuf> {
    scan_files_with_ext(&["mp4", "mkv", "avi", "mov"], "assets")
}

pub fn scan_bin_files() -> Vec<PathBuf> {
    scan_files_with_ext(&["bin"], "assets")
}

pub fn scan_dist_files() -> Vec<PathBuf> {
    scan_files_with_ext(&["exe"], "dist")
}

fn scan_files_with_ext(exts: &[&str], folder: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let target_dir = exe_dir.join(folder);
            if target_dir.exists() {
                for entry in walkdir::WalkDir::new(&target_dir)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let path = entry.path();
                    if let Some(extension) = path.extension() {
                        let ext_str = extension.to_string_lossy().to_lowercase();
                        if exts.contains(&ext_str.as_str()) {
                            files.push(path.to_path_buf());
                        }
                    }
                }
            }
        }
    }
    files
}
