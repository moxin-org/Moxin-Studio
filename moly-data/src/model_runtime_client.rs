//! Runtime client for the ominix-api load/unload/status endpoints.
//!
//! All requests go to localhost:8080 (ominix-api) and follow the API contracts:
//!
//!   GET  /v1/models               → list + status of every loaded model
//!   POST /v1/models/{id}/load     → load a model into memory (blocks until done)
//!   POST /v1/models/{id}/unload   → free the model from memory

use serde::Deserialize;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};

/// Handle to the ominix-api child process we launched (None if we didn't launch it).
static SERVER_CHILD: Mutex<Option<std::process::Child>> = Mutex::new(None);

/// PID of the ominix-api server (whether we spawned it or it was already running).
/// Stored as AtomicI32 so it can be read safely from signal handlers.
static SERVER_PID: AtomicI32 = AtomicI32::new(0);

/// Record the PID of an externally-running ominix-api server.
pub fn set_server_pid(pid: i32) {
    SERVER_PID.store(pid, Ordering::Relaxed);
}

/// Kill the ominix-api process and all its children.
/// Safe to call from signal handlers (uses only atomic reads + libc::kill).
pub fn kill_server_process() {
    let pid = SERVER_PID.swap(0, Ordering::Relaxed);
    if pid > 0 {
        #[cfg(unix)]
        unsafe {
            // Kill child processes, then the server itself
            libc::kill(-pid, libc::SIGKILL); // kill process group if it's a leader
            libc::kill(pid, libc::SIGKILL);  // kill the process directly
        }
    }
    // Also reap our child handle if we spawned it
    if let Ok(mut guard) = SERVER_CHILD.try_lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Detect the PID of an already-running ominix-api via `pgrep`.
fn detect_and_store_server_pid() {
    if let Ok(output) = std::process::Command::new("pgrep")
        .args(["-f", "ominix-api"])
        .output()
    {
        if let Ok(s) = std::str::from_utf8(&output.stdout) {
            if let Some(pid) = s.lines().next().and_then(|l| l.trim().parse::<i32>().ok()) {
                log::info!("Detected running ominix-api (pid {})", pid);
                SERVER_PID.store(pid, Ordering::Relaxed);
            }
        }
    }
}

// ─── Server-side model status ─────────────────────────────────────────────────

/// Status as reported by the ominix-api `/v1/models` endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerModelStatus {
    Loaded,
    Loading,
    Unloaded,
    Error,
}

impl ServerModelStatus {
    fn from_str(s: &str) -> Self {
        match s {
            "loaded"   => Self::Loaded,
            "loading"  => Self::Loading,
            "error"    => Self::Error,
            _          => Self::Unloaded,
        }
    }
}

/// One entry from `GET /v1/models`.
#[derive(Debug, Clone)]
pub struct ServerModelInfo {
    /// The model ID as known to the API (= RegistryRuntime::api_model_id)
    pub api_id:    String,
    pub status:    ServerModelStatus,
    pub memory_gb: Option<f32>,
}

// ─── Deserialisation helpers ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct ModelsListResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    memory_gb: Option<f32>,
}

// ─── Auto-launch helpers ──────────────────────────────────────────────────────

/// Locate the `ominix-api` binary by searching in order:
///
/// 1. `OMINIX_API_BIN` environment variable
/// 2. Directories on `PATH`
/// 3. Sibling of the running executable (production install layout)
/// 4. Dev layout: `../OminiX-API/target/release/ominix-api` relative to the
///    workspace root inferred from the current executable path
pub fn find_api_binary() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;

    // 1. Explicit override
    if let Ok(val) = std::env::var("OMINIX_API_BIN") {
        let p = PathBuf::from(val);
        if p.exists() { return Some(p); }
    }

    // 2. PATH
    if let Ok(path_env) = std::env::var("PATH") {
        for dir in path_env.split(':') {
            let candidate = PathBuf::from(dir).join("ominix-api");
            if candidate.exists() { return Some(candidate); }
        }
    }

    // 3. Sibling of running binary (e.g. /Applications/Moxin.app/Contents/MacOS/)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidate = parent.join("ominix-api");
            if candidate.exists() { return Some(candidate); }
        }
    }

    // 4. Dev layout:
    //    exe = .../Moxin-Studio/target/debug/moxin-studio
    //    api = .../OminiX-API/target/release/ominix-api   (release preferred)
    //          .../OminiX-API/target/debug/ominix-api     (fallback)
    if let Ok(exe) = std::env::current_exe() {
        // Walk up: moxin-studio -> debug -> target -> Moxin-Studio -> OminiX (workspace root)
        if let Some(ominix_root) = exe.ancestors().nth(4) {
            for build_kind in &["release", "debug"] {
                let candidate = ominix_root
                    .join("OminiX-API/target")
                    .join(build_kind)
                    .join("ominix-api");
                if candidate.exists() { return Some(candidate); }
            }
        }
    }

    None
}

/// Ensure the `ominix-api` server is running on localhost:8080.
///
/// * If it is already up, returns immediately.
/// * If not, tries to locate and launch the binary, then waits up to 30 s
///   (polling every 500 ms) for it to become ready.
///
/// Call this inside a background thread (it blocks).
pub fn ensure_server_running() -> Result<(), String> {
    let client = ModelRuntimeClient::localhost();

    // Already up? Find its PID so we can kill it on exit.
    if client.is_alive() {
        detect_and_store_server_pid();
        return Ok(());
    }

    // Find the binary
    let binary = find_api_binary().ok_or_else(|| {
        "ominix-api binary not found. Install it or set OMINIX_API_BIN.".to_string()
    })?;

    log::info!("Auto-starting ominix-api from {}", binary.display());

    let child = std::process::Command::new(&binary)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to launch ominix-api: {}", e))?;

    // Remember the child so we can kill it when the studio exits.
    let pid = child.id() as i32;
    SERVER_PID.store(pid, Ordering::Relaxed);
    if let Ok(mut guard) = SERVER_CHILD.lock() {
        *guard = Some(child);
    }

    // Poll until ready (max 30 s)
    for _ in 0..60 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if client.is_alive() {
            log::info!("ominix-api is ready");
            return Ok(());
        }
    }

    Err("ominix-api launched but did not become ready within 30 seconds".to_string())
}

// ─── Client ───────────────────────────────────────────────────────────────────

/// Thin blocking HTTP client for the ominix-api runtime endpoints.
///
/// All calls block the calling thread — run them inside `std::thread::spawn`.
pub struct ModelRuntimeClient {
    base_url: String,
}

impl ModelRuntimeClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let mut url = base_url.into();
        if url.ends_with('/') {
            url.pop();
        }
        Self { base_url: url }
    }

    pub fn localhost() -> Self {
        Self::new("http://localhost:8080")
    }

    // ── Liveness check ───────────────────────────────────────────────────────

    /// Returns `true` if the server responds to `GET /v1/models` within 2 s.
    pub fn is_alive(&self) -> bool {
        let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
        else { return false };

        let url = format!("{}/v1/models", self.base_url);
        client.get(&url).send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    // ── List ─────────────────────────────────────────────────────────────────

    /// `GET /v1/models` — returns status for every model known to the server.
    pub fn list_models(&self) -> Result<Vec<ServerModelInfo>, String> {
        let client = self.client(5)?;
        let url    = format!("{}/v1/models", self.base_url);
        let resp   = client.get(&url).send().map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }

        let body: ModelsListResponse = resp.json().map_err(|e| e.to_string())?;
        Ok(body.data.into_iter().map(|e| ServerModelInfo {
            api_id:    e.id,
            status:    ServerModelStatus::from_str(&e.status),
            memory_gb: e.memory_gb,
        }).collect())
    }

    // ── Load ──────────────────────────────────────────────────────────────────

    /// `POST /v1/models/load` — blocks until the model is ready.
    /// Large models may take several minutes.
    /// `model_type`: "llm", "vlm", "asr", "tts", or "image"
    pub fn load_model(&self, api_model_id: &str, model_type: &str) -> Result<(), String> {
        let client = self.client(600)?;          // 10-minute ceiling
        let url    = format!("{}/v1/models/load", self.base_url);
        let body   = serde_json::json!({ "model": api_model_id, "model_type": model_type });
        let resp   = client.post(&url).json(&body).send().map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let text   = resp.text().unwrap_or_default();
            Err(format!("HTTP {} — {}", status, text.trim()))
        }
    }

    // ── Unload ────────────────────────────────────────────────────────────────

    /// `POST /v1/models/unload` — frees the model from memory.
    /// `model_type`: "llm", "vlm", "asr", "tts", "image", or "all"
    pub fn unload_model(&self, model_type: &str) -> Result<(), String> {
        let client = self.client(30)?;
        let url    = format!("{}/v1/models/unload", self.base_url);
        let body   = serde_json::json!({ "model_type": model_type });
        let resp   = client.post(&url).json(&body).send().map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let text   = resp.text().unwrap_or_default();
            Err(format!("HTTP {} — {}", status, text.trim()))
        }
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    fn client(&self, timeout_secs: u64) -> Result<reqwest::blocking::Client, String> {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| e.to_string())
    }
}
