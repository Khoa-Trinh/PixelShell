use super::{
    core::process_runner,
    types::{RunnerMode, RunnerStatus},
    utils::resolve_target,
};
use anyhow::Result;
use std::{env, sync::mpsc::Sender, thread};

pub fn run_async(target_name: Option<String>, sender: Sender<RunnerStatus>) -> Result<()> {
    // 1. Resolve Path
    let current_exe = env::current_exe()?;
    let exe_dir = current_exe.parent().unwrap();
    let dist_dir = exe_dir.join("dist");

    // NOTE: resolve_target might prompt interactively if target_name is None.
    // In a GUI context, you usually pass the specific target so it doesn't block.
    // If target_name is None, the GUI thread might hang waiting for console input that never comes.
    // Ensure the GUI always passes a name or handle selection in GUI logic first.
    let selected_path = match resolve_target(&dist_dir, target_name.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            let _ = sender.send(RunnerStatus::Error(e.to_string()));
            return Ok(());
        }
    };

    // 2. Spawn Background Thread
    thread::spawn(move || {
        let tx_callback = sender.clone();

        // GUI usually implies "Watchdog" mode (keeping it alive)
        let result = process_runner(&selected_path, RunnerMode::Watchdog, move |status| {
            let _ = tx_callback.send(status);
        });

        if let Err(e) = result {
            let _ = sender.send(RunnerStatus::Error(e.to_string()));
        }
    });

    Ok(())
}
