use super::types::{DebugJob, DebugStatus, PayloadFooter};
use super::utils::draw_rect;
use anyhow::{bail, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use minifb::{Key, Window, WindowOptions};
use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    mem, thread,
    time::{Duration, Instant},
};

pub fn process_debug_session<F>(job: DebugJob, callback: F) -> Result<()>
where
    F: Fn(DebugStatus),
{
    callback(DebugStatus::Starting);

    let mut f = File::open(&job.file_path).context("Failed to open file")?;

    // DETECT MODE: .bin (Raw) vs .exe (Container)
    let file_size = f.metadata()?.len();
    let is_exe = job.file_path.extension().map_or(false, |e| e == "exe");

    let (mut reader, width, height, fps) = if is_exe {
        // --- EXE MODE: READ FOOTER ---
        if file_size < mem::size_of::<PayloadFooter>() as u64 {
            bail!("File too small to be a valid Pixel Shell EXE");
        }

        f.seek(SeekFrom::End(-(mem::size_of::<PayloadFooter>() as i64)))?;
        let mut footer_buf = [0u8; mem::size_of::<PayloadFooter>()];
        f.read_exact(&mut footer_buf)?;

        let footer: PayloadFooter = unsafe { std::mem::transmute(footer_buf) };

        if &footer.magic != b"PS_PATCH" {
            bail!("Invalid EXE: Magic signature 'PS_PATCH' not found.");
        }

        // Seek to Video Data
        f.seek(SeekFrom::Start(footer.video_offset))?;

        // Read FPS Header from the video blob
        let fps = f.read_u16::<LittleEndian>()?;

        (
            BufReader::new(f),
            footer.width as usize,
            footer.height as usize,
            fps,
        )
    } else {
        // --- BIN MODE: RAW READ ---
        let fps = f.read_u16::<LittleEndian>()?;
        (BufReader::new(f), 1920, 1080, fps) // Default res for raw bin if unknown
    };

    // SETUP WINDOW
    let mut window = Window::new(
        &format!("Debug View - {} FPS ({}x{})", fps, width, height),
        width,
        height,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )
    .context("Unable to create debug window")?;

    let mut buffer: Vec<u32> = vec![0; width * height];
    let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
    let mut frame_idx = 0;
    let mut rect_buf = [0u8; 8];

    // RENDER LOOP
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let start_time = Instant::now();
        buffer.fill(0xFF000000);
        let mut rect_count = 0;

        loop {
            if reader.read_exact(&mut rect_buf).is_err() {
                callback(DebugStatus::Finished);
                return Ok(());
            }

            let x = u16::from_le_bytes(rect_buf[0..2].try_into().unwrap());
            let y = u16::from_le_bytes(rect_buf[2..4].try_into().unwrap());
            let w = u16::from_le_bytes(rect_buf[4..6].try_into().unwrap());
            let h = u16::from_le_bytes(rect_buf[6..8].try_into().unwrap());

            if w == 0 && h == 0 {
                break;
            } // Frame End

            rect_count += 1;
            draw_rect(
                &mut buffer,
                width,
                height,
                x as usize,
                y as usize,
                w as usize,
                h as usize,
            );
        }

        if frame_idx % fps as usize == 0 {
            callback(DebugStatus::Playing {
                frame: frame_idx,
                rect_count,
            });
        }

        window.update_with_buffer(&buffer, width, height)?;
        frame_idx += 1;

        let elapsed = start_time.elapsed();
        if elapsed < frame_duration {
            thread::sleep(frame_duration - elapsed);
        }
    }

    callback(DebugStatus::Finished);
    Ok(())
}
