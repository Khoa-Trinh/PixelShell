use ps_core::PixelRect;
use std::path::Path;
use std::process::Command;

// Unsafe Snowplow Algorithm (Kept exactly as optimized)
pub fn extract_rects_optimized(
    buffer: &[u8],
    width: u32,
    height: u32,
    threshold: u8,
    active_indices: &mut Vec<isize>,
) -> Vec<PixelRect> {
    let w = width as usize;
    let h = height as usize;
    active_indices.fill(-1);
    let mut boxes: Vec<PixelRect> = Vec::with_capacity(4000);

    for y in 0..h {
        let row_start = y * w;
        let row = unsafe { buffer.get_unchecked(row_start..row_start + w) };
        let mut x = 0;

        while x < w {
            let pixel = unsafe { *row.get_unchecked(x) };
            if pixel < threshold {
                unsafe {
                    let idx = active_indices.get_unchecked_mut(x);
                    if *idx != -1 {
                        *idx = -1;
                    }
                }
                x += 1;
                continue;
            }

            let start_x = x;
            while x < w {
                let p = unsafe { *row.get_unchecked(x) };
                if p < threshold {
                    break;
                }
                x += 1;
            }

            let run_width = (x - start_x) as u16;
            let current_start_x = start_x as u16;
            let active_idx = unsafe { *active_indices.get_unchecked(start_x) };
            let mut merged = false;

            if active_idx != -1 {
                let idx = active_idx as usize;
                if idx < boxes.len() {
                    let b = unsafe { boxes.get_unchecked_mut(idx) };
                    if b.y + b.h == (y as u16) && b.x == current_start_x && b.w == run_width {
                        b.h += 1;
                        merged = true;
                    }
                }
            }

            if !merged {
                let new_idx = boxes.len() as isize;
                boxes.push(PixelRect {
                    x: current_start_x,
                    y: y as u16,
                    w: run_width,
                    h: 1,
                });
                unsafe {
                    *active_indices.get_unchecked_mut(start_x) = new_idx;
                }
            }
        }
    }
    boxes
}

pub fn detect_fps(path: &Path) -> Option<u16> {
    let output = Command::new("ffprobe")
        .args(&[
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=r_frame_rate",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .ok()?;
    let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if out.contains('/') {
        let parts: Vec<&str> = out.split('/').collect();
        Some((parts[0].parse::<f64>().ok()? / parts[1].parse::<f64>().ok()?).round() as u16)
    } else {
        out.parse::<f64>().ok().map(|f| f.round() as u16)
    }
}

pub fn get_frame_count(path: &Path) -> Option<u64> {
    let output = Command::new("ffprobe")
        .args(&[
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=nb_frames",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .ok()?;
    if let Ok(c) = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
    {
        return Some(c);
    }
    let d = Command::new("ffprobe")
        .args(&[
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .ok()?;
    let dur: f64 = String::from_utf8_lossy(&d.stdout).trim().parse().ok()?;
    Some((dur * detect_fps(path)? as f64).round() as u64)
}
