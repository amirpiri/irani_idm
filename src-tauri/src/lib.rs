mod engine;

use engine::{Manager, ProxySettings};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager as _, State};

#[cfg(target_os = "windows")]
#[link(name = "PowrProf")]
extern "system" {
    fn SetSuspendState(hibernate: u8, force_critical: u8, disable_wake_event: u8) -> u8;
}

const PROJECT_URL: &str = "https://github.com/amirpiri/irani_idm";
const LATEST_RELEASE_API: &str = "https://api.github.com/repos/amirpiri/irani_idm/releases/latest";

#[derive(Clone)]
struct CompletionActionState(Arc<Mutex<String>>);

#[derive(Default)]
struct CompletionTracker {
    known_ids: HashSet<u64>,
    pending_ids: HashSet<u64>,
    batch_had_success: bool,
}

impl CompletionTracker {
    fn seed<'a>(&mut self, downloads: impl IntoIterator<Item = (u64, &'a str)>) {
        for (id, status) in downloads {
            self.known_ids.insert(id);
            if matches!(status, "preparing" | "downloading") {
                self.pending_ids.insert(id);
            }
        }
    }

    fn observe<'a>(&mut self, downloads: impl IntoIterator<Item = (u64, &'a str)>) -> bool {
        let statuses: HashMap<u64, &str> = downloads.into_iter().collect();

        for (&id, &status) in &statuses {
            let is_new = self.known_ids.insert(id);
            if is_new || matches!(status, "preparing" | "downloading") {
                self.pending_ids.insert(id);
            }
        }

        let finished: Vec<u64> = self.pending_ids.iter().copied().filter(|id| {
            matches!(statuses.get(id), Some(&"completed") | Some(&"error"))
                || !statuses.contains_key(id)
        }).collect();
        for id in finished {
            if statuses.get(&id) == Some(&"completed") {
                self.batch_had_success = true;
            }
            self.pending_ids.remove(&id);
        }

        if self.pending_ids.is_empty() && self.batch_had_success {
            self.batch_had_success = false;
            return true;
        }
        false
    }
}

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
fn open_browser_install_guide(browser: String, language: String) -> Result<(), String> {
    let guide_url = browser_install_guide_url(&browser, &language)?;
    open::that(guide_url)
        .map_err(|error| format!("Could not open the installation guide: {error}"))
}

fn browser_install_guide_url(browser: &str, language: &str) -> Result<String, String> {
    if !["chrome", "edge", "firefox"].contains(&browser) {
        return Err("Unsupported browser".into());
    }
    if !["en", "fa"].contains(&language) {
        return Err("Unsupported guide language".into());
    }
    Ok(format!(
        "{PROJECT_URL}/blob/main/docs/extensions/{browser}/README.{language}.md"
    ))
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
fn app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
fn run_completion_action(action: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        match action.as_str() {
            "shutdown" => std::process::Command::new("shutdown.exe")
                .args(["/s", "/t", "0"])
                .spawn()
                .map(|_| ())
                .map_err(|error| format!("Could not start shutdown: {error}")),
            "sleep" => {
                // SetSuspendState is a Win32 API, not a rundll32 entry point.
                // Call it directly to use its actual Windows function signature.
                let result = unsafe { SetSuspendState(0, 0, 0) };
                if result == 0 {
                    Err("Windows could not enter sleep mode".into())
                } else {
                    Ok(())
                }
            }
            "none" => Ok(()),
            _ => return Err("Unsupported download completion action".into()),
        }
    }
    #[cfg(target_os = "linux")]
    {
        let Some(command) = linux_completion_command(&action)? else {
            return Ok(());
        };
        let status = std::process::Command::new("systemctl")
            .arg(command)
            .status()
            .map_err(|error| format!("Could not start systemctl {command}: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("systemctl {command} failed with status {status}"))
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        match action.as_str() {
            "none" => Ok(()),
            "shutdown" | "sleep" => {
                Err("Download completion actions are not supported on this platform".into())
            }
            _ => Err("Unsupported download completion action".into()),
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_completion_command(action: &str) -> Result<Option<&'static str>, String> {
    match action {
        "none" => Ok(None),
        "shutdown" => Ok(Some("poweroff")),
        "sleep" => Ok(Some("suspend")),
        _ => Err("Unsupported download completion action".into()),
    }
}

#[tauri::command]
fn set_completion_action(state: State<CompletionActionState>, action: String) -> Result<(), String> {
    if !["none", "shutdown", "sleep"].contains(&action.as_str()) {
        return Err("Unsupported download completion action".into());
    }
    *state.0.lock().map_err(|_| "Completion action settings are unavailable")? = action;
    Ok(())
}

#[tauri::command]
async fn check_for_update(app: tauri::AppHandle) -> Result<Option<UpdateInfo>, String> {
    let current_version = app.package_info().version.to_string();
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

fn handle_download_link(app: &tauri::AppHandle, raw_url: &str) -> Result<(), String> {
    let link = url::Url::parse(raw_url).map_err(|error| error.to_string())?;
    if link.scheme() != "iranidm" || link.host_str() != Some("download") {
        return Err("Unsupported Irani IDM link".into());
    }
    let target = link.query_pairs()
        .find(|(key, _)| key == "url")
        .map(|(_, value)| value.into_owned())
        .ok_or("Download link does not contain a URL")?;
    let target_url = url::Url::parse(&target).map_err(|error| error.to_string())?;
    if !["http", "https"].contains(&target_url.scheme()) || target_url.host_str().is_none() {
        return Err("Only HTTP and HTTPS downloads are supported".into());
    }

    let manager = app.state::<Manager>();
    let id = manager.add(
        target_url.to_string(),
        downloads_dir().ok_or("Could not find the Downloads folder")?,
        8,
    )?;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = open_download_window(app.clone(), app.state::<Manager>(), id).await {
            eprintln!("Could not open download details: {error}");
        }
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    });
    Ok(())
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
    let content_height: f64 = 640.0;
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
            .maximizable(false)
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

#[tauri::command]
fn get_proxy_settings(m: State<Manager>) -> ProxySettings {
    m.proxy_settings()
}

#[tauri::command]
fn set_proxy_settings(app: tauri::AppHandle, m: State<Manager>, settings: ProxySettings) -> Result<(), String> {
    let enabled = settings.enabled;
    m.set_proxy_settings(settings)?;
    let downloads = m.views(0.5);
    let _ = app.emit("downloads", downloads.clone());
    for download in downloads {
        let label = format!("download-{}", download.id);
        let _ = app.emit_to(
            tauri::EventTarget::webview_window(label),
            "proxy-settings-changed",
            enabled,
        );
    }
    Ok(())
}

#[tauri::command]
async fn test_proxy_settings(settings: ProxySettings) -> Result<String, String> {
    Manager::test_proxy_settings(settings).await
}

pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_deep_link::init());
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            if let Some(config_dir) = dirs::config_dir() {
                migrate_download_history(&config_dir.join("com.example.rustdm"), &dir)?;
            }
            std::fs::create_dir_all(&dir)?;
            let mgr = Manager::load(&dir);
            app.manage(mgr.clone());
            let completion_action = CompletionActionState(Arc::new(Mutex::new("none".into())));
            app.manage(completion_action.clone());
            use tauri_plugin_deep_link::DeepLinkExt;
            if let Some(urls) = app.deep_link().get_current()? {
                for url in urls {
                    if let Err(error) = handle_download_link(app.handle(), url.as_str()) {
                        eprintln!("Rejected browser download link: {error}");
                    }
                }
            }
            let deep_link_app = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    if let Err(error) = handle_download_link(&deep_link_app, url.as_str()) {
                        eprintln!("Rejected browser download link: {error}");
                    }
                }
            });
            #[cfg(any(windows, target_os = "linux"))]
            app.deep_link().register_all()?;
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let (mut last, mut ticks) = (Instant::now(), 0u32);
                // Seed persisted history so it is not mistaken for new work,
                // while enrolling any downloads already active at startup.
                let initial_downloads = mgr.views(0.5);
                let mut completion_tracker = CompletionTracker::default();
                completion_tracker.seed(initial_downloads.iter()
                    .map(|download| (download.id, download.status)));
                loop {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    let dt = last.elapsed().as_secs_f64();
                    last = Instant::now();
                    let downloads = mgr.views(dt);
                    let should_run_action = completion_tracker.observe(downloads.iter()
                        .map(|download| (download.id, download.status)));
                    if should_run_action {
                        let action = completion_action.0.lock()
                            .map(|value| value.clone())
                            .unwrap_or_else(|_| "none".into());
                        if action != "none" {
                            if let Err(error) = run_completion_action(action) {
                                eprintln!("Download completion action failed: {error}");
                                let _ = handle.emit("completion-action-error", error);
                            }
                        }
                    }
                    let _ = handle.emit("downloads", downloads);
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
            open_browser_install_guide,
            open_release_page,
            app_version,
            run_completion_action,
            set_completion_action,
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
            default_dir,
            get_proxy_settings,
            set_proxy_settings,
            test_proxy_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running app");
}

#[cfg(test)]
mod tests {
    use super::{browser_install_guide_url, release_version, CompletionTracker};

    #[test]
    fn browser_install_guide_uses_the_selected_browser_and_language() {
        assert_eq!(
            browser_install_guide_url("chrome", "fa").unwrap(),
            "https://github.com/amirpiri/irani_idm/blob/main/docs/extensions/chrome/README.fa.md"
        );
        assert_eq!(
            browser_install_guide_url("firefox", "en").unwrap(),
            "https://github.com/amirpiri/irani_idm/blob/main/docs/extensions/firefox/README.en.md"
        );
        assert!(browser_install_guide_url("unknown", "en").is_err());
        assert!(browser_install_guide_url("edge", "de").is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_completion_actions_map_to_systemctl_commands() {
        assert_eq!(super::linux_completion_command("none").unwrap(), None);
        assert_eq!(
            super::linux_completion_command("shutdown").unwrap(),
            Some("poweroff")
        );
        assert_eq!(
            super::linux_completion_command("sleep").unwrap(),
            Some("suspend")
        );
        assert!(super::linux_completion_command("invalid").is_err());
    }

    #[test]
    fn release_versions_accept_v_prefix_and_compare_semantically() {
        assert!(release_version("v1.10.0").unwrap() > release_version("1.9.9").unwrap());
        assert!(release_version("v2.0.0-beta.1").unwrap() < release_version("2.0.0").unwrap());
    }

    #[test]
    fn release_versions_reject_invalid_tags() {
        assert!(release_version("release-next").is_err());
    }

    #[test]
    fn paused_download_history_does_not_block_a_new_batch() {
        let mut tracker = CompletionTracker::default();
        tracker.seed([(1, "paused"), (2, "completed")]);

        assert!(tracker.observe([(1, "paused"), (2, "completed"), (3, "completed")]));
    }

    #[test]
    fn waits_for_every_download_in_the_active_batch() {
        let mut tracker = CompletionTracker::default();

        assert!(!tracker.observe([(10, "downloading")]));
        assert!(!tracker.observe([(10, "completed"), (11, "downloading")]));
        assert!(tracker.observe([(10, "completed"), (11, "completed")]));
        assert!(!tracker.observe([(10, "completed"), (11, "completed")]));
    }

    #[test]
    fn observes_a_download_that_finishes_between_monitor_ticks() {
        let mut tracker = CompletionTracker::default();
        tracker.seed([]);

        assert!(tracker.observe([(20, "completed")]));
    }

    #[test]
    fn failed_batch_does_not_run_the_completion_action() {
        let mut tracker = CompletionTracker::default();

        assert!(!tracker.observe([(30, "downloading")]));
        assert!(!tracker.observe([(30, "error")]));
    }
}
