use super::{
    core::process_debug_session,
    types::{DebugArgs, DebugJob, DebugStatus},
};
use anyhow::{bail, Result};
use dialoguer::{theme::ColorfulTheme, Select};
use std::{env, fs};

pub fn run_cli(_args: DebugArgs) -> Result<()> {
    let current_exe = env::current_exe()?;
    let exe_dir = current_exe.parent().unwrap();
    let assets_dir = exe_dir.join("assets");
    let dist_dir = exe_dir.join("dist");

    // 1. Select Mode: Check Project (Bin) OR Check Build (Exe)
    let modes = vec!["Inspect Project (.bin)", "Inspect Build (.exe)"];
    let mode_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select Debug Mode")
        .items(&modes)
        .default(0)
        .interact()?;

    let target_path = if mode_idx == 0 {
        // --- SELECT .BIN ---
        if !assets_dir.exists() {
            bail!("Assets folder missing.");
        }

        // Pick Project
        let projects: Vec<_> = fs::read_dir(&assets_dir)?.filter_map(|e| e.ok()).collect();
        let p_names: Vec<String> = projects
            .iter()
            .map(|p| p.file_name().to_string_lossy().into())
            .collect();

        if p_names.is_empty() {
            bail!("No projects found in assets/.");
        }

        let p_idx = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select Project")
            .items(&p_names)
            .interact()?;
        let p_path = projects[p_idx].path();

        // Pick File
        let bins: Vec<_> = fs::read_dir(&p_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "bin"))
            .collect();
        let b_names: Vec<String> = bins
            .iter()
            .map(|b| b.file_name().to_string_lossy().into())
            .collect();

        if b_names.is_empty() {
            bail!("No .bin files found in selected project.");
        }

        let b_idx = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select File")
            .items(&b_names)
            .interact()?;
        bins[b_idx].path()
    } else {
        // --- SELECT .EXE ---
        if !dist_dir.exists() {
            bail!("Dist folder missing. Build something first.");
        }

        let exes: Vec<_> = fs::read_dir(&dist_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "exe"))
            .collect();

        if exes.is_empty() {
            bail!("No executables found in dist.");
        }

        let e_names: Vec<String> = exes
            .iter()
            .map(|e| e.file_name().to_string_lossy().into())
            .collect();
        let e_idx = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select Executable")
            .items(&e_names)
            .interact()?;
        exes[e_idx].path()
    };

    println!("Debugging: {:?}", target_path);
    let job = DebugJob {
        file_path: target_path,
    };

    process_debug_session(job, |status| {
        if let DebugStatus::Error(e) = status {
            eprintln!("Error: {}", e);
        }
    })?;

    Ok(())
}
