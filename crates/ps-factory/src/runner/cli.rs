use super::{
    core::process_runner,
    types::{RunArgs, RunnerMode, RunnerStatus},
    utils::resolve_target,
};
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};
use std::{env, thread, time::Duration};

pub fn run_cli(args: RunArgs) -> Result<()> {
    // 1. Setup Paths
    let current_exe = env::current_exe()?;
    let exe_dir = current_exe.parent().unwrap();
    let dist_dir = exe_dir.join("dist");

    // 2. Select Executable
    let selected_path = resolve_target(&dist_dir, args.target.as_deref())?;

    // 3. Determine Mode
    let (mode, is_silent) = if args.detach {
        (RunnerMode::Detach, false)
    } else if args.silent {
        (RunnerMode::Watchdog, true)
    } else {
        // Interactive Selection
        let modes = vec![
            "Normal (Watchdog visible)",
            "Silent (Watchdog hidden, auto-restart)",
            "Detach (Run once & exit CLI)",
        ];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select Run Mode")
            .default(0)
            .items(&modes)
            .interact()?;

        match selection {
            0 => (RunnerMode::Watchdog, false),
            1 => (RunnerMode::Watchdog, true),
            2 => (RunnerMode::Detach, false),
            _ => (RunnerMode::Watchdog, false),
        }
    };

    // 4. Handle "Silent" Mode (CLI Specific)
    if is_silent {
        #[cfg(target_os = "windows")]
        unsafe {
            use windows::Win32::System::Console::GetConsoleWindow;
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

            let hwnd = GetConsoleWindow();
            if hwnd.0 != 0 {
                println!("👻 Going Silent! (Check Task Manager to stop 'ps-cli')");
                thread::sleep(Duration::from_millis(500));
                ShowWindow(hwnd, SW_HIDE);
            }
        }
    } else if mode == RunnerMode::Watchdog {
        println!(
            "✅ Watchdog Active for: {:?}",
            selected_path.file_name().unwrap()
        );
        println!("   (Press Ctrl+C to stop)");
    }

    // 5. Run Logic
    process_runner(&selected_path, mode, |status| match status {
        RunnerStatus::Starting(name) => println!("🚀 Starting: {}", name),
        RunnerStatus::Running(pid) => println!("   -> PID: {}", pid),
        RunnerStatus::Restarting => println!("🔄 Process exited. Restarting in 2s..."),
        RunnerStatus::Detached => println!("👋 Detached successfully."),
        RunnerStatus::Error(e) => eprintln!("❌ Error: {}", e),
    })?;

    Ok(())
}
