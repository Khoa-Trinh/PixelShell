use super::{
    core::build_single_target,
    types::{BuildStatus, BuildTarget},
};
use anyhow::Result;
use std::{env, fs, sync::mpsc::Sender, thread};

pub fn run_async(targets: Vec<BuildTarget>, sender: Sender<BuildStatus>) -> Result<()> {
    let current_exe = env::current_exe()?;
    let exe_dir = current_exe.parent().unwrap();
    let template_path = exe_dir.join("ps-runner.exe");
    let dist_dir = exe_dir.join("dist");

    if !template_path.exists() {
        let _ = sender.send(BuildStatus::Error("Missing ps-runner.exe template".into()));
        return Ok(());
    }

    fs::create_dir_all(&dist_dir)?;

    thread::spawn(move || {
        let _ = sender.send(BuildStatus::Starting);

        for target in targets {
            let name = format!("{}_{}", target.project, target.resolution);
            let _ = sender.send(BuildStatus::Building(name));

            match build_single_target(&target, &template_path, &dist_dir) {
                Ok(path) => {
                    let _ = sender.send(BuildStatus::Finished(path));
                }
                Err(e) => {
                    let _ = sender.send(BuildStatus::Error(e.to_string()));
                }
            }
        }
    });

    Ok(())
}
