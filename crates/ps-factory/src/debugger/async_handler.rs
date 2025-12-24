use super::{
    core::process_debug_session,
    types::{DebugJob, DebugStatus},
};
use anyhow::Result;
use std::{path::PathBuf, sync::mpsc::Sender, thread};

pub fn run_async(file_path: PathBuf, sender: Sender<DebugStatus>) -> Result<()> {
    thread::spawn(move || {
        let tx = sender.clone();
        let job = DebugJob { file_path };
        let result = process_debug_session(job, move |status| {
            let _ = tx.send(status);
        });
        if let Err(e) = result {
            let _ = sender.send(DebugStatus::Error(e.to_string()));
        }
    });
    Ok(())
}
