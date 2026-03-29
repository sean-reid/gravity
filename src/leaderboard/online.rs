//! Online leaderboard client for the Gravity Well Arena backend.
//!
//! All network I/O happens off the main thread. The game loop calls [`OnlineLeaderboard::poll`]
//! each frame to drain completed results from an internal channel. Nothing here will ever block
//! the game thread.

use std::sync::mpsc;

// ---------------------------------------------------------------------------
// Default backend URL (overridable at construction time)
// ---------------------------------------------------------------------------

/// Default base URL for the Cloudflare Worker leaderboard API.
pub const DEFAULT_BASE_URL: &str = "https://gravity-well-arena-leaderboard.seanreid.workers.dev";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Current connection status of the online leaderboard client.
#[derive(Debug, Clone)]
pub enum OnlineStatus {
    /// No operation in flight.
    Idle,
    /// A registration or initial request is in progress.
    Connecting,
    /// Successfully registered with the backend.
    Connected,
    /// The most recent operation produced an error.
    Error(String),
}

/// A result delivered asynchronously from a background network operation.
#[derive(Debug)]
pub enum OnlineResult {
    /// Registration succeeded.
    Registered { player_id: String },
    /// A score was accepted by the server.
    ScoreSubmitted { rank: u64, total: u64 },
    /// A leaderboard page was fetched.
    LeaderboardFetched { entries: Vec<OnlineEntry>, total: u64 },
    /// A player stats response.
    PlayerStats { data: String },
    /// Something went wrong.
    Error(String),
}

/// One row in a leaderboard response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OnlineEntry {
    #[serde(default)]
    pub rank: u64,
    #[serde(default)]
    pub player_id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub score: u64,
    #[serde(default)]
    pub proper_time: f64,
    #[serde(default)]
    pub accuracy: f64,
    #[serde(default)]
    pub health_remaining: f64,
    #[serde(default)]
    pub deepest_altitude: f64,
    #[serde(default)]
    pub level_number: u32,
    #[serde(default)]
    pub timestamp: String,
}

/// Payload sent to `POST /api/scores`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScoreSubmission {
    pub player_id: String,
    pub level_number: u32,
    pub seed: u64,
    pub score: u64,
    pub proper_time: f64,
    pub coordinate_time: f64,
    pub accuracy: f64,
    pub health_remaining: f64,
    pub deepest_altitude: f64,
    pub bots_killed: u32,
    pub bots_spaghettified: u32,
    pub shots_fired: u32,
    pub shots_hit: u32,
    pub damage_taken: f64,
    pub dilation_ratio: f64,
}

// ---------------------------------------------------------------------------
// OnlineLeaderboard
// ---------------------------------------------------------------------------

/// Non-blocking online leaderboard client.
///
/// All network requests execute on background threads (native) or via
/// `spawn_local` (WASM). Call [`poll`](Self::poll) once per frame to process
/// completed results.
pub struct OnlineLeaderboard {
    base_url: String,
    player_id: Option<String>,
    display_name: String,

    // Channel pair – senders are cloned into background tasks.
    result_sender: mpsc::Sender<OnlineResult>,
    result_receiver: mpsc::Receiver<OnlineResult>,

    // Scores queued while we wait for registration to complete.
    pending_submissions: Vec<ScoreSubmission>,

    // Public observable state --------------------------------------------------
    /// The rank returned by the most recent score submission: `(rank, total)`.
    pub last_rank: Option<(u64, u64)>,
    /// Seed of the last submitted score, for auto-fetching leaderboard after submission.
    last_submitted_seed: Option<u64>,
    /// The most recently fetched leaderboard page.
    pub cached_leaderboard: Vec<OnlineEntry>,
    /// Whether a leaderboard fetch has completed (true even if empty).
    pub leaderboard_fetched: bool,
    /// Total entries reported by the last leaderboard fetch.
    pub cached_leaderboard_total: u64,
    /// Connection / error status.
    pub status: OnlineStatus,
}

impl OnlineLeaderboard {
    // -- Construction --------------------------------------------------------

    /// Create a new client. No network calls happen until you explicitly call
    /// [`register`](Self::register) or another method.
    pub fn new(base_url: String, display_name: String) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            base_url,
            player_id: None,
            display_name,
            result_sender: tx,
            result_receiver: rx,
            pending_submissions: Vec::new(),
            last_rank: None,
            last_submitted_seed: None,
            cached_leaderboard: Vec::new(),
            leaderboard_fetched: false,
            cached_leaderboard_total: 0,
            status: OnlineStatus::Idle,
        }
    }

    /// Convenience constructor that uses the default backend URL.
    pub fn with_default_url(display_name: String) -> Self {
        Self::new(DEFAULT_BASE_URL.to_owned(), display_name)
    }

    // -- Polling -------------------------------------------------------------

    /// Drain all completed network results and update internal state.
    ///
    /// Call once per frame. This never blocks.
    pub fn poll(&mut self) {
        while let Ok(result) = self.result_receiver.try_recv() {
            match result {
                OnlineResult::Registered { player_id } => {
                    log::info!("Online leaderboard: registered as {player_id}");
                    self.player_id = Some(player_id);
                    self.status = OnlineStatus::Connected;
                    // Flush anything that was queued before registration.
                    self.drain_pending();
                }
                OnlineResult::ScoreSubmitted { rank, total } => {
                    log::info!("Online leaderboard: rank {rank}/{total}");
                    self.last_rank = Some((rank, total));
                    // Score accepted — clear pending queue
                    self.pending_submissions.clear();
                    // Auto-fetch leaderboard now that our score is in
                    if let Some(seed) = self.last_submitted_seed {
                        self.fetch_leaderboard(seed, 10);
                    }
                }
                OnlineResult::LeaderboardFetched { entries, total } => {
                    self.cached_leaderboard = entries;
                    self.cached_leaderboard_total = total;
                    self.leaderboard_fetched = true;
                }
                OnlineResult::PlayerStats { data: _ } => {
                    // Reserved for future UI display.
                }
                OnlineResult::Error(msg) => {
                    log::warn!("Online leaderboard error: {msg}");
                    // If we got a 404, our player ID is stale — re-register
                    // and re-queue the last score submission
                    if msg.contains("404") && self.player_id.is_some() {
                        log::info!("Player not found on server, re-registering...");
                        self.player_id = None;
                        self.register();
                        // drain_pending will fire after registration completes
                    }
                    self.status = OnlineStatus::Error(msg);
                }
            }
        }
    }

    // -- Registration --------------------------------------------------------

    /// Register with the backend (POST /api/register).
    ///
    /// On success an `OnlineResult::Registered` will appear in the channel.
    pub fn register(&mut self) {
        self.status = OnlineStatus::Connecting;
        let url = format!("{}/api/register", self.base_url);
        let body = serde_json::json!({ "display_name": self.display_name }).to_string();
        let sender = self.result_sender.clone();

        http_post(url, body, sender, |response_body| {
            #[derive(serde::Deserialize)]
            struct Resp {
                player_id: String,
                #[allow(dead_code)]
                display_name: String,
            }
            match serde_json::from_str::<Resp>(&response_body) {
                Ok(r) => OnlineResult::Registered {
                    player_id: r.player_id,
                },
                Err(e) => OnlineResult::Error(format!("register parse error: {e}")),
            }
        });
    }

    // -- Score submission ----------------------------------------------------

    /// Submit a score. If we are not yet registered the submission is queued
    /// and will be sent automatically once registration completes.
    pub fn submit_score(&mut self, mut submission: ScoreSubmission) {
        self.last_submitted_seed = Some(submission.seed);
        // Always keep a copy in pending so it can be re-sent after re-registration
        self.pending_submissions.push(submission.clone());
        match &self.player_id {
            Some(pid) => {
                submission.player_id = pid.clone();
                let url = format!("{}/api/scores", self.base_url);
                let body = match serde_json::to_string(&submission) {
                    Ok(b) => b,
                    Err(e) => {
                        log::error!("Failed to serialize score submission: {e}");
                        return;
                    }
                };
                let sender = self.result_sender.clone();
                http_post(url, body, sender, |response_body| {
                    #[derive(serde::Deserialize)]
                    struct Resp {
                        rank: u64,
                        total_entries: u64,
                    }
                    match serde_json::from_str::<Resp>(&response_body) {
                        Ok(r) => OnlineResult::ScoreSubmitted {
                            rank: r.rank,
                            total: r.total_entries,
                        },
                        Err(e) => OnlineResult::Error(format!("score submit parse error: {e}")),
                    }
                });
            }
            None => {
                // Not registered yet – already queued above, will drain after registration.
            }
        }
    }

    /// Try to submit all queued scores (called automatically after registration).
    pub fn drain_pending(&mut self) {
        if self.player_id.is_none() {
            return;
        }
        let queued: Vec<ScoreSubmission> = self.pending_submissions.drain(..).collect();
        for sub in queued {
            self.submit_score(sub);
        }
    }

    // -- Leaderboard fetching ------------------------------------------------

    /// Fetch the top `limit` entries for a given seed.
    pub fn fetch_leaderboard(&mut self, seed: u64, limit: u32) {
        self.leaderboard_fetched = false;
        let url = format!(
            "{}/api/leaderboard/{}?limit={}&offset=0",
            self.base_url, seed, limit
        );
        let sender = self.result_sender.clone();
        http_get(url, sender, parse_leaderboard_response);
    }

    /// Fetch entries around the current player for a given seed.
    ///
    /// Does nothing if not registered.
    pub fn fetch_around_player(&self, seed: u64, range: u32) {
        let pid = match &self.player_id {
            Some(p) => p.clone(),
            None => return,
        };
        let url = format!(
            "{}/api/leaderboard/{}/around/{}?range={}",
            self.base_url, seed, pid, range
        );
        let sender = self.result_sender.clone();
        http_get(url, sender, parse_leaderboard_response);
    }

    /// Fetch stats for the current player.
    ///
    /// Does nothing if not registered.
    pub fn fetch_player_stats(&self) {
        let pid = match &self.player_id {
            Some(p) => p.clone(),
            None => return,
        };
        let url = format!("{}/api/player/{}/stats", self.base_url, pid);
        let sender = self.result_sender.clone();
        http_get(url, sender, |body| OnlineResult::PlayerStats { data: body });
    }

    /// Update the display name on the server (PATCH /api/player/:player_id).
    ///
    /// Does nothing if not registered.
    pub fn update_display_name(&mut self, new_name: String) {
        let pid = match &self.player_id {
            Some(p) => p.clone(),
            None => return,
        };
        self.display_name = new_name.clone();
        let url = format!("{}/api/player/{}", self.base_url, pid);
        let body = serde_json::json!({ "display_name": new_name }).to_string();
        let sender = self.result_sender.clone();
        http_patch(url, body, sender, |_| {
            // We don't surface a dedicated result for name changes.
            // Just swallow the response; errors still go through Error.
            OnlineResult::PlayerStats {
                data: String::new(),
            }
        });
    }

    // -- Accessors -----------------------------------------------------------

    /// Whether the client has a player_id (i.e., registration completed).
    pub fn is_registered(&self) -> bool {
        self.player_id.is_some()
    }

    /// Return the player_id, if registered.
    pub fn player_id(&self) -> Option<&str> {
        self.player_id.as_deref()
    }

    /// Return the display name.
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Manually set a previously saved player_id (e.g. loaded from disk)
    /// so that we skip registration.
    pub fn set_player_id(&mut self, id: String) {
        self.player_id = Some(id);
        self.status = OnlineStatus::Connected;
    }

    /// Update the display name used for registration.
    pub fn set_display_name(&mut self, name: String) {
        self.display_name = name;
    }
}

// ---------------------------------------------------------------------------
// Shared response parser
// ---------------------------------------------------------------------------

fn parse_leaderboard_response(body: String) -> OnlineResult {
    #[derive(serde::Deserialize)]
    struct Resp {
        entries: Vec<OnlineEntry>,
        total: u64,
    }
    match serde_json::from_str::<Resp>(&body) {
        Ok(r) => OnlineResult::LeaderboardFetched {
            entries: r.entries,
            total: r.total,
        },
        Err(e) => OnlineResult::Error(format!("leaderboard parse error: {e}")),
    }
}

// ===========================================================================
// Platform-specific HTTP helpers
// ===========================================================================

// The handler closure is `fn(String) -> OnlineResult` — a plain function
// pointer so it is trivially Send + 'static.

// ---------------------------------------------------------------------------
// Native: ureq on a background std::thread
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
fn http_post(
    url: String,
    body: String,
    sender: mpsc::Sender<OnlineResult>,
    handler: fn(String) -> OnlineResult,
) {
    std::thread::Builder::new()
        .name("online-post".into())
        .spawn(move || {
            let result = native_post(&url, &body, handler);
            let _ = sender.send(result);
        })
        .ok(); // If thread spawn fails we silently drop — the game continues.
}

#[cfg(not(target_arch = "wasm32"))]
fn http_get(
    url: String,
    sender: mpsc::Sender<OnlineResult>,
    handler: fn(String) -> OnlineResult,
) {
    std::thread::Builder::new()
        .name("online-get".into())
        .spawn(move || {
            let result = native_get(&url, handler);
            let _ = sender.send(result);
        })
        .ok();
}

#[cfg(not(target_arch = "wasm32"))]
fn http_patch(
    url: String,
    body: String,
    sender: mpsc::Sender<OnlineResult>,
    handler: fn(String) -> OnlineResult,
) {
    std::thread::Builder::new()
        .name("online-patch".into())
        .spawn(move || {
            let result = native_patch(&url, &body, handler);
            let _ = sender.send(result);
        })
        .ok();
}

#[cfg(not(target_arch = "wasm32"))]
fn native_post(url: &str, body: &str, handler: fn(String) -> OnlineResult) -> OnlineResult {
    match ureq::post(url)
        .set("Content-Type", "application/json")
        .send_string(body)
    {
        Ok(resp) => match resp.into_string() {
            Ok(text) => handler(text),
            Err(e) => OnlineResult::Error(format!("read response: {e}")),
        },
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            OnlineResult::Error(format!("HTTP {code}: {text}"))
        }
        Err(e) => OnlineResult::Error(format!("network: {e}")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn native_get(url: &str, handler: fn(String) -> OnlineResult) -> OnlineResult {
    match ureq::get(url).call() {
        Ok(resp) => match resp.into_string() {
            Ok(text) => handler(text),
            Err(e) => OnlineResult::Error(format!("read response: {e}")),
        },
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            OnlineResult::Error(format!("HTTP {code}: {text}"))
        }
        Err(e) => OnlineResult::Error(format!("network: {e}")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn native_patch(url: &str, body: &str, handler: fn(String) -> OnlineResult) -> OnlineResult {
    match ureq::request("PATCH", url)
        .set("Content-Type", "application/json")
        .send_string(body)
    {
        Ok(resp) => match resp.into_string() {
            Ok(text) => handler(text),
            Err(e) => OnlineResult::Error(format!("read response: {e}")),
        },
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            OnlineResult::Error(format!("HTTP {code}: {text}"))
        }
        Err(e) => OnlineResult::Error(format!("network: {e}")),
    }
}

// ---------------------------------------------------------------------------
// WASM: web-sys fetch + wasm-bindgen-futures::spawn_local
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
fn http_post(
    url: String,
    body: String,
    sender: mpsc::Sender<OnlineResult>,
    handler: fn(String) -> OnlineResult,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = wasm_fetch("POST", &url, Some(&body)).await;
        let online_result = match result {
            Ok(text) => handler(text),
            Err(e) => OnlineResult::Error(e),
        };
        let _ = sender.send(online_result);
    });
}

#[cfg(target_arch = "wasm32")]
fn http_get(
    url: String,
    sender: mpsc::Sender<OnlineResult>,
    handler: fn(String) -> OnlineResult,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = wasm_fetch("GET", &url, None).await;
        let online_result = match result {
            Ok(text) => handler(text),
            Err(e) => OnlineResult::Error(e),
        };
        let _ = sender.send(online_result);
    });
}

#[cfg(target_arch = "wasm32")]
fn http_patch(
    url: String,
    body: String,
    sender: mpsc::Sender<OnlineResult>,
    handler: fn(String) -> OnlineResult,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = wasm_fetch("PATCH", &url, Some(&body)).await;
        let online_result = match result {
            Ok(text) => handler(text),
            Err(e) => OnlineResult::Error(e),
        };
        let _ = sender.send(online_result);
    });
}

#[cfg(target_arch = "wasm32")]
async fn wasm_fetch(method: &str, url: &str, body: Option<&str>) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let mut opts = web_sys::RequestInit::new();
    opts.method(method);
    opts.mode(web_sys::RequestMode::Cors);

    if let Some(b) = body {
        opts.body(Some(&wasm_bindgen::JsValue::from_str(b)));
    }

    let request =
        web_sys::Request::new_with_str_and_init(url, &opts).map_err(|e| format!("{e:?}"))?;

    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_owned())?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch: {e:?}"))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| "response cast failed".to_owned())?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text_promise = resp.text().map_err(|e| format!("{e:?}"))?;
    let text_value = JsFuture::from(text_promise)
        .await
        .map_err(|e| format!("text: {e:?}"))?;

    text_value
        .as_string()
        .ok_or_else(|| "response was not a string".to_owned())
}
