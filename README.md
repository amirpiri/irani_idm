# Rust DM — multi-connection download manager (English / فارسی)

Rust backend (reqwest + tokio) + Tauri 2 shell. The UI is plain HTML/CSS/JS in `ui/`
(no npm needed), with full RTL support for Persian.

## Features
- Multi-segment downloads (default 8 connections, up to 32) with HTTP Range
- Pause / resume (also after restarting the app), auto-retry, stall timeout
- Per-download speed limit, live speed / ETA, per-segment details dialog
- Category groups, search, queue start/stop all
- English + Persian UI (switch in Settings; RTL layout applied automatically)

## Prerequisites
- Rust (stable): https://rustup.rs
- `cargo install tauri-cli --version "^2"`
- **Windows:** WebView2 (preinstalled on Win 10/11) + MSVC Build Tools
- **Linux (Debian/Ubuntu):**
  `sudo apt install libwebkit2gtk-4.1-dev build-essential libssl-dev libxdo-dev libayatana-appindicator3-dev librsvg2-dev`

## Run / build
```
cd src-tauri
cargo tauri icon path/to/logo.png   # once: generates the icons/ folder
cargo tauri dev                      # run
cargo tauri build                    # installers
```
Installers appear in `src-tauri/target/release/bundle/`:
Windows → `.msi` / `.exe` (NSIS); Linux → `.deb`, `.rpm`, `.AppImage`.
Build on each target OS (or use GitHub Actions with a windows + ubuntu matrix).

## Build with GitHub Actions (no local setup)
1. Push this folder to a GitHub repository.
2. Open the **Actions** tab, select **Build**, and click **Run workflow** (it also runs on every push to `main` and on pull requests).
3. When it finishes, download the installers from the run's **Artifacts**
   (`rust-dm-Windows`: .msi + .exe, `rust-dm-Linux`: .deb + .rpm + .AppImage).
4. To publish a release: `git tag v0.1.0 && git push origin v0.1.0` — the installers are attached to a GitHub Release automatically.

Icons are generated from `app-icon.png` if `src-tauri/icons/` doesn't exist. Replace `app-icon.png` with your own logo (1024x1024 PNG) any time.

## Ideas for next steps
Scheduler, browser extension / clipboard capture, download queue concurrency limit,
proxy + auth settings, YouTube/HLS support, system tray, bundling the Vazirmatn font.
