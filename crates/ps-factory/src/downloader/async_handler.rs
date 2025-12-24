use super::core::process_download;
use super::types::{DownloadJob, DownloadStatus};
use super::utils::check_dependencies;
use anyhow::Result;
use std::sync::mpsc::Sender;
use std::thread;

pub fn run_async(job: DownloadJob, sender: Sender<DownloadStatus>) -> Result<()> {
    check_dependencies()?;
    // Spawning a thread here ensures the GUI never blocks
    thread::spawn(move || {
        let result = process_download(job, |status| {
            let _ = sender.send(status);
        });
        if let Err(e) = result {
            let _ = sender.send(DownloadStatus::Error(e.to_string()));
        }
    });
    Ok(())
}
