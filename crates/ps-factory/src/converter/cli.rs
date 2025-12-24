use super::core::process_conversion;
use super::types::*;
use super::utils::detect_fps;
use anyhow::{bail, Context, Result};
use dialoguer::{theme::ColorfulTheme, MultiSelect, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::{env, fs};

pub fn run_cli(args: ConvertArgs) -> Result<()> {
    // 1. Setup Environment
    let current_exe = env::current_exe().context("Failed to get exe path")?;
    let exe_dir = current_exe
        .parent()
        .context("Failed to get exe directory")?;
    let assets_root = exe_dir.join("assets");

    // 2. Select Project
    let project_name = match args.project_name {
        Some(name) => name,
        None => {
            if !assets_root.exists() {
                bail!("'assets' folder missing.");
            }
            let entries = fs::read_dir(&assets_root)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect::<Vec<_>>();
            if entries.is_empty() {
                bail!("No projects found.");
            }

            let idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select Project")
                .items(&entries)
                .default(0)
                .interact()?;
            entries[idx].clone()
        }
    };

    // 3. Select Resolutions
    let resolution_names: Vec<String> = match args.resolutions {
        Some(s) => s.split(',').map(|x| x.trim().to_string()).collect(),
        None => {
            let options = vec!["720p", "1080p", "1440p", "2160p"];
            let defaults = vec![true, true, false, false];
            let selections = MultiSelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Select Output Resolutions")
                .items(&options)
                .defaults(&defaults)
                .interact()?;
            if selections.is_empty() {
                bail!("Select at least one resolution.");
            }
            selections.iter().map(|&i| options[i].to_string()).collect()
        }
    };

    // 4. Locate Source Video
    let project_dir = assets_root.join(&project_name);
    let vid_path = ["mkv", "mp4", "avi", "mov", "webm"]
        .iter()
        .map(|ext| project_dir.join(format!("{}.{}", project_name, ext)))
        .find(|p| p.exists())
        .context("No video found in project folder")?;

    let fps = detect_fps(&vid_path).unwrap_or(30);
    println!("Detected: {} FPS", fps);

    // 5. Process Loop
    for res_name in resolution_names {
        let width = match res_name.as_str() {
            "720p" => 1280,
            "1080p" => 1920,
            "1440p" => 2560,
            "2160p" => 3840,
            _ => continue,
        };
        let height = width * 9 / 16;
        let out_path = project_dir.join(format!("{}_{}.bin", project_name, res_name));

        println!("\n--- Processing {} ({}x{}) ---", res_name, width, height);

        let job = ConvertJob {
            input_path: vid_path.clone(),
            output_path: out_path,
            width,
            height,
            fps,
            use_gpu: args.use_gpu,
        };

        // SETUP CLI PROGRESS BAR
        let pb = ProgressBar::new(100);
        pb.set_style(ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({per_sec}) {eta} {msg}"
        ).unwrap().progress_chars("#>-"));

        // RUN LOGIC with CLI Callback
        let pb_clone = pb.clone();
        process_conversion(job, move |status| match status {
            ConverterStatus::Starting => pb_clone.set_message("Starting..."),
            ConverterStatus::Analyzing(_) => pb_clone.set_message("Analyzing..."),
            ConverterStatus::Processing {
                current_frame,
                total_frames,
                ..
            } => {
                pb_clone.set_length(total_frames);
                pb_clone.set_position(current_frame);
            }
            ConverterStatus::Finished => pb_clone.finish_with_message("Done!"),
            ConverterStatus::Error(e) => pb_clone.abandon_with_message(format!("Error: {}", e)),
        })?;
    }

    println!("\n--- ALL TASKS COMPLETE ---");
    Ok(())
}
