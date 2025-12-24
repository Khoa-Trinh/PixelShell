use super::{
    core::build_single_target,
    types::{BuildArgs, BuildTarget},
    utils::get_available_builds,
};
use anyhow::{bail, Context, Result};
use dialoguer::{theme::ColorfulTheme, MultiSelect};
use indicatif::{ProgressBar, ProgressStyle};
use std::{env, fs};

pub fn run_cli(args: BuildArgs) -> Result<()> {
    // 1. Setup Paths
    let current_exe = env::current_exe().context("Failed to get exe path")?;
    let exe_dir = current_exe
        .parent()
        .context("Failed to get exe directory")?;
    let assets_dir = exe_dir.join("assets");
    let dist_dir = exe_dir.join("dist");
    let template_path = exe_dir.join("ps-runner.exe");

    if !template_path.exists() {
        bail!("❌ Missing Template!\nCould not find 'ps-runner.exe' at:\n{:?}\n\nPlease build ps-runner first.", template_path);
    }

    // 2. Discover Targets
    let all_targets = get_available_builds(&assets_dir)?;
    if all_targets.is_empty() {
        bail!("No assets found. Run 'convert' first.");
    }

    // 3. Filter / Select Targets
    let selected_targets: Vec<BuildTarget> =
        if args.build_all || args.project_name.is_some() || args.resolutions.is_some() {
            // Flag Mode
            println!("Filtering targets based on flags...");
            let req_res: Option<Vec<String>> = args
                .resolutions
                .as_ref()
                .map(|s| s.split(',').map(|r| r.trim().to_string()).collect());

            all_targets
                .into_iter()
                .filter(|t| {
                    if let Some(p) = &args.project_name {
                        if &t.project != p {
                            return false;
                        }
                    }
                    if let Some(r_list) = &req_res {
                        if !r_list.contains(&t.resolution) {
                            return false;
                        }
                    }
                    true
                })
                .collect()
        } else {
            // Interactive Mode
            let options: Vec<String> = all_targets
                .iter()
                .map(|t| format!("{} @ {}", t.project, t.resolution))
                .collect();

            let selection = MultiSelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Select Executables to Build")
                .items(&options)
                .interact()?;

            if selection.is_empty() {
                println!("No targets selected.");
                return Ok(());
            }

            selection
                .into_iter()
                .map(|i| all_targets[i].clone())
                .collect()
        };

    if selected_targets.is_empty() {
        bail!("No matching targets found.");
    }

    // 4. Execution Loop
    fs::create_dir_all(&dist_dir)?;
    println!("📂 Output directory: {:?}", dist_dir);
    println!(); // Spacing

    // SETUP PROGRESS BAR
    let pb = ProgressBar::new(selected_targets.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    for target in selected_targets {
        let display_name = format!("{} [{}]", target.project, target.resolution);
        pb.set_message(format!("Building {}...", display_name));

        match build_single_target(&target, &template_path, &dist_dir) {
            Ok(path) => {
                // Print above the bar
                pb.println(format!(
                    "✅ Created: {:?}",
                    path.file_name().unwrap_or_default()
                ));
            }
            Err(e) => {
                pb.println(format!("❌ Failed {}: {}", display_name, e));
            }
        }
        pb.inc(1);
    }

    pb.finish_with_message("✨ All tasks complete.");
    Ok(())
}
