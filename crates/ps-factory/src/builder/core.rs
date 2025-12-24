use super::types::{BuildTarget, PayloadFooter};
use anyhow::{bail, Context, Result};
use std::{
    fs::{self, File},
    io::Write,
    mem,
    path::{Path, PathBuf},
    slice,
}; // Import from types module

/// Reads the template, appends data, and writes the final standalone EXE.
pub fn build_single_target(
    target: &BuildTarget,
    template_path: &Path,
    output_dir: &Path,
) -> Result<PathBuf> {
    // 1. Validation
    if !template_path.exists() {
        bail!("Template not found at {:?}", template_path);
    }

    let exe_name = format!("{}_{}.exe", target.project, target.resolution);
    let output_path = output_dir.join(&exe_name);

    // 2. Load Data
    let template_bytes = fs::read(template_path).context("Failed to read template exe")?;
    let video_data = fs::read(&target.bin_path).context("Failed to read video bin")?;
    let audio_data = fs::read(&target.audio_path).context("Failed to read audio ogg")?;

    // 3. Calculate Offsets
    let template_len = template_bytes.len() as u64;
    let video_len = video_data.len() as u64;
    let audio_len = audio_data.len() as u64;

    let video_offset = template_len;
    let audio_offset = template_len + video_len;

    let footer = PayloadFooter {
        video_offset,
        video_len,
        audio_offset,
        audio_len,
        width: target.width,
        height: target.height,
        magic: *b"PS_PATCH",
    };

    // 4. Write Output File
    let mut file = File::create(&output_path).context("Failed to create output file")?;

    file.write_all(&template_bytes)?;
    file.write_all(&video_data)?;
    file.write_all(&audio_data)?;

    let footer_bytes = unsafe {
        slice::from_raw_parts(
            &footer as *const PayloadFooter as *const u8,
            mem::size_of::<PayloadFooter>(),
        )
    };
    file.write_all(footer_bytes)?;

    Ok(output_path)
}
