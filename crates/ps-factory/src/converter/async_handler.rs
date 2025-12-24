use super::core::process_conversion;
use super::types::{ConvertJob, ConverterStatus};
use anyhow::Result;
use std::sync::mpsc::Sender;
use std::thread;

pub fn run_async(job: ConvertJob, sender: Sender<ConverterStatus>) -> Result<()> {
    // We spawn a new thread so the GUI doesn't freeze
    thread::spawn(move || {
        // FIX: Create a CLONE for the callback to use
        let tx_callback = sender.clone();

        let result = process_conversion(job, move |status| {
            // Use the CLONE inside the loop
            let _ = tx_callback.send(status);
        });

        // If an error happens, we still have the ORIGINAL 'sender' to report it
        if let Err(e) = result {
            let _ = sender.send(ConverterStatus::Error(e.to_string()));
        }
    });
    Ok(())
}
