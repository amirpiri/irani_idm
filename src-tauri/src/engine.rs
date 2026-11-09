//! Multi-segment, resumable HTTP download engine.
use futures_util::StreamExt;
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::SeekFrom,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering::Relaxed},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tokio::{
    fs::OpenOptions,
    io::{AsyncSeekExt, AsyncWriteExt},
    time::sleep,
};

const WAITING: u8 = 0;
const RECV: u8 = 1;
const DONE: u8 = 2;

pub struct Seg {
    start: u64,
    end: u64, // inclusive; u64::MAX = unknown length
    done: AtomicU64,
    state: AtomicU8,
}
impl Seg {
    fn len(&self) -> u64 {
        if self.end == u64::MAX { u64::MAX } else { self.end - self.start + 1 }
    }
}

#[derive(Clone, PartialEq)]
enum Status {
    Paused,
    Downloading,
    Completed,
    Error(String),
}

struct Item {
    url: String,
    path: PathBuf,
    total: Option<u64>,
    resumable: bool,
    status: Status,
    segs: Vec<Arc<Seg>>,
    cancel: Arc<AtomicBool>,
    limit: Arc<AtomicU64>, // KB/s, 0 = unlimited
    speed: f64,
    last: u64,
}

#[derive(Serialize, Deserialize)]
struct SavedSeg { start: u64, end: u64, done: u64 }
#[derive(Serialize, Deserialize)]
struct Saved {
    id: u64,
    url: String,
    path: PathBuf,
    total: Option<u64>,
    resumable: bool,
    completed: bool,
    error: Option<String>,
    limit_kbps: u64,
    segs: Vec<SavedSeg>,
}

#[derive(Serialize, Clone)]
pub struct SegView { done: u64, total: Option<u64>, state: u8 }
#[derive(Serialize, Clone)]
pub struct View {
    id: u64,
    name: String,
    url: String,
    path: String,
    total: Option<u64>,
    downloaded: u64,
    speed: u64,
    eta: Option<u64>,
    status: &'static str,
    error: Option<String>,
    resumable: bool,
    limit_kbps: u64,
    segments: Vec<SegView>,
}

#[derive(Clone)]
pub struct Manager {
    items: Arc<Mutex<HashMap<u64, Item>>>,
    client: reqwest::Client,
    store: PathBuf,
    next_id: Arc<AtomicU64>,
}

impl Manager {
    pub fn load(data_dir: &Path) -> Self {
        let store = data_dir.join("downloads.json");
        let saved: Vec<Saved> = std::fs::read(&store)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let mut map = HashMap::new();
        let mut max = 0;
        for s in saved {
            max = max.max(s.id);
            let segs: Vec<_> = s
                .segs
                .iter()
                .map(|x| {
                    let done = x.done;
                    let full = x.end != u64::MAX && done >= x.end - x.start + 1;
                    Arc::new(Seg {
                        start: x.start,
                        end: x.end,
                        done: AtomicU64::new(done),
                        state: AtomicU8::new(if full { DONE } else { WAITING }),
                    })
                })
                .collect();
            let last = segs.iter().map(|g| g.done.load(Relaxed)).sum();
            let status = if s.completed {
                Status::Completed
            } else if let Some(e) = s.error {
                Status::Error(e)
            } else {
                Status::Paused
            };
            map.insert(
                s.id,
                Item {
                    url: s.url, path: s.path, total: s.total, resumable: s.resumable, status, segs,
                    cancel: Arc::new(AtomicBool::new(false)),
                    limit: Arc::new(AtomicU64::new(s.limit_kbps)),
                    speed: 0.0, last,
                },
            );
        }
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (RustDM/0.1)")
            .connect_timeout(Duration::from_secs(15))
            .build()
            .expect("http client");
        Manager {
            items: Arc::new(Mutex::new(map)),
            client,
            store,
            next_id: Arc::new(AtomicU64::new(max + 1)),
        }
    }

    pub fn save(&self) {
        let list: Vec<Saved> = self
            .items
            .lock()
            .unwrap()
            .iter()
            .map(|(id, it)| Saved {
                id: *id,
                url: it.url.clone(),
                path: it.path.clone(),
                total: it.total,
                resumable: it.resumable,
                completed: it.status == Status::Completed,
                error: if let Status::Error(e) = &it.status { Some(e.clone()) } else { None },
                limit_kbps: it.limit.load(Relaxed),
                segs: it.segs.iter().map(|s| SavedSeg { start: s.start, end: s.end, done: s.done.load(Relaxed) }).collect(),
            })
            .collect();
        if let Ok(j) = serde_json::to_vec(&list) {
            let _ = std::fs::write(&self.store, j);
        }
    }

    /// Snapshot for the UI; `dt` = seconds since previous call (for speed).
    pub fn views(&self, dt: f64) -> Vec<View> {
        let mut g = self.items.lock().unwrap();
        let mut v: Vec<View> = g
            .iter_mut()
            .map(|(id, it)| {
                let downloaded: u64 = it.segs.iter().map(|s| s.done.load(Relaxed)).sum();
                let inst = downloaded.saturating_sub(it.last) as f64 / dt.max(0.05);
                it.last = downloaded;
                it.speed = if it.status == Status::Downloading { it.speed * 0.6 + inst * 0.4 } else { 0.0 };
                let eta = match (it.total, it.speed > 1.0) {
                    (Some(t), true) => Some((t.saturating_sub(downloaded) as f64 / it.speed) as u64),
                    _ => None,
                };
                let (status, error) = match &it.status {
                    Status::Paused => ("paused", None),
                    Status::Downloading => ("downloading", None),
                    Status::Completed => ("completed", None),
                    Status::Error(e) => ("error", Some(e.clone())),
                };
                View {
                    id: *id,
                    name: it.path.file_name().map(|n| n.to_string_lossy().into()).unwrap_or_default(),
                    url: it.url.clone(),
                    path: it.path.to_string_lossy().into(),
                    total: it.total,
                    downloaded,
                    speed: it.speed as u64,
                    eta,
                    status,
                    error,
                    resumable: it.resumable,
                    limit_kbps: it.limit.load(Relaxed),
                    segments: it
                        .segs
                        .iter()
                        .map(|s| SegView {
                            done: s.done.load(Relaxed),
                            total: if s.end == u64::MAX { None } else { Some(s.len()) },
                            state: s.state.load(Relaxed),
                        })
                        .collect(),
                }
            })
            .collect();
        v.sort_by_key(|x| x.id);
        v
    }

    pub async fn add(&self, url: String, dir: PathBuf, conns: u32) -> Result<u64, String> {
        let parsed = url::Url::parse(url.trim()).map_err(|e| e.to_string())?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err("only http/https supported".into());
        }
        let (total, ranged, name) = self.probe(&parsed).await?;
        tokio::fs::create_dir_all(&dir).await.map_err(|e| e.to_string())?;
        let path = unique_path(&dir, &sanitize(&name));
        let segs: Vec<Arc<Seg>> = match (total, ranged) {
            (Some(t), true) if t > 0 => {
                let n = (conns.clamp(1, 32) as u64).min((t / (256 * 1024)).max(1));
                let size = t.div_ceil(n);
                (0..n)
                    .map(|i| {
                        let start = i * size;
                        let end = ((i + 1) * size - 1).min(t - 1);
                        Arc::new(Seg { start, end, done: AtomicU64::new(0), state: AtomicU8::new(WAITING) })
                    })
                    .collect()
            }
            _ => vec![Arc::new(Seg {
                start: 0,
                end: total.filter(|t| *t > 0).map(|t| t - 1).unwrap_or(u64::MAX),
                done: AtomicU64::new(0),
                state: AtomicU8::new(WAITING),
            })],
        };
        // pre-allocate
        let f = tokio::fs::File::create(&path).await.map_err(|e| e.to_string())?;
        if let Some(t) = total {
            f.set_len(t).await.map_err(|e| e.to_string())?;
        }
        drop(f);
        let id = self.next_id.fetch_add(1, Relaxed);
        self.items.lock().unwrap().insert(
            id,
            Item {
                url: parsed.to_string(), path, total, resumable: ranged, status: Status::Paused, segs,
                cancel: Arc::new(AtomicBool::new(false)),
                limit: Arc::new(AtomicU64::new(0)),
                speed: 0.0, last: 0,
            },
        );
        self.resume(id);
        Ok(id)
    }

    async fn probe(&self, u: &url::Url) -> Result<(Option<u64>, bool, String), String> {
        let r = self.client.get(u.as_str()).header("Range", "bytes=0-0").send().await.map_err(|e| e.to_string())?;
        let st = r.status();
        if !st.is_success() {
            return Err(format!("HTTP {st}"));
        }
        let h = r.headers();
        let (total, ranged) = if st.as_u16() == 206 {
            let t = h
                .get("content-range")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.rsplit('/').next())
                .and_then(|s| s.parse::<u64>().ok());
            (t, t.is_some())
        } else {
            (r.content_length(), false)
        };
        let name = h
            .get("content-disposition")
            .and_then(|v| v.to_str().ok())
            .and_then(disp_name)
            .or_else(|| {
                let last = r.url().path_segments()?.last()?.to_string();
                (!last.is_empty()).then(|| percent_decode_str(&last).decode_utf8_lossy().into_owned())
            })
            .unwrap_or_else(|| "download".into());
        Ok((total, ranged, name))
    }

    pub fn resume(&self, id: u64) {
        let ok = matches!(
            self.items.lock().unwrap().get(&id).map(|i| i.status.clone()),
            Some(Status::Paused) | Some(Status::Error(_))
        );
        if ok {
            let m = self.clone();
            tauri::async_runtime::spawn(async move { m.run(id).await });
        }
    }
    pub fn pause(&self, id: u64) {
        if let Some(it) = self.items.lock().unwrap().get(&id) {
            it.cancel.store(true, Relaxed);
        }
    }
    pub fn resume_all(&self) {
        let ids: Vec<u64> = self.items.lock().unwrap().keys().cloned().collect();
        ids.into_iter().for_each(|i| self.resume(i));
    }
    pub fn pause_all(&self) {
        self.items.lock().unwrap().values().for_each(|i| i.cancel.store(true, Relaxed));
    }
    pub fn set_limit(&self, id: u64, kbps: u64) {
        if let Some(it) = self.items.lock().unwrap().get(&id) {
            it.limit.store(kbps, Relaxed);
        }
    }
    pub fn path_of(&self, id: u64) -> Option<PathBuf> {
        self.items.lock().unwrap().get(&id).map(|i| i.path.clone())
    }
    pub fn remove(&self, id: u64, delete_file: bool) {
        let it = self.items.lock().unwrap().remove(&id);
        if let Some(it) = it {
            it.cancel.store(true, Relaxed);
            let incomplete = it.status != Status::Completed;
            if delete_file || incomplete {
                let p = it.path;
                tauri::async_runtime::spawn(async move {
                    sleep(Duration::from_millis(1500)).await; // let workers release the file (Windows)
                    let _ = tokio::fs::remove_file(p).await;
                });
            }
        }
        self.save();
    }

    async fn run(&self, id: u64) {
        let (url, path, segs, cancel, limit, ranged) = {
            let mut g = self.items.lock().unwrap();
            let Some(it) = g.get_mut(&id) else { return };
            it.status = Status::Downloading;
            it.cancel.store(false, Relaxed);
            (it.url.clone(), it.path.clone(), it.segs.clone(), it.cancel.clone(), it.limit.clone(), it.resumable)
        };
        let n = segs.len() as u64;
        let handles: Vec<_> = segs
            .iter()
            .filter(|s| s.state.load(Relaxed) != DONE)
            .map(|s| {
                tokio::spawn(run_seg(self.client.clone(), url.clone(), path.clone(), s.clone(), ranged, cancel.clone(), limit.clone(), n))
            })
            .collect();
        let mut err = None;
        for h in handles {
            match h.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => err = Some(e),
                Err(e) => err = Some(e.to_string()),
            }
        }
        if let Some(it) = self.items.lock().unwrap().get_mut(&id) {
            let all_done = it.segs.iter().all(|s| s.state.load(Relaxed) == DONE);
            it.status = if all_done {
                if it.total.is_none() {
                    it.total = Some(it.segs.iter().map(|s| s.done.load(Relaxed)).sum());
                }
                Status::Completed
            } else if let Some(e) = err {
                Status::Error(e)
            } else {
                Status::Paused
            };
        }
        self.save();
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_seg(
    client: reqwest::Client, url: String, path: PathBuf, seg: Arc<Seg>, ranged: bool,
    cancel: Arc<AtomicBool>, limit: Arc<AtomicU64>, nseg: u64,
) -> Result<(), String> {
    let mut tries = 0u64;
    loop {
        match attempt(&client, &url, &path, &seg, ranged, &cancel, &limit, nseg).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                seg.state.store(WAITING, Relaxed);
                if cancel.load(Relaxed) {
                    return Ok(());
                }
                tries += 1;
                if tries > 5 {
                    return Err(e);
                }
                sleep(Duration::from_secs(tries * 2)).await;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn attempt(
    client: &reqwest::Client, url: &str, path: &Path, seg: &Seg, ranged: bool,
    cancel: &AtomicBool, limit: &AtomicU64, nseg: u64,
) -> Result<(), String> {
    let from = seg.start + seg.done.load(Relaxed);
    if seg.end != u64::MAX && from > seg.end {
        seg.state.store(DONE, Relaxed);
        return Ok(());
    }
    seg.state.store(RECV, Relaxed);
    let mut req = client.get(url);
    if ranged {
        let to = if seg.end == u64::MAX { String::new() } else { seg.end.to_string() };
        req = req.header("Range", format!("bytes={from}-{to}"));
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    let st = resp.status();
    if ranged && st.as_u16() != 206 {
        return Err(format!("HTTP {st} (range not honoured)"));
    }
    if !st.is_success() {
        return Err(format!("HTTP {st}"));
    }
    let mut f = OpenOptions::new().write(true).open(path).await.map_err(|e| e.to_string())?;
    f.seek(SeekFrom::Start(from)).await.map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    let (t0, mut got, mut stall) = (Instant::now(), 0u64, 0u32);
    loop {
        if cancel.load(Relaxed) {
            let _ = f.flush().await;
            seg.state.store(WAITING, Relaxed);
            return Ok(());
        }
        match tokio::time::timeout(Duration::from_secs(1), stream.next()).await {
            Err(_) => {
                stall += 1;
                if stall >= 30 {
                    return Err("timeout".into());
                }
            }
            Ok(None) => break,
            Ok(Some(Err(e))) => return Err(e.to_string()),
            Ok(Some(Ok(mut c))) => {
                stall = 0;
                if seg.end != u64::MAX {
                    let left = seg.len() - seg.done.load(Relaxed);
                    if c.len() as u64 > left {
                        c.truncate(left as usize);
                    }
                }
                f.write_all(&c).await.map_err(|e| e.to_string())?;
                seg.done.fetch_add(c.len() as u64, Relaxed);
                got += c.len() as u64;
                let k = limit.load(Relaxed);
                if k > 0 {
                    let rate = (k * 1024 / nseg).max(1) as f64;
                    let ahead = got as f64 / rate - t0.elapsed().as_secs_f64();
                    if ahead > 0.0 {
                        sleep(Duration::from_secs_f64(ahead.min(1.0))).await;
                    }
                }
            }
        }
    }
    f.flush().await.map_err(|e| e.to_string())?;
    if seg.end != u64::MAX && seg.done.load(Relaxed) < seg.len() {
        return Err("connection closed early".into());
    }
    seg.state.store(DONE, Relaxed);
    Ok(())
}

fn disp_name(h: &str) -> Option<String> {
    let low = h.to_ascii_lowercase();
    if let Some(i) = low.find("filename*=") {
        let v = h[i + 10..].split(';').next()?.trim();
        let v = v.splitn(3, '\'').last()?;
        return Some(percent_decode_str(v).decode_utf8_lossy().into_owned());
    }
    let i = low.find("filename=")?;
    let v = h[i + 9..].split(';').next()?.trim().trim_matches('"');
    (!v.is_empty()).then(|| v.to_string())
}

fn sanitize(n: &str) -> String {
    let s: String = n.chars().map(|c| if "/\\:*?\"<>|".contains(c) || c.is_control() { '_' } else { c }).collect();
    let s = s.trim().trim_matches('.').to_string();
    if s.is_empty() { "download".into() } else { s }
}

fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    if !p.exists() {
        return p;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (1..)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|p| !p.exists())
        .unwrap()
}
