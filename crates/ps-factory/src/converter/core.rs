use super::types::*;
use super::utils::{extract_rects_optimized, get_frame_count};
use anyhow::{Context, Result};
use crossbeam_channel::{bounded, Receiver, Sender};
use ps_core::PixelRect;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Read, Write},
    process::{Command, Stdio},
    thread,
    time::Instant,
};

/// The high-performance engine.
/// Accepts a generic `callback` to report progress.
pub fn process_conversion<F>(job: ConvertJob, callback: F) -> Result<()>
where
    F: Fn(ConverterStatus) + Send + Clone + 'static,
{
    callback(ConverterStatus::Starting);

    // 1. Analyze Video
    callback(ConverterStatus::Analyzing("Detecting Metadata...".into()));
    let total_frames = get_frame_count(&job.input_path).unwrap_or(0);

    // 2. FFmpeg Setup
    let filters = format!(
        "scale={w}:{h},format=gray,gblur=sigma=1.0:steps=1,eq=contrast=1000:saturation=0",
        w = job.width,
        h = job.height
    );

    let mut cmd = Command::new("ffmpeg");
    if job.use_gpu {
        cmd.arg("-hwaccel").arg("cuda");
    }

    cmd.arg("-i")
        .arg(&job.input_path)
        .arg("-vf")
        .arg(filters)
        .arg("-f")
        .arg("rawvideo")
        .arg("-pix_fmt")
        .arg("gray")
        .arg("-");

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to spawn ffmpeg")?;

    let mut stdout = child.stdout.take().context("Failed to open stdout")?;

    // 3. Channel Setup
    let queue_size = 64;
    let (tx_raw, rx_raw): (Sender<RawFrame>, Receiver<RawFrame>) = bounded(queue_size);
    let (tx_processed, rx_processed): (Sender<ProcessedFrame>, Receiver<ProcessedFrame>) =
        bounded(queue_size);
    let (tx_recycle, rx_recycle): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = bounded(queue_size);

    let frame_size = (job.width * job.height) as usize;

    // Pre-fill Recycle Bin
    for _ in 0..queue_size {
        let _ = tx_recycle.send(vec![0u8; frame_size]);
    }

    // 4. Writer Thread (Handles Disk I/O + Reporting)
    let output_path = job.output_path.clone();
    let cb_writer = callback.clone();
    let fps = job.fps;

    let write_handle = thread::spawn(move || -> Result<()> {
        let mut file_out = BufWriter::with_capacity(4 * 1024 * 1024, File::create(output_path)?);
        file_out.write_all(&fps.to_le_bytes())?;

        let mut next_needed_id = 0;
        let mut reorder_buffer: HashMap<u64, Vec<PixelRect>> = HashMap::new();

        let start_time = Instant::now();
        let mut last_report = Instant::now();

        for frame in rx_processed {
            reorder_buffer.insert(frame.id, frame.rects);
            let _ = tx_recycle.send(frame.recycled_buffer); // Return buffer immediately

            while let Some(rects) = reorder_buffer.remove(&next_needed_id) {
                // Bulk write rects (unsafe cast is virtually free)
                let rect_bytes = unsafe {
                    std::slice::from_raw_parts(
                        rects.as_ptr() as *const u8,
                        rects.len() * std::mem::size_of::<PixelRect>(),
                    )
                };
                file_out.write_all(rect_bytes)?;

                let eos = PixelRect::EOS_MARKER;
                let eos_bytes = unsafe {
                    std::slice::from_raw_parts(
                        &eos as *const PixelRect as *const u8,
                        std::mem::size_of::<PixelRect>(),
                    )
                };
                file_out.write_all(eos_bytes)?;

                next_needed_id += 1;

                // Report Progress (throttled to ~10 times/sec to save CPU)
                if next_needed_id % 30 == 0 || last_report.elapsed().as_millis() > 100 {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 {
                        next_needed_id as f64 / elapsed
                    } else {
                        0.0
                    };

                    cb_writer(ConverterStatus::Processing {
                        current_frame: next_needed_id,
                        total_frames,
                        fps_speed: speed,
                    });
                    last_report = Instant::now();
                }
            }
        }
        cb_writer(ConverterStatus::Finished);
        Ok(())
    });

    // 5. Reader Thread
    thread::spawn(move || {
        let mut frame_id = 0;
        loop {
            let mut buffer = rx_recycle.recv().unwrap_or_else(|_| vec![0u8; frame_size]);
            if stdout.read_exact(&mut buffer).is_err() {
                break;
            } // EOF
            if tx_raw
                .send(RawFrame {
                    id: frame_id,
                    data: buffer,
                })
                .is_err()
            {
                break;
            }
            frame_id += 1;
        }
    });

    // 6. Parallel Compute (Main Thread Logic)
    let width = job.width;
    let height = job.height;

    rx_raw.into_iter().par_bridge().for_each(|raw| {
        SCRATCH_BUFFER.with(|cell| {
            let mut indices = cell.borrow_mut();
            if indices.len() != width as usize {
                *indices = vec![-1isize; width as usize];
            }

            let rects = extract_rects_optimized(&raw.data, width, height, 127, &mut indices);

            let _ = tx_processed.send(ProcessedFrame {
                id: raw.id,
                rects,
                recycled_buffer: raw.data,
            });
        });
    });

    drop(tx_processed);
    write_handle.join().expect("Writer panic")?;
    Ok(())
}
