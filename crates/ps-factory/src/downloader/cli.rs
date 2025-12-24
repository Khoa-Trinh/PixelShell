use super::core::process_download;
use super::types::{DownloadArgs, DownloadJob, DownloadStatus};
use super::utils::{check_dependencies, resolve_fps, resolve_resolution};
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input};
use indicatif::{ProgressBar, ProgressStyle};
use std::cell::RefCell;

pub fn run_cli(args: DownloadArgs) -> Result<()> {
    check_dependencies()?;

    let url: String = match args.url {
        Some(u) => u,
        None => Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter YouTube URL")
            .interact_text()?,
    };

    let (width, height) = resolve_resolution(args.resolution)?;
    let fps = resolve_fps(args.fps)?;

    let project_name: String = match args.project_name {
        Some(name) => name,
        None => Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter Project Name")
            .default("my_project".into())
            .interact_text()?,
    };

    let job = DownloadJob {
        url,
        project_name,
        width,
        height,
        fps,
        use_gpu: false,
    };

    println!(
        "\n🚀 Starting Job: {} [{}x{} @ {}fps]",
        job.project_name, width, height, fps
    );

    // SETUP CLI PROGRESS BAR
    // We use a length of 100 since our status updates are percentages (0.0 - 1.0)
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>3}% ({per_sec}) {msg}",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    // Clone for the closure
    let pb_clone = pb.clone();

    // We use a helper to track stages so we can persist messages when a stage finishes
    let current_stage = RefCell::new("init");

    process_download(job, move |status| {
        let mut last_stage = current_stage.borrow_mut();

        match status {
            DownloadStatus::Starting => {
                pb_clone.set_message("Initializing...");
            }
            DownloadStatus::Downloading(pct) => {
                if *last_stage != "download" {
                    *last_stage = "download";
                    pb_clone.reset(); // Reset stats for the new stage
                }
                pb_clone.set_message("Downloading Source...");
                pb_clone.set_position((pct * 100.0) as u64);
            }
            DownloadStatus::ProcessingVideo(pct) => {
                if *last_stage != "process" {
                    // When switching to processing, log that download is done
                    if *last_stage == "download" {
                        pb_clone.println("✅ Download Complete");
                    }
                    *last_stage = "process";
                    pb_clone.reset();
                }
                pb_clone.set_message("Processing Video...");
                pb_clone.set_position((pct * 100.0) as u64);
            }
            DownloadStatus::ExtractingAudio(pct) => {
                if *last_stage != "audio" {
                    if *last_stage == "process" {
                        pb_clone.println("✅ Video Processing Complete");
                    }
                    *last_stage = "audio";
                    pb_clone.reset();
                }
                pb_clone.set_message("Extracting Audio...");
                pb_clone.set_position((pct * 100.0) as u64);
            }
            DownloadStatus::Finished(path) => {
                pb_clone.finish_with_message("All tasks finished.");
                println!("✅ Success! Project saved at: {:?}", path);
            }
            DownloadStatus::Error(e) => {
                pb_clone.abandon_with_message(format!("Error: {}", e));
            }
        }
    })?;

    Ok(())
}
