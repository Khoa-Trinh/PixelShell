# ✨ Pixel Shell

**High-Performance Desktop Overlay Engine & Asset Factory**

Pixel Shell is a specialized engine designed to render high-framerate, transparent video overlays on Windows with minimal resource usage. It utilizes a custom **"Snowplow" RLE compression algorithm** to render uncompressed video frames directly via GDI, bypassing standard video players for absolute background transparency.

The project features a unique **Binary Patching Architecture**: instead of compiling code for every video, the CLI injects compressed asset data directly into a pre-compiled generic **Runner** executable, creating standalone, portable `.exe` files instantly.

---

## 📥 Download Pre-built Binaries

Don’t want to build from source?

You can download the latest ready-to-use versions of the tools directly from **GitHub Releases**.

* **ps-gui.exe** — The Visual Interface
* **ps-cli.exe** — The Command Line Builder
* **ps-runner.exe** — The Template Engine

Place them in the same folder, and you are ready to go.

---

## 🚀 Features

* ⚡ **Zero-Copy Rendering** — Custom `.bin` format optimized for CPU-based sparse rendering
* 🔊 **Audio Sync** — High-priority audio thread using `kira` for precise A/V synchronization
* 📦 **Standalone Output** — Generates single-file `.exe` overlays with no external dependencies
* 🖥️ **Visual Interface** — User-friendly GUI for managing projects, downloads, and builds without using the terminal
* 🛠️ **All-in-One CLI** — Advanced Download, Convert, Debug, and Build tools for automation
* 🛡️ **Watchdog Mode** — Automatically restarts overlays if they crash or are closed

---

## 📂 Project Structure

This is a Cargo workspace organized into applications and shared libraries.

```text
pixel-shell/
├── apps/
│   ├── ps-cli/          # Command Line Interface (backend logic)
│   ├── ps-gui/          # GUI frontend (egui-based visual tool)
│   └── ps-runner/       # Template EXE (player engine)
├── crates/
│   ├── ps-core/         # Shared data structures (PixelRect, headers)
│   └── ps-factory/      # Binary building & patching logic
├── target/              # Build artifacts
├── pixel-shell.ico      # Application icon
└── Cargo.toml           # Workspace configuration
```

---

## 🛠️ Building from Source

If you want to contribute or modify the engine, follow these steps.

### Prerequisites

* Rust (via Rustup)
* FFmpeg & FFprobe (required for asset conversion)
* yt-dlp (required for downloading source material)

### Compilation

You must build the entire workspace to generate the GUI, CLI, and Runner template.

```bash
git clone https://github.com/Khoa-Trinh/PixelShell.git
cd PixelShell
cargo build --release
```

### Assemble the Toolset

Create a working folder (e.g., `PixelShellTool`) and copy the artifacts:

```text
target/release/ps-gui.exe    -> PixelShellTool/ps-gui.exe
target/release/ps-cli.exe    -> PixelShellTool/ps-cli.exe
target/release/ps-runner.exe -> PixelShellTool/ps-runner.exe
```

---

## 🖥️ GUI Usage Guide

For the easiest experience, use the graphical interface.

1. Launch **ps-gui.exe**.
2. **Configuration**: On first run, go to the *Settings* tab and ensure the paths to `ffmpeg`, `yt-dlp`, and the `ps-runner.exe` template are correct.
3. **Workflow**:

   * **Download**: Paste a YouTube URL, select a resolution (1080p / 720p / etc.), and name your project.
   * **Output Settings**: Select the desired framerate (30 / 60 FPS) and resolution.
   * **Process**: Click **Run All Tasks** to automatically download, convert, and build the standalone executable.
   * **Manage**: Use the *Runner* tab to launch and monitor your generated overlays with the built-in Watchdog.

---

## 💻 CLI Usage Guide

For automation or advanced usage, open a terminal in the folder containing the executables.

### 1. Download Content

Downloads a video, extracts audio, and prepares it for processing.

```bash
ps-cli.exe download --url "https://youtu.be/..." --resolution 1080p --project "my_overlay"
```

### 2. Convert Assets

Transcodes video frames into the optimized `.bin` format using the Snowplow algorithm.

```bash
ps-cli.exe convert --project "my_overlay" --resolutions "1080p,720p" --use-gpu
```

### 3. Build Standalone EXE

Injects converted assets into the runner template.

```bash
ps-cli.exe build --project "my_overlay" --resolutions "1080p,720p"
# Output will be placed in the /dist folder
```

### 4. Run the Overlay

Running via command line instead of double-clicking enables Watchdog mode.

```bash
ps-cli.exe run --target "my_overlay_1080p.exe"
```

---

## 🔧 Troubleshooting

* **Template not found** — Ensure `ps-runner.exe` is in the same folder as the CLI / GUI executable.
* **FFmpeg not found** — Ensure FFmpeg is installed and added to your system PATH, or configure the absolute path in the GUI settings.
* **Black Background** — Ensure your source video has a solid black background (`#000000`) for the transparency engine to work correctly.

---

## 📜 License

This project is licensed under the **MIT License**.
