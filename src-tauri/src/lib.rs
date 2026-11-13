mod engine;

use engine::Manager;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager as _, State};

const PROJECT_URL: &str = "https://github.com/amirpiri/irani_idm";
const LATEST_RELEASE_API: &str = "https://api.github.com/repos/amirpiri/irani_idm/releases/latest";

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

#[derive(Serialize)]
struct UpdateInfo {
    current_version: String,
    latest_version: String,
    download_url: String,
}

fn release_version(tag: &str) -> Result<Version, String> {
    Version::parse(tag.strip_prefix('v').unwrap_or(tag))
        .map_err(|error| format!("Invalid release version '{tag}': {error}"))
}

#[tauri::command]
fn open_project_page() -> Result<(), String> {
    open::that(PROJECT_URL).map_err(|error| error.to_string())
}

#[tauri::command]
fn open_release_page(url: String) -> Result<(), String> {
    let parsed = url::Url::parse(&url).map_err(|error| error.to_string())?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("github.com")
        || !parsed
            .path()
            .starts_with("/amirpiri/irani_idm/releases/tag/")
    {
        return Err("Invalid release page URL".into());
    }
    open::that(parsed.as_str()).map_err(|error| error.to_string())
}

#[tauri::command]
async fn check_for_update() -> Result<Option<UpdateInfo>, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let current = release_version(&current_version)?;
    let client = reqwest::Client::builder()
        .user_agent(concat!("Irani-IDM/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| error.to_string())?;
    let release: GitHubRelease = client
        .get(LATEST_RELEASE_API)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let latest = release_version(&release.tag_name)?;
    if latest <= current {
        return Ok(None);
    }

    let parsed_url = url::Url::parse(&release.html_url).map_err(|error| error.to_string())?;
    if parsed_url.scheme() != "https"
        || parsed_url.host_str() != Some("github.com")
        || !parsed_url
            .path()
            .starts_with("/amirpiri/irani_idm/releases/tag/")
    {
        return Err("GitHub returned an invalid release page URL".into());
    }

    Ok(Some(UpdateInfo {
        current_version,
        latest_version: release.tag_name,
        download_url: release.html_url,
    }))
}

fn downloads_dir() -> Option<PathBuf> {
    dirs::download_dir().or_else(|| dirs::home_dir().map(|home| home.join("Downloads")))
}

fn migrate_download_history(old_data_dir: &Path, new_data_dir: &Path) -> std::io::Result<()> {
    let old_store = old_data_dir.join("downloads.json");
    let new_store = new_data_dir.join("downloads.json");
    if !new_store.exists() && old_store.is_file() {
        std::fs::create_dir_all(new_data_dir)?;
        std::fs::copy(old_store, new_store)?;
    }
    Ok(())
}

#[tauri::command]
fn add_download(
    m: State<'_, Manager>,
    url: String,
    dir: Option<String>,
    connections: Option<u32>,
) -> Result<u64, String> {
    let dir = dir
        .filter(|d| !d.trim().is_empty())
        .map(PathBuf::from)
        .or_else(downloads_dir)
        .ok_or("no download directory")?;
    m.add(url, dir, connections.unwrap_or(8))
}

#[tauri::command]
fn pause(m: State<Manager>, id: u64) {
    m.pause(id)
}

#[tauri::command]
fn resume(m: State<Manager>, id: u64) {
    m.resume(id)
}

#[tauri::command]
fn pause_all(m: State<Manager>) {
    m.pause_all()
}

#[tauri::command]
fn resume_all(m: State<Manager>) {
    m.resume_all()
}

#[tauri::command]
fn set_limit(m: State<Manager>, id: u64, kbps: u64) {
    m.set_limit(id, kbps)
}

#[tauri::command]
async fn remove(m: State<'_, Manager>, id: u64, delete_file: bool) -> Result<(), String> {
    m.remove(id, delete_file).await
}

#[tauri::command]
fn download_view(m: State<Manager>, id: u64) -> Result<engine::View, String> {
    m.view(id).ok_or_else(|| "Download not found".into())
}

#[tauri::command]
async fn open_download_window(
    app: tauri::AppHandle,
    m: State<'_, Manager>,
    id: u64,
) -> Result<(), String> {
    let download = m.view(id).ok_or_else(|| "Download not found".to_string())?;
    let label = format!("download-{id}");
    if let Some(window) = app.get_webview_window(&label) {
        return window.set_focus().map_err(|e| e.to_string());
    }

    let (screen_width, screen_height): (f64, f64) = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .map(|monitor| {
            let scale_factor = monitor.scale_factor();
            let work_area = monitor.work_area().size;
            (
                work_area.width as f64 / scale_factor,
                work_area.height as f64 / scale_factor,
            )
        })
        .unwrap_or((1024.0, 768.0));
    let min_width = screen_width.min(560.0);
    let min_height = screen_height.min(480.0);
    let window_width = 820.0_f64.min((screen_width * 0.9).max(min_width));
    let max_height = (screen_height * 0.9).max(min_height);
    let content_height: f64 = 560.0 + download.segment_count() as f64 * 32.0;
    let window_height = content_height.clamp(min_height, max_height);
    let title = format!("{} - Irani IDM", download.name);
    let window_app = app.clone();
    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let result = if let Some(window) = window_app.get_webview_window(&label) {
            window.set_focus().map_err(|e| e.to_string())
        } else {
            tauri::WebviewWindowBuilder::new(
                &window_app,
                &label,
                tauri::WebviewUrl::App("download.html".into()),
            )
            .title(title)
            .resizable(true)
            .inner_size(window_width, window_height)
            .min_inner_size(min_width, min_height)
            .build()
            .map(|_| ())
            .map_err(|e| e.to_string())
        };
        let _ = result_tx.send(result);
    })
    .map_err(|e| e.to_string())?;

    result_rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn open_item(m: State<Manager>, id: u64, folder: bool) -> Result<(), String> {
    let p = m.path_of(id).ok_or("not found")?;
    #[cfg(target_os = "windows")]
    if folder {
        use std::os::windows::process::CommandExt;

        let mut command = std::process::Command::new("explorer.exe");
        command.raw_arg(format!("/select,\"{}\"", p.display()));
        return command.spawn().map(|_| ()).map_err(|e| e.to_string());
    }
    let target = if folder {
        p.parent().map(|x| x.to_path_buf()).ok_or("no parent")?
    } else {
        p
    };
    open::that(target).map_err(|e| e.to_string())
}

#[tauri::command]
fn default_dir() -> String {
    downloads_dir()
        .map(|p| p.to_string_lossy().into())
        .unwrap_or_default()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            if let Some(config_dir) = dirs::config_dir() {
                migrate_download_history(&config_dir.join("com.example.rustdm"), &dir)?;
            }
            std::fs::create_dir_all(&dir)?;
            let mgr = Manager::load(&dir);
            app.manage(mgr.clone());
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let (mut last, mut ticks) = (Instant::now(), 0u32);
                loop {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    let dt = last.elapsed().as_secs_f64();
                    last = Instant::now();
                    let _ = handle.emit("downloads", mgr.views(dt));
                    ticks += 1;
                    if ticks % 10 == 0 {
                        mgr.save();
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_project_page,
            open_release_page,
            check_for_update,
            add_download,
            pause,
            resume,
            pause_all,
            resume_all,
            set_limit,
            remove,
            download_view,
            open_download_window,
            open_item,
            default_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running app");
}

#[cfg(test)]
mod tests {
    use super::release_version;

    #[test]
    fn release_versions_accept_v_prefix_and_compare_semantically() {
        assert!(release_version("v1.10.0").unwrap() > release_version("1.9.9").unwrap());
        assert!(release_version("v2.0.0-beta.1").unwrap() < release_version("2.0.0").unwrap());
    }

    #[test]
    fn release_versions_reject_invalid_tags() {
        assert!(release_version("release-next").is_err());
    }
}
