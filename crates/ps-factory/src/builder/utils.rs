use super::types::BuildTarget;
use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

// NEW: Public helper to detect resolution from a filename
pub fn detect_resolution(filename: &str) -> (String, u16, u16) {
    let resolutions = vec![
        ("720p", 1280, 720),
        ("1080p", 1920, 1080),
        ("1440p", 2560, 1440),
        ("2160p", 3840, 2160),
    ];

    for (res_name, w, h) in resolutions {
        if filename.contains(res_name) {
            return (res_name.to_string(), w, h);
        }
    }

    // Default fallback
    ("1080p".to_string(), 1920, 1080)
}

// NEW: Public helper to find audio file
pub fn detect_audio_path(bin_path: &Path) -> PathBuf {
    let filename = bin_path.file_stem().unwrap().to_string_lossy();
    let parent = bin_path.parent().unwrap();

    // 1. Try exact match (project.ogg)
    let exact = parent.join(format!("{}.ogg", filename));
    if exact.exists() {
        return exact;
    }

    // 2. Try stripping resolution tags (project_1080p.bin -> project.ogg)
    let base_name = filename
        .replace("_720p", "")
        .replace("_1080p", "")
        .replace("_1440p", "")
        .replace("_2160p", "");

    let stripped = parent.join(format!("{}.ogg", base_name));
    if stripped.exists() {
        return stripped;
    }

    // Return empty if not found (builder will error later, or handle gracefully)
    PathBuf::new()
}

pub fn get_available_builds(assets_dir: &Path) -> Result<Vec<BuildTarget>> {
    if !assets_dir.exists() {
        return Ok(vec![]);
    }
    let mut targets = Vec::new();

    // We still iterate resolutions here to scan for SPECIFIC files
    // distinct from the detection logic above
    let resolutions = vec![
        ("720p", 1280, 720),
        ("1080p", 1920, 1080),
        ("1440p", 2560, 1440),
        ("2160p", 3840, 2160),
    ];

    for entry in fs::read_dir(assets_dir)? {
        let entry = entry?;
        if entry.path().is_dir() {
            let project_name = entry.file_name().to_string_lossy().to_string();
            let project_dir = entry.path();
            let audio_path = project_dir.join(format!("{}.ogg", project_name));

            if !audio_path.exists() {
                continue;
            }

            for (res_name, w, h) in &resolutions {
                let bin_name = format!("{}_{}.bin", project_name, res_name);
                let bin_path = project_dir.join(&bin_name);

                if bin_path.exists() {
                    targets.push(BuildTarget {
                        project: project_name.clone(),
                        resolution: res_name.to_string(),
                        width: *w,
                        height: *h,
                        bin_path: fs::canonicalize(&bin_path)?,
                        audio_path: fs::canonicalize(&audio_path)?,
                    });
                }
            }
        }
    }
    Ok(targets)
}
