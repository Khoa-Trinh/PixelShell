use clap::{Parser, Subcommand};
use ps_factory::{builder, converter, debugger, downloader, runner};

#[derive(Parser)]
#[command(name = "Pixel Shell Factory")]
#[command(version = "1.0")]
#[command(about = "All-in-one tool for creating Pixel Shell overlays")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // 1. Download
    Download {
        #[arg(short, long)]
        url: Option<String>,

        #[arg(short, long)]
        resolution: Option<String>,

        #[arg(short, long)]
        fps: Option<u32>,

        #[arg(short, long)]
        project: Option<String>,
    },

    // 2. Convert
    Convert {
        #[arg(short, long)]
        project: Option<String>,

        #[arg(short, long)]
        resolutions: Option<String>,

        #[arg(long, default_value_t = false)]
        gpu: bool,
    },

    // 3. Build
    Build {
        #[arg(short, long)]
        project: Option<String>,

        #[arg(short, long)]
        resolutions: Option<String>,

        #[arg(short, long, default_value_t = false)]
        all: bool,
    },

    // 4. Run
    Run {
        #[arg(short, long)]
        target: Option<String>,

        #[arg(short, long, default_value_t = false)]
        silent: bool,

        #[arg(short, long, default_value_t = false)]
        detach: bool,
    },

    // 5. Debug
    Debug {
        #[arg(short, long)]
        project: Option<String>,

        #[arg(short, long)]
        file: Option<String>,
    },

    // 6. ALL (Pipeline)
    /// Runs Download -> Convert -> Build -> Run in sequence
    All {
        // --- Downloader Args ---
        #[arg(short, long)]
        url: Option<String>,

        #[arg(short, long)]
        resolution: Option<String>,

        #[arg(short, long)]
        fps: Option<u32>,

        #[arg(short, long)]
        project: Option<String>, // Critical for linking steps

        // --- Converter Args ---
        #[arg(long, default_value_t = false)]
        gpu: bool,

        // --- Runner Args ---
        #[arg(short, long, default_value_t = false)]
        silent: bool,

        #[arg(short, long, default_value_t = false)]
        detach: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        // [1] DOWNLOAD
        Commands::Download {
            url,
            resolution,
            fps,
            project,
        } => {
            let args = downloader::DownloadArgs {
                url: url.clone(),
                resolution: resolution.clone(),
                fps: *fps,
                project_name: project.clone(),
            };
            if let Err(e) = downloader::run_cli(args) {
                eprintln!("❌ Download Error: {}", e);
            }
        }

        // [2] CONVERT
        Commands::Convert {
            project,
            resolutions,
            gpu,
        } => {
            let args = converter::ConvertArgs {
                project_name: project.clone(),
                resolutions: resolutions.clone(),
                use_gpu: *gpu,
            };
            if let Err(e) = converter::run_cli(args) {
                eprintln!("❌ Conversion Error: {}", e);
            }
        }

        // [3] BUILD
        Commands::Build {
            project,
            resolutions,
            all,
        } => {
            let args = builder::BuildArgs {
                project_name: project.clone(),
                resolutions: resolutions.clone(),
                build_all: *all,
            };
            if let Err(e) = builder::run_cli(args) {
                eprintln!("❌ Build Error: {}", e);
            }
        }

        // [4] RUNNER
        Commands::Run {
            target,
            silent,
            detach,
        } => {
            let args = runner::RunArgs {
                target: target.clone(),
                silent: *silent,
                detach: *detach,
            };
            if let Err(e) = runner::run_cli(args) {
                eprintln!("❌ Runner Error: {}", e);
            }
        }

        // [5] DEBUG
        Commands::Debug { project, file } => {
            let args = debugger::DebugArgs {
                project_name: project.clone(),
                file_name: file.clone(),
            };
            if let Err(e) = debugger::run_cli(args) {
                eprintln!("❌ Debugger Error: {}", e);
            }
        }

        // [6] ALL (The Pipeline)
        Commands::All {
            url,
            resolution,
            fps,
            project,
            gpu,
            silent,
            detach,
        } => {
            println!("🚀 STARTING PIPELINE (All-in-One)");

            // Step 1: Download
            println!("\n▶ STEP 1: DOWNLOAD");
            let dl_args = downloader::DownloadArgs {
                url: url.clone(),
                resolution: resolution.clone(),
                fps: *fps,
                project_name: project.clone(),
            };
            if let Err(e) = downloader::run_cli(dl_args) {
                eprintln!("❌ Pipeline stopped at Download: {}", e);
                return;
            }

            // Step 2: Convert
            // We reuse the project name (if provided) to auto-select the project
            println!("\n▶ STEP 2: CONVERT");
            let cv_args = converter::ConvertArgs {
                project_name: project.clone(),
                resolutions: resolution.clone(), // Use same resolution preference
                use_gpu: *gpu,
            };
            if let Err(e) = converter::run_cli(cv_args) {
                eprintln!("❌ Pipeline stopped at Conversion: {}", e);
                return;
            }

            // Step 3: Build
            println!("\n▶ STEP 3: BUILD");
            let bd_args = builder::BuildArgs {
                project_name: project.clone(),
                resolutions: resolution.clone(),
                build_all: false,
            };
            if let Err(e) = builder::run_cli(bd_args) {
                eprintln!("❌ Pipeline stopped at Build: {}", e);
                return;
            }

            // Step 4: Run
            println!("\n▶ STEP 4: RUN");
            // If project name is known, pass it as target hint to find the right EXE
            let run_args = runner::RunArgs {
                target: project.clone(),
                silent: *silent,
                detach: *detach,
            };
            if let Err(e) = runner::run_cli(run_args) {
                eprintln!("❌ Pipeline stopped at Runner: {}", e);
                return;
            }

            println!("\n✨ PIPELINE COMPLETE ✨");
        }
    }
}
