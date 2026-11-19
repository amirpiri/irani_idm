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
const PROBE_TIMEOUT: Duration = Duration::from_secs(15);

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
    Preparing,
    Paused,
    Downloading,
    Completed,
    Error(String),
}

struct Item {
    url: String,
    server_ip: Option<String>,
    server_country: Option<String>,
    server_country_code: Option<String>,
    location_lookup_pending: bool,
    path: PathBuf,
    total: Option<u64>,
    resumable: bool,
    file_created: bool,
    status: Status,
    proxy_used: bool,
    restart_after_cancel: bool,
    connections: u32,
    metadata_pending: bool,
    removing: bool,
    segs: Vec<Arc<Seg>>,
    cancel: Arc<AtomicBool>,
    limit: Arc<AtomicU64>, // KB/s, 0 = unlimited
    speed: f64,
    last: u64,
}

struct RateState {
    tokens: f64,
    rate: f64,
    updated_at: Instant,
}

struct RateLimiter {
    state: tokio::sync::Mutex<RateState>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            state: tokio::sync::Mutex::new(RateState {
                tokens: 0.0,
                rate: 0.0,
                updated_at: Instant::now(),
            }),
        }
    }

    async fn acquire(&self, bytes: usize, limit: &AtomicU64, cancel: &AtomicBool) -> bool {
        let mut remaining = bytes as f64;
        while remaining > 0.0 {
            if cancel.load(Relaxed) {
                return false;
            }
            let kbps = limit.load(Relaxed);
            if kbps == 0 {
                return true;
            }
            let rate = kbps as f64 * 1024.0;
            let mut state = self.state.lock().await;
            let now = Instant::now();
            let elapsed = now.duration_since(state.updated_at).as_secs_f64();
            state.tokens = (state.tokens + elapsed * state.rate).min(state.rate);
            state.updated_at = now;
            if rate != state.rate {
                state.tokens = state.tokens.min(rate);
                state.rate = rate;
            }

            let needed = remaining.min(rate);
            if state.tokens >= needed {
                state.tokens -= needed;
                remaining -= needed;
            } else {
                let wait = (needed - state.tokens) / rate;
                drop(state);
                sleep(Duration::from_secs_f64(wait.min(0.1))).await;
            }
        }
        true
    }
}

#[derive(Serialize, Deserialize)]
struct SavedSeg { start: u64, end: u64, done: u64 }
#[derive(Serialize, Deserialize)]
struct Saved {
    id: u64,
    url: String,
    #[serde(default)]
    server_ip: Option<String>,
    #[serde(default)]
    server_country: Option<String>,
    #[serde(default)]
    server_country_code: Option<String>,
    path: PathBuf,
    total: Option<u64>,
    resumable: bool,
    #[serde(default = "default_file_created")]
    file_created: bool,
    #[serde(default = "default_connections")]
    connections: u32,
    #[serde(default)]
    metadata_pending: bool,
    completed: bool,
    #[serde(default)]
    proxy_used: bool,
    error: Option<String>,
    limit_kbps: u64,
    segs: Vec<SavedSeg>,
}

fn default_connections() -> u32 {
    8
}

fn default_file_created() -> bool {
    true
}

#[derive(Serialize, Clone)]
pub struct SegView { done: u64, total: Option<u64>, state: u8 }
#[derive(Serialize, Clone)]
pub struct View {
    pub id: u64,
    pub name: String,
    url: String,
    server_country: Option<String>,
    server_country_code: Option<String>,
    path: String,
    total: Option<u64>,
    downloaded: u64,
    speed: u64,
    eta: Option<u64>,
    pub status: &'static str,
    proxy_used: bool,
    error: Option<String>,
    resumable: bool,
    limit_kbps: u64,
    segments: Vec<SegView>,
}

#[derive(Clone)]
pub struct Manager {
    items: Arc<Mutex<HashMap<u64, Item>>>,
    client: Arc<Mutex<reqwest::Client>>,
    store: PathBuf,
    proxy_store: PathBuf,
    proxy: Arc<Mutex<ProxySettings>>,
    next_id: Arc<AtomicU64>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProxySettings {
    pub enabled: bool,
    pub protocol: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

impl Default for ProxySettings {
    fn default() -> Self {
        Self { enabled: false, protocol: "http".into(), host: String::new(), port: 8080, username: String::new(), password: String::new() }
    }
}

fn build_client(settings: &ProxySettings) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (RustDM/0.1)")
        .connect_timeout(Duration::from_secs(15));
    if settings.enabled {
        let scheme = match settings.protocol.as_str() {
            "http" => "http",
            // Resolve destination hostnames through the SOCKS server. This is
            // important on networks where local DNS cannot resolve them.
            "socks5" => "socks5h",
            _ => return Err("Unsupported proxy protocol".into()),
        };
        if settings.host.trim().is_empty() || settings.port == 0 {
            return Err("Enter a valid proxy host and port".into());
        }
        let host = settings.host.trim();
        let host = if host.contains(':') && !host.starts_with('[') {
            format!("[{host}]")
        } else {
            host.to_string()
        };
        let mut url = url::Url::parse(&format!("{scheme}://{host}:{}", settings.port))
            .map_err(|_| "Invalid proxy address".to_string())?;
        if !settings.username.is_empty() {
            url.set_username(&settings.username).map_err(|_| "Invalid proxy username".to_string())?;
            url.set_password((!settings.password.is_empty()).then_some(settings.password.as_str()))
                .map_err(|_| "Invalid proxy password".to_string())?;
        }
        builder = builder.proxy(reqwest::Proxy::all(url.as_str()).map_err(|error| error.to_string())?);
    }
    builder.build().map_err(|error| error.to_string())
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
        for mut s in saved {
            max = max.max(s.id);
            // Older versions wrote directly to the final filename. Move
            // unfinished files to the visible temporary-name convention.
            if !s.completed && s.file_created {
                if let Some(temporary) = temporary_download_path(&s.path) {
                    if s.path.exists() && !temporary.exists()
                        && std::fs::rename(&s.path, &temporary).is_ok()
                    {
                        s.path = temporary;
                    }
                }
            }
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
                    url: s.url, path: s.path, total: s.total, resumable: s.resumable, status,
                    server_ip: s.server_ip,
                    server_country: s.server_country,
                    server_country_code: s.server_country_code,
                    location_lookup_pending: false,
                    proxy_used: s.proxy_used,
                    restart_after_cancel: false,
                    file_created: s.file_created,
                    connections: s.connections.clamp(1, 32), metadata_pending: s.metadata_pending, segs,
                    removing: false,
                    cancel: Arc::new(AtomicBool::new(false)),
                    limit: Arc::new(AtomicU64::new(s.limit_kbps)),
                    speed: 0.0, last,
                },
            );
        }
        let proxy_store = data_dir.join("proxy.json");
        let proxy: ProxySettings = std::fs::read(&proxy_store).ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
        let client = build_client(&proxy).expect("valid saved proxy settings");
        Manager {
            items: Arc::new(Mutex::new(map)),
            client: Arc::new(Mutex::new(client)),
            store,
            proxy_store,
            proxy: Arc::new(Mutex::new(proxy)),
            next_id: Arc::new(AtomicU64::new(max + 1)),
        }
    }

    pub fn proxy_settings(&self) -> ProxySettings {
        self.proxy.lock().unwrap().clone()
    }

    pub fn set_proxy_settings(&self, settings: ProxySettings) -> Result<(), String> {
        let client = build_client(&settings)?;
        let json = serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?;
        std::fs::write(&self.proxy_store, json).map_err(|error| error.to_string())?;
        *self.client.lock().unwrap() = client;
        let changed = {
            let mut current = self.proxy.lock().unwrap();
            let changed = *current != settings;
            *current = settings;
            changed
        };
        // Existing HTTP clients keep their original proxy configuration.
        // Stop active transfers so they can resume with the newly built client.
        if changed {
            for item in self.items.lock().unwrap().values_mut() {
                if item.status == Status::Downloading && !item.removing {
                    item.restart_after_cancel = true;
                    item.cancel.store(true, Relaxed);
                }
            }
        }
        Ok(())
    }

    pub async fn test_proxy_settings(settings: ProxySettings) -> Result<String, String> {
        // Testing should work before the user enables the proxy globally. The
        // test always routes this one request through the supplied endpoint.
        let mut test_settings = settings;
        test_settings.enabled = true;
        let client = build_client(&test_settings)?;
        let response = client
            .get("https://api.ipify.org")
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .map_err(|error| format!("Proxy connection failed: {error}"))?
            .error_for_status()
            .map_err(|error| format!("Proxy request failed: {error}"))?;
        let ip = response
            .text()
            .await
            .map_err(|error| format!("Could not read proxy test response: {error}"))?;
        let ip = ip.trim();
        if ip.is_empty() {
            return Err("The proxy test returned an empty IP address".into());
        }
        Ok(format!("Proxy connection successful. Exit IP: {ip}"))
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
                server_ip: it.server_ip.clone(),
                server_country: it.server_country.clone(),
                server_country_code: it.server_country_code.clone(),
                path: it.path.clone(),
                total: it.total,
                resumable: it.resumable,
                file_created: it.file_created,
                connections: it.connections,
                metadata_pending: it.metadata_pending,
                completed: it.status == Status::Completed,
                proxy_used: it.proxy_used,
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
        let proxy_enabled = self.proxy.lock().unwrap().enabled;
        let mut v: Vec<View> = g
            .iter_mut()
            .map(|(id, it)| {
                let downloaded: u64 = it.segs.iter().map(|s| s.done.load(Relaxed)).sum();
                let inst = downloaded.saturating_sub(it.last) as f64 / dt.max(0.05);
                it.last = downloaded;
                it.speed = if it.status == Status::Downloading { it.speed * 0.6 + inst * 0.4 } else { 0.0 };
                Self::view_of(*id, it, proxy_enabled)
            })
            .collect();
        v.sort_by_key(|x| x.id);
        v
    }

    pub fn view(&self, id: u64) -> Option<View> {
        let items = self.items.lock().unwrap();
        let proxy_enabled = self.proxy.lock().unwrap().enabled;
        items.get(&id).map(|it| Self::view_of(id, it, proxy_enabled))
    }

    fn view_of(id: u64, it: &Item, proxy_enabled: bool) -> View {
        let downloaded: u64 = it.segs.iter().map(|s| s.done.load(Relaxed)).sum();
        let eta = match (it.total, it.speed > 1.0) {
            (Some(t), true) => Some((t.saturating_sub(downloaded) as f64 / it.speed) as u64),
            _ => None,
        };
        let (status, error) = match &it.status {
            Status::Preparing => ("preparing", None),
            Status::Paused => ("paused", None),
            Status::Downloading => ("downloading", None),
            Status::Completed => ("completed", None),
            Status::Error(e) => ("error", Some(e.clone())),
        };
        View {
            id,
            name: it.path.file_name().map(|n| n.to_string_lossy().into()).unwrap_or_default(),
            url: it.url.clone(),
            server_country: it.server_country.clone(),
            server_country_code: it.server_country_code.clone(),
            path: it.path.to_string_lossy().into(),
            total: it.total,
            downloaded,
            speed: it.speed as u64,
            eta,
            status,
            proxy_used: it.proxy_used && proxy_enabled && it.status == Status::Downloading,
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
    }

    pub fn add(&self, url: String, dir: PathBuf, conns: u32) -> Result<u64, String> {
        let parsed = url::Url::parse(url.trim()).map_err(|e| e.to_string())?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err("only http/https supported".into());
        }
        let url = parsed.to_string();
        let name = url_name(&parsed);
        let connections = conns.clamp(1, 32);
        let id = self.next_id.fetch_add(1, Relaxed);
        {
            let mut items = self.items.lock().unwrap();
            if items.values().any(|item| item.url == url && item.status == Status::Preparing) {
                return Err("This URL is already being prepared".into());
            }
            items.insert(
                id,
                Item {
                    url,
                    server_ip: None,
                    server_country: None,
                    server_country_code: None,
                    location_lookup_pending: false,
                    path: dir.join(name),
                    total: None,
                    resumable: false,
                    file_created: false,
                    status: Status::Preparing,
                    proxy_used: false,
                    restart_after_cancel: false,
                    connections,
                    metadata_pending: false,
                    removing: false,
                    segs: Vec::new(),
                    cancel: Arc::new(AtomicBool::new(false)),
                    limit: Arc::new(AtomicU64::new(0)),
                    speed: 0.0,
                    last: 0,
                },
            );
        }
        self.save();
        let manager = self.clone();
        tauri::async_runtime::spawn(async move { manager.prepare(id).await });
        Ok(id)
    }

    async fn prepare(&self, id: u64) {
        let (url, dir, connections, cancel) = {
            let items = self.items.lock().unwrap();
            let Some(item) = items.get(&id) else { return };
            if item.removing || item.status != Status::Preparing {
                return;
            }
            (
                item.url.clone(),
                item.path.parent().unwrap_or(Path::new(".")).to_path_buf(),
                item.connections,
                item.cancel.clone(),
            )
        };
        if let Err(error) = tokio::fs::create_dir_all(&dir).await {
            self.fail_preparation(id, error.to_string());
            return;
        }
        let parsed = match url::Url::parse(&url) {
            Ok(url) => url,
            Err(error) => {
                self.fail_preparation(id, error.to_string());
                return;
            }
        };
        let result = tokio::select! {
            _ = wait_for_cancel(&cancel) => {
                self.finish_cancelled_preparation(id);
                return;
            },
            result = tokio::time::timeout(PROBE_TIMEOUT, self.probe(&parsed)) => result,
        };
        match result {
            Ok(Ok((total, ranged, name, ip))) => {
                self.record_server(id, ip);
                self.finish_preparation(id, total, ranged, name, connections).await;
            }
            Ok(Err(error)) => self.fail_preparation(id, error),
            Err(_) => self.start_single_connection_fallback(id).await,
        }
    }

    async fn finish_preparation(
        &self,
        id: u64,
        total: Option<u64>,
        ranged: bool,
        name: String,
        connections: u32,
    ) {
        let dir = {
            let items = self.items.lock().unwrap();
            let Some(item) = items.get(&id) else { return };
            if item.removing || item.cancel.load(Relaxed) {
                None
            } else {
                Some(item.path.parent().unwrap_or(Path::new(".")).to_path_buf())
            }
        };
        let Some(dir) = dir else {
            self.finish_cancelled_preparation(id);
            return;
        };
        let path = match create_download_file(&dir, &sanitize(&name), total) {
            Ok(path) => path,
            Err(error) => {
                self.fail_preparation(id, error);
                return;
            }
        };
        let segs = make_segments(total, ranged, connections);
        let mut cancelled = false;
        {
            let mut items = self.items.lock().unwrap();
            let Some(item) = items.get_mut(&id) else {
                let _ = std::fs::remove_file(path);
                return;
            };
            if item.removing || item.cancel.load(Relaxed) {
                let _ = std::fs::remove_file(path);
                cancelled = true;
                if !item.removing {
                    item.status = Status::Paused;
                }
            } else {
                item.path = path;
                item.total = total;
                item.resumable = ranged;
                item.file_created = true;
                item.metadata_pending = false;
                item.segs = segs;
                item.status = Status::Paused;
            }
        }
        if cancelled {
            self.save();
            return;
        }
        self.save();
        self.resume(id);
    }

    async fn start_single_connection_fallback(&self, id: u64) {
        let (dir, url, connections) = {
            let items = self.items.lock().unwrap();
            let Some(item) = items.get(&id) else { return };
            if item.removing || item.cancel.load(Relaxed) {
                (None, String::new(), 0)
            } else {
                (
                    Some(item.path.parent().unwrap_or(Path::new(".")).to_path_buf()),
                    item.url.clone(),
                    item.connections,
                )
            }
        };
        let Some(dir) = dir else {
            self.finish_cancelled_preparation(id);
            return;
        };
        let name = match url::Url::parse(&url) {
            Ok(url) => url_name(&url),
            Err(error) => {
                self.fail_preparation(id, error.to_string());
                return;
            }
        };
        let path = match tokio::fs::create_dir_all(&dir).await {
            Ok(()) => dir.join(name),
            Err(error) => {
                self.fail_preparation(id, error.to_string());
                return;
            }
        };
        let mut cancelled = false;
        {
            let mut items = self.items.lock().unwrap();
            let Some(item) = items.get_mut(&id) else { return };
            if item.removing || item.cancel.load(Relaxed) {
                cancelled = true;
                if !item.removing {
                    item.status = Status::Paused;
                }
            } else {
                item.path = path;
                item.total = None;
                item.resumable = false;
                item.metadata_pending = true;
                item.segs = make_segments(None, false, connections);
                item.status = Status::Paused;
            }
        }
        if cancelled {
            self.save();
            return;
        }
        self.save();
        self.resume(id);
    }

    fn fail_preparation(&self, id: u64, error: String) {
        if let Some(item) = self.items.lock().unwrap().get_mut(&id) {
            if !item.removing {
                item.status = Status::Error(error);
            }
        }
        self.save();
    }

    fn finish_cancelled_preparation(&self, id: u64) {
        if let Some(item) = self.items.lock().unwrap().get_mut(&id) {
            if !item.removing {
                item.status = Status::Paused;
            }
        }
        self.save();
    }

    async fn probe(&self, u: &url::Url) -> Result<(Option<u64>, bool, String, Option<std::net::IpAddr>), String> {
        let client = self.client.lock().unwrap().clone();
        let r = client.get(u.as_str()).header("Range", "bytes=0-0").send().await.map_err(|e| e.to_string())?;
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
        let name = response_name(r.headers(), r.url());
        // With a configured HTTP/SOCKS proxy, `remote_addr()` may be absent
        // or refer to the proxy rather than the download host. Resolve the
        // response URL host as a fallback so we can still identify its country.
        let prefer_peer = !self.proxy.lock().unwrap().enabled;
        let ip = resolve_server_ip(r.url(), r.remote_addr().map(|address| address.ip()), prefer_peer).await;
        Ok((total, ranged, name, ip))
    }

    fn record_server(&self, id: u64, ip: Option<std::net::IpAddr>) {
        let ip = ip.map(|address| address.to_string());
        let should_lookup = {
            let mut items = self.items.lock().unwrap();
            let Some(item) = items.get_mut(&id) else { return };
            let should_lookup = if let Some(ip) = ip.as_ref() {
                let same_ip = item.server_ip.as_ref() == Some(ip);
                // Retry lookups when the IP was persisted by an earlier run
                // but its country lookup never completed successfully.
                if same_ip
                    && item.server_country.is_some()
                    && item.server_country_code.is_some()
                {
                    false
                } else if same_ip && item.location_lookup_pending {
                    false
                } else {
                    item.server_ip = Some(ip.clone());
                    if !same_ip {
                        item.server_country = None;
                        item.server_country_code = None;
                    }
                    item.location_lookup_pending = true;
                    true
                }
            } else {
                false
            };
            should_lookup
        };
        if should_lookup {
            self.save();
            let manager = self.clone();
            let ip = ip.unwrap();
            tauri::async_runtime::spawn(async move { manager.lookup_server_location(id, ip).await; });
        }
    }

    async fn lookup_server_location(&self, id: u64, ip: String) {
        #[derive(Deserialize)]
        struct IpLocation {
            success: bool,
            country: Option<String>,
            country_code: Option<String>,
        }
        // Use the same client as downloads so geolocation also works when
        // direct internet access is unavailable and the app is using a proxy.
        let client = self.client.lock().unwrap().clone();
        let result = client
            .get(format!("https://ipwho.is/{ip}"))
            .timeout(Duration::from_secs(5))
            .send().await
            .and_then(reqwest::Response::error_for_status);
        let response = match result {
            Ok(response) => response,
            Err(error) => {
                eprintln!("IP geolocation request failed for {ip}: {error}");
                self.finish_location_lookup(id, &ip, None, None);
                return;
            }
        };
        let location = match response.json::<IpLocation>().await {
            Ok(location) => location,
            Err(error) => {
                eprintln!("Could not parse IP geolocation response for {ip}: {error}");
                self.finish_location_lookup(id, &ip, None, None);
                return;
            }
        };
        if !location.success {
            eprintln!("IP geolocation service returned no location for {ip}");
            self.finish_location_lookup(id, &ip, None, None);
            return;
        }
        self.finish_location_lookup(id, &ip, location.country, location.country_code);
    }

    fn finish_location_lookup(
        &self,
        id: u64,
        ip: &str,
        country: Option<String>,
        country_code: Option<String>,
    ) {
        let mut items = self.items.lock().unwrap();
        let Some(item) = items.get_mut(&id) else { return };
        if item.server_ip.as_deref() == Some(ip) {
            item.location_lookup_pending = false;
            item.server_country = country;
            item.server_country_code = country_code;
            drop(items);
            self.save();
        }
    }

    async fn finalize_fallback(
        &self,
        id: u64,
        response: &reqwest::Response,
    ) -> Result<PathBuf, String> {
        let (old_path, pending) = {
            let items = self.items.lock().unwrap();
            let item = items.get(&id).ok_or("Download not found")?;
            (item.path.clone(), item.metadata_pending)
        };
        if !pending {
            return Ok(old_path);
        }
        let dir = old_path.parent().unwrap_or(Path::new("."));
        let name = sanitize(&response_name(response.headers(), response.url()));
        let path = create_download_file(dir, &name, response.content_length())?;
        let mut items = self.items.lock().unwrap();
        let Some(item) = items.get_mut(&id) else {
            let _ = std::fs::remove_file(&path);
            return Err("Download not found".into());
        };
        if item.removing {
            let _ = std::fs::remove_file(&path);
            return Err("Download is being removed".into());
        }
        item.path = path.clone();
        item.total = response.content_length();
        item.file_created = true;
        item.metadata_pending = false;
        drop(items);
        self.save();
        Ok(path)
    }

    pub fn resume(&self, id: u64) {
        let prepare = {
            let mut items = self.items.lock().unwrap();
            let Some(item) = items.get_mut(&id) else { return };
            if item.removing || !matches!(item.status, Status::Paused | Status::Error(_)) {
                return;
            }
            item.cancel.store(false, Relaxed);
            if item.segs.is_empty() {
                item.status = Status::Preparing;
                true
            } else {
                false
            }
        };
        let manager = self.clone();
        if prepare {
            self.save();
            tauri::async_runtime::spawn(async move { manager.prepare(id).await });
        } else {
            tauri::async_runtime::spawn(async move { manager.run(id).await });
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
    fn is_metadata_pending(&self, id: u64) -> bool {
        self.items
            .lock()
            .unwrap()
            .get(&id)
            .is_some_and(|item| item.metadata_pending)
    }
    pub async fn remove(&self, id: u64, delete_file: bool) -> Result<(), String> {
        {
            let mut items = self.items.lock().unwrap();
            let item = items.get_mut(&id).ok_or("Download not found")?;
            if item.removing {
                return Err("Download is already being removed".into());
            }
            item.removing = true;
            item.cancel.store(true, Relaxed);
        }

        loop {
            let downloading = self
                .items
                .lock()
                .unwrap()
                .get(&id)
                .is_some_and(|item| item.status == Status::Downloading);
            if !downloading {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }

        if delete_file {
            let (path, file_created) = {
                let items = self.items.lock().unwrap();
                let item = items.get(&id).ok_or("Download not found")?;
                (item.path.clone(), item.file_created)
            };
            if file_created {
                let mut last_error = None;
                for attempt in 0..10 {
                    match tokio::fs::remove_file(&path).await {
                        Ok(()) => {
                            last_error = None;
                            break;
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                            last_error = None;
                            break;
                        }
                        Err(error) => {
                            last_error = Some(error);
                            if attempt < 9 {
                                sleep(Duration::from_millis(100)).await;
                            }
                        }
                    }
                }
                if let Some(error) = last_error {
                    if let Some(item) = self.items.lock().unwrap().get_mut(&id) {
                        item.removing = false;
                    }
                    return Err(format!("Could not delete downloaded file: {error}"));
                }
            }
        }

        self.items.lock().unwrap().remove(&id).ok_or("Download not found")?;
        self.save();
        Ok(())
    }

    async fn run(&self, id: u64) {
        let (url, path, segs, cancel, limit, ranged) = {
            let mut g = self.items.lock().unwrap();
            let Some(it) = g.get_mut(&id) else { return };
            if it.removing {
                return;
            }
            it.status = Status::Downloading;
            it.cancel.store(false, Relaxed);
            it.proxy_used = self.proxy.lock().unwrap().enabled;
            (it.url.clone(), it.path.clone(), it.segs.clone(), it.cancel.clone(), it.limit.clone(), it.resumable)
        };
        let client = self.client.lock().unwrap().clone();
        let pacer = Arc::new(RateLimiter::new());
        let handles: Vec<_> = segs
            .iter()
            .filter(|s| s.state.load(Relaxed) != DONE)
            .map(|s| {
                tokio::spawn(run_seg(
                    client.clone(),
                    url.clone(),
                    path.clone(),
                    s.clone(),
                    ranged,
                    self.clone(),
                    id,
                    cancel.clone(),
                    limit.clone(),
                    pacer.clone(),
                ))
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
        let completed_path = {
            let items = self.items.lock().unwrap();
            items.get(&id).and_then(|item| {
                (!item.removing && item.segs.iter().all(|segment| segment.state.load(Relaxed) == DONE))
                    .then(|| item.path.clone())
            })
        };
        let completion_error = completed_path.and_then(|path| {
            match complete_download_file(&path) {
                Ok(final_path) => {
                    if let Some(item) = self.items.lock().unwrap().get_mut(&id) {
                        item.path = final_path;
                    }
                    None
                }
                Err(error) => Some(error),
            }
        });
        let restart = if let Some(it) = self.items.lock().unwrap().get_mut(&id) {
            let all_done = it.segs.iter().all(|s| s.state.load(Relaxed) == DONE);
            let restart = it.restart_after_cancel && !it.removing && !all_done;
            it.restart_after_cancel = false;
            it.status = if all_done && completion_error.is_none() {
                if it.total.is_none() {
                    it.total = Some(it.segs.iter().map(|s| s.done.load(Relaxed)).sum());
                }
                Status::Completed
            } else if all_done {
                Status::Error(completion_error.unwrap_or_else(|| "Could not finalize download".into()))
            } else if restart {
                Status::Paused
            } else if let Some(e) = err {
                Status::Error(e)
            } else {
                Status::Paused
            };
            restart
        } else { false };
        self.save();
        if restart {
            self.resume(id);
        }
    }
}

fn make_segments(total: Option<u64>, ranged: bool, connections: u32) -> Vec<Arc<Seg>> {
    match (total, ranged) {
        (Some(total), true) if total > 0 => {
            let count = (connections.clamp(1, 32) as u64).min((total / (256 * 1024)).max(1));
            let size = total.div_ceil(count);
            (0..count)
                .map(|index| {
                    let start = index * size;
                    let end = ((index + 1) * size - 1).min(total - 1);
                    Arc::new(Seg {
                        start,
                        end,
                        done: AtomicU64::new(0),
                        state: AtomicU8::new(WAITING),
                    })
                })
                .collect()
        }
        _ => vec![Arc::new(Seg {
            start: 0,
            end: if ranged {
                total.filter(|size| *size > 0).map(|size| size - 1).unwrap_or(u64::MAX)
            } else {
                u64::MAX
            },
            done: AtomicU64::new(0),
            state: AtomicU8::new(WAITING),
        })],
    }
}

#[cfg(test)]
mod segment_tests {
    use super::*;

    #[test]
    fn proxy_client_supports_http_and_socks5() {
        for protocol in ["http", "socks5"] {
            let settings = ProxySettings {
                enabled: true,
                protocol: protocol.into(),
                host: "127.0.0.1".into(),
                port: 1080,
                username: "user name".into(),
                password: "p@ss".into(),
            };
            assert!(build_client(&settings).is_ok(), "{protocol} proxy should build");
        }
    }

    #[test]
    fn invalid_proxy_settings_are_rejected() {
        let settings = ProxySettings {
            enabled: true,
            protocol: "ftp".into(),
            ..ProxySettings::default()
        };
        assert!(build_client(&settings).is_err());
    }

    #[test]
    fn ranged_download_uses_requested_connections_for_large_files() {
        let total = 8 * 1024 * 1024;
        let segments = make_segments(Some(total), true, 8);

        assert_eq!(segments.len(), 8);
        assert_eq!(segments.iter().map(|segment| segment.len()).sum::<u64>(), total);
    }

    #[test]
    fn non_ranged_download_uses_one_connection() {
        assert_eq!(make_segments(Some(8 * 1024 * 1024), false, 8).len(), 1);
    }
}

fn create_download_file(dir: &Path, name: &str, total: Option<u64>) -> Result<PathBuf, String> {
    let (stem, extension) = match name.rfind('.') {
        Some(index) if index > 0 => (&name[..index], &name[index..]),
        _ => (name, ""),
    };
    for suffix in 0u64.. {
        let filename = if suffix == 0 {
            name.to_string()
        } else {
            format!("{stem} ({suffix}){extension}")
        };
        let final_path = dir.join(filename);
        let path = append_download_suffix(&final_path).ok_or("Invalid download filename")?;
        if final_path.exists() {
            continue;
        }
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                if let Some(total) = total {
                    if let Err(error) = file.set_len(total) {
                        drop(file);
                        let _ = std::fs::remove_file(&path);
                        return Err(error.to_string());
                    }
                }
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("Could not choose a unique file name".into())
}

fn temporary_download_path(final_path: &Path) -> Option<PathBuf> {
    let name = final_path.file_name()?.to_string_lossy();
    if name.ends_with(".downloading") {
        return Some(final_path.to_path_buf());
    }
    append_download_suffix(final_path)
}

fn append_download_suffix(final_path: &Path) -> Option<PathBuf> {
    let name = final_path.file_name()?.to_string_lossy();
    Some(final_path.with_file_name(format!("{name}.downloading")))
}

fn complete_download_file(temporary_path: &Path) -> Result<PathBuf, String> {
    let name = temporary_path.file_name().ok_or("Invalid temporary download filename")?;
    let name = name.to_string_lossy();
    let final_name = name.strip_suffix(".downloading")
        .ok_or("Temporary download filename is missing its .downloading suffix")?;
    let final_path = temporary_path.with_file_name(final_name);
    if final_path.exists() {
        return Err(format!("Cannot finalize download because '{}' already exists", final_path.display()));
    }
    std::fs::rename(temporary_path, &final_path).map_err(|error| error.to_string())?;
    Ok(final_path)
}

fn url_name(url: &url::Url) -> String {
    url.path_segments()
        .and_then(Iterator::last)
        .filter(|name| !name.is_empty())
        .map(|name| percent_decode_str(name).decode_utf8_lossy().into_owned())
        .map(|name| sanitize(&name))
        .unwrap_or_else(|| "download".into())
}

fn response_name(headers: &reqwest::header::HeaderMap, url: &url::Url) -> String {
    headers
        .get("content-disposition")
        .and_then(|value| value.to_str().ok())
        .and_then(disp_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| url_name(url))
}

async fn wait_for_cancel(cancel: &AtomicBool) {
    while !cancel.load(Relaxed) {
        sleep(Duration::from_millis(50)).await;
    }
}

async fn resolve_server_ip(
    url: &url::Url,
    peer_ip: Option<std::net::IpAddr>,
    prefer_peer: bool,
) -> Option<std::net::IpAddr> {
    if prefer_peer {
        if let Some(ip) = peer_ip {
            return Some(ip);
        }
    }
    let host = url.host_str()?;
    if let Ok(ip) = host.parse() {
        return Some(ip);
    }
    let port = url.port_or_known_default()?;
    match tokio::time::timeout(Duration::from_secs(3), tokio::net::lookup_host((host, port))).await {
        Ok(Ok(addresses)) => addresses.into_iter().next().map(|address| address.ip()),
        Ok(Err(error)) => {
            eprintln!("Could not resolve download host {host}: {error}");
            None
        }
        Err(_) => {
            eprintln!("Timed out resolving download host {host}");
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_seg(
    client: reqwest::Client, url: String, path: PathBuf, seg: Arc<Seg>, ranged: bool,
    manager: Manager, id: u64,
    cancel: Arc<AtomicBool>, limit: Arc<AtomicU64>, pacer: Arc<RateLimiter>,
) -> Result<(), String> {
    let mut tries = 0u64;
    loop {
        let path = manager.path_of(id).unwrap_or_else(|| path.clone());
        match attempt(&client, &url, &path, &seg, ranged, &manager, id, &cancel, &limit, &pacer).await {
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
                tokio::select! {
                    _ = sleep(Duration::from_secs(tries * 2)) => {}
                    _ = async {
                        while !cancel.load(Relaxed) {
                            sleep(Duration::from_millis(50)).await;
                        }
                    } => return Ok(()),
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn attempt(
    client: &reqwest::Client, url: &str, path: &Path, seg: &Seg, ranged: bool,
    manager: &Manager, id: u64,
    cancel: &AtomicBool, limit: &AtomicU64, pacer: &RateLimiter,
) -> Result<(), String> {
    let mut from = seg.start + seg.done.load(Relaxed);
    if seg.end != u64::MAX && from > seg.end {
        seg.state.store(DONE, Relaxed);
        return Ok(());
    }
    if !ranged && from > 0 {
        seg.done.store(0, Relaxed);
        from = 0;
        if !manager.is_metadata_pending(id) {
            tokio::fs::File::create(path).await.map_err(|e| e.to_string())?;
        }
    }
    seg.state.store(RECV, Relaxed);
    let mut req = client.get(url);
    if ranged {
        let to = if seg.end == u64::MAX { String::new() } else { seg.end.to_string() };
        req = req.header("Range", format!("bytes={from}-{to}"));
    }
    let resp = tokio::select! {
        _ = wait_for_cancel(cancel) => {
            seg.state.store(WAITING, Relaxed);
            return Ok(());
        }
        result = req.send() => result.map_err(|e| e.to_string())?,
    };
    let prefer_peer = !manager.proxy.lock().unwrap().enabled;
    let server_ip = resolve_server_ip(resp.url(), resp.remote_addr().map(|address| address.ip()), prefer_peer).await;
    manager.record_server(id, server_ip);
    let st = resp.status();
    if ranged && st.as_u16() != 206 {
        return Err(format!("HTTP {st} (range not honoured)"));
    }
    if !st.is_success() {
        return Err(format!("HTTP {st}"));
    }
    let path = manager.finalize_fallback(id, &resp).await?;
    let mut f = OpenOptions::new().write(true).open(path).await.map_err(|e| e.to_string())?;
    f.seek(SeekFrom::Start(from)).await.map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    let mut stall = 0u32;
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
                if !pacer.acquire(c.len(), limit, cancel).await {
                    let _ = f.flush().await;
                    seg.state.store(WAITING, Relaxed);
                    return Ok(());
                }
                f.write_all(&c).await.map_err(|e| e.to_string())?;
                seg.done.fetch_add(c.len() as u64, Relaxed);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn manager_with_download() -> (Manager, PathBuf, u64) {
        let data_dir = std::env::temp_dir().join(format!(
            "irani-idm-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&data_dir).unwrap();
        let manager = Manager::load(&data_dir);
        let id = 1;
        let path = data_dir.join("download.bin");
        std::fs::write(&path, b"download").unwrap();
        manager.items.lock().unwrap().insert(
            id,
            Item {
                url: "https://example.test/download.bin".into(),
                server_ip: None,
                server_country: None,
                server_country_code: None,
                location_lookup_pending: false,
                path: path.clone(),
                total: Some(8),
                resumable: true,
                file_created: true,
                status: Status::Paused,
                proxy_used: false,
                restart_after_cancel: false,
                connections: 8,
                metadata_pending: false,
                removing: false,
                segs: Vec::new(),
                cancel: Arc::new(AtomicBool::new(false)),
                limit: Arc::new(AtomicU64::new(0)),
                speed: 0.0,
                last: 0,
            },
        );
        manager.next_id.store(id + 1, Relaxed);
        (manager, path, id)
    }

    #[tokio::test]
    async fn disabling_proxy_hides_proxy_indicator_for_active_downloads() {
        let (manager, path, id) = manager_with_download();
        {
            let mut items = manager.items.lock().unwrap();
            let item = items.get_mut(&id).unwrap();
            item.status = Status::Downloading;
            item.proxy_used = true;
        }
        let enabled = ProxySettings {
            enabled: true,
            host: "127.0.0.1".into(),
            port: 8080,
            ..ProxySettings::default()
        };
        manager.set_proxy_settings(enabled.clone()).unwrap();
        assert!(manager.view(id).unwrap().proxy_used);
        {
            let items = manager.items.lock().unwrap();
            let item = items.get(&id).unwrap();
            assert!(item.cancel.load(Relaxed), "active transfer should be interrupted to switch clients");
            assert!(item.restart_after_cancel, "active transfer should be scheduled to resume");
        }

        let disabled = ProxySettings { enabled: false, ..enabled };
        manager.set_proxy_settings(disabled).unwrap();
        assert!(!manager.view(id).unwrap().proxy_used);

        manager.items.lock().unwrap().get_mut(&id).unwrap().status = Status::Paused;
        manager.remove(id, false).await.unwrap();
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[tokio::test]
    async fn remove_deletes_the_file_when_requested() {
        let (manager, path, id) = manager_with_download();

        manager.remove(id, true).await.unwrap();

        assert!(!path.exists());
        assert!(manager.view(id).is_none());
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[tokio::test]
    async fn remove_keeps_the_file_when_not_requested() {
        let (manager, path, id) = manager_with_download();

        manager.remove(id, false).await.unwrap();

        assert!(path.exists());
        assert!(manager.view(id).is_none());
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[tokio::test]
    async fn failed_file_removal_keeps_download_available() {
        let (manager, path, id) = manager_with_download();
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();

        let error = manager.remove(id, true).await.unwrap_err();

        assert!(error.starts_with("Could not delete downloaded file:"));
        assert!(manager.view(id).is_some());
        manager.remove(id, false).await.unwrap();
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[tokio::test]
    async fn add_returns_while_metadata_probe_is_pending() {
        let (manager, path, original_id) = manager_with_download();
        let unrelated_file = path.parent().unwrap().join("slow.bin");
        std::fs::write(&unrelated_file, b"preserve").unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            std::future::pending::<()>().await;
        });

        let started = Instant::now();
        let id = manager
            .add(
                format!("http://{address}/slow.bin"),
                path.parent().unwrap().to_path_buf(),
                8,
            )
            .unwrap();

        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(manager.view(id).unwrap().status, "preparing");
        manager.remove(id, true).await.unwrap();
        assert_eq!(std::fs::read(&unrelated_file).unwrap(), b"preserve");
        server.abort();
        manager.remove(original_id, false).await.unwrap();
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn file_creation_does_not_overwrite_existing_downloads() {
        let dir = std::env::temp_dir().join(format!(
            "irani-idm-file-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let original = dir.join("file.bin");
        std::fs::write(&original, b"keep").unwrap();

        let created = create_download_file(&dir, "file.bin", Some(2)).unwrap();

        assert_eq!(std::fs::read(&original).unwrap(), b"keep");
        assert_eq!(created.file_name().unwrap(), "file (1).bin.downloading");
        assert_eq!(std::fs::metadata(created).unwrap().len(), 2);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn completed_download_is_renamed_to_its_original_filename() {
        let dir = std::env::temp_dir().join(format!(
            "irani-idm-complete-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let temporary = create_download_file(&dir, "movie.mp4", Some(4)).unwrap();
        let final_path = complete_download_file(&temporary).unwrap();

        assert_eq!(final_path.file_name().unwrap(), "movie.mp4");
        assert!(final_path.exists());
        assert!(!temporary.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn completion_does_not_overwrite_an_existing_final_file() {
        let dir = std::env::temp_dir().join(format!(
            "irani-idm-collision-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let final_path = dir.join("movie.mp4");
        std::fs::write(&final_path, b"keep").unwrap();
        let temporary = dir.join("movie.mp4.downloading");
        std::fs::write(&temporary, b"part").unwrap();

        assert!(complete_download_file(&temporary).is_err());
        assert_eq!(std::fs::read(&final_path).unwrap(), b"keep");
        assert_eq!(std::fs::read(&temporary).unwrap(), b"part");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn startup_marks_legacy_incomplete_files_as_downloading() {
        let data_dir = std::env::temp_dir().join(format!(
            "irani-idm-migration-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let download_dir = data_dir.join("downloads");
        std::fs::create_dir_all(&download_dir).unwrap();
        let old_path = download_dir.join("legacy.zip");
        std::fs::write(&old_path, b"partial").unwrap();
        let saved = Saved {
            id: 1,
            url: "https://example.test/legacy.zip".into(),
            server_ip: None,
            server_country: None,
            server_country_code: None,
            path: old_path.clone(),
            total: Some(10),
            resumable: true,
            file_created: true,
            connections: 1,
            metadata_pending: false,
            completed: false,
            proxy_used: false,
            error: None,
            limit_kbps: 0,
            segs: vec![SavedSeg { start: 0, end: 9, done: 7 }],
        };
        std::fs::write(data_dir.join("downloads.json"), serde_json::to_vec(&vec![saved]).unwrap())
            .unwrap();

        let manager = Manager::load(&data_dir);
        let migrated_path = manager.path_of(1).unwrap();

        assert_eq!(migrated_path.file_name().unwrap(), "legacy.zip.downloading");
        assert!(!old_path.exists());
        assert_eq!(std::fs::read(migrated_path).unwrap(), b"partial");
        std::fs::remove_dir_all(data_dir).unwrap();
    }

    #[tokio::test]
    async fn rate_limiter_caps_combined_connections() {
        let limiter = Arc::new(RateLimiter::new());
        let limit = Arc::new(AtomicU64::new(10));
        let cancel = Arc::new(AtomicBool::new(false));
        let started = Instant::now();

        let workers: Vec<_> = (0..2)
            .map(|_| {
                let limiter = limiter.clone();
                let limit = limit.clone();
                let cancel = cancel.clone();
                tokio::spawn(async move {
                    limiter.acquire(1024, &limit, &cancel).await
                })
            })
            .collect();

        for worker in workers {
            assert!(worker.await.unwrap());
        }
        assert!(started.elapsed() >= Duration::from_millis(150));
    }
}
