use super::types::{RunnerMode, RunnerStatus};
use anyhow::{Context, Result};
use std::{path::Path, process::Command, thread, time::Duration};

/// Manages the lifecycle of the target executable.
pub fn process_runner<F>(exe_path: &Path, mode: RunnerMode, callback: F) -> Result<()>
where
    F: Fn(RunnerStatus) + Send + Clone + 'static,
{
    let exe_name = exe_path.file_name().unwrap().to_string_lossy().to_string();
    callback(RunnerStatus::Starting(exe_name.clone()));

    match mode {
        RunnerMode::Detach => {
            Command::new(exe_path)
                .spawn()
                .context("Failed to detach process")?;
            callback(RunnerStatus::Detached);
            Ok(())
        }
        RunnerMode::Watchdog => {
            // Infinite Loop
            loop {
                let mut child = Command::new(exe_path)
                    .spawn()
                    .context("Failed to spawn process")?;

                callback(RunnerStatus::Running(child.id()));

                // Block until process exits
                let _ = child.wait();

                callback(RunnerStatus::Restarting);

                // Wait before restart to prevent CPU thrashing if immediate crash
                thread::sleep(Duration::from_secs(2));
            }
        }
    }
}
