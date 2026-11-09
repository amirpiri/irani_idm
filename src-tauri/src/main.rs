#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod engine;
use engine::Manager;
use std::{path::PathBuf, time::{Duration, Instant}};
use tauri::{Emitter, Manager as _, State};

#[tauri::command]
async fn add_download(m: State<'_, Manager>, url: String, dir: Option<String>, connections: Option<u32>) -> Result<u64, String> {
    let dir = dir
        .filter(|d| !d.trim().is_empty())
        .map(PathBuf::from)
        .or_else(dirs::download_dir)
        .ok_or("no download directory")?;
    m.add(url, dir, connections.unwrap_or(8)).await
}
#[tauri::command] fn pause(m: State<Manager>, id: u64) { m.pause(id) }
#[tauri::command] fn resume(m: State<Manager>, id: u64) { m.resume(id) }
#[tauri::command] fn pause_all(m: State<Manager>) { m.pause_all() }
#[tauri::command] fn resume_all(m: State<Manager>) { m.resume_all() }
#[tauri::command] fn set_limit(m: State<Manager>, id: u64, kbps: u64) { m.set_limit(id, kbps) }
#[tauri::command] fn remove(m: State<Manager>, id: u64, delete_file: bool) { m.remove(id, delete_file) }
#[tauri::command]
fn open_item(m: State<Manager>, id: u64, folder: bool) -> Result<(), String> {
    let p = m.path_of(id).ok_or("not found")?;
    let target = if folder { p.parent().map(|x| x.to_path_buf()).ok_or("no parent")? } else { p };
    open::that(target).map_err(|e| e.to_string())
}
#[tauri::command]
fn default_dir() -> String {
    dirs::download_dir().map(|p| p.to_string_lossy().into()).unwrap_or_default()
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
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
                    if ticks % 10 == 0 { mgr.save(); }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            add_download, pause, resume, pause_all, resume_all, set_limit, remove, open_item, default_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running app");
}
