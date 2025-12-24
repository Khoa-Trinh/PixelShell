use anyhow::{bail, Result};
use dialoguer::{theme::ColorfulTheme, Select};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn resolve_target(dist_dir: &Path, target_hint: Option<&str>) -> Result<PathBuf> {
    if !dist_dir.exists() {
        bail!("'dist' folder not found. Run 'build' first.");
    }

    let mut executables: Vec<(String, PathBuf)> = fs::read_dir(dist_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let p = e.path();
            p.is_file() && (p.extension().map_or(false, |ext| ext == "exe") || cfg!(unix))
        })
        .map(|e| (e.file_name().to_string_lossy().to_string(), e.path()))
        .collect();

    executables.sort_by(|a, b| a.0.cmp(&b.0));

    if executables.is_empty() {
        bail!("No executables found in 'dist/'.");
    }

    if let Some(name) = target_hint {
        let matches: Vec<&PathBuf> = executables
            .iter()
            .filter(|(n, _)| n.contains(name))
            .map(|(_, p)| p)
            .collect();

        match matches.len() {
            0 => bail!("No match for '{}'", name),
            1 => Ok(matches[0].clone()),
            _ => {
                println!("Ambiguous match for '{}', picking: {:?}", name, matches[0]);
                Ok(matches[0].clone())
            }
        }
    } else {
        // Interactive Selection (CLI context mostly, but logic resides here)
        // If called blindly by async handler with None, this might hang if stdio is captured.
        // However, resolve_target is designed for this dual use.
        // For strictly non-interactive usage, caller should ensure target_hint is Some
        // OR rely on us picking the first one if we want that behavior.
        // But the original code used dialoguer here.
        let options: Vec<String> = executables.iter().map(|(n, _)| n.clone()).collect();
        let idx = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select Executable")
            .items(&options)
            .default(0)
            .interact()?;
        Ok(executables[idx].1.clone())
    }
}
