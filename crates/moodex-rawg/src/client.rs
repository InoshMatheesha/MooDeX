// client.rs - RAWG HTTP client with stale-while-revalidate cache and a semaphore of 4 (spec 9.3, 9.4).

use crate::error::RawgError;
use crate::wire::{Page, RawgGameDetail, WireDetail, WirePage};
use moodex_core::RawgGame;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use time::macros::format_description;
use time::{Date, OffsetDateTime};
use tokio::sync::{broadcast, Semaphore};

const PAGE_SIZE: &str = "20";

pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// One cached response body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntry {
    pub body: Vec<u8>,
    pub etag: Option<String>,
    pub fetched_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}

// Storage for cached responses; implemented by the library actor so only it touches SQLite.
pub trait HttpCache: Send + Sync + 'static {
    fn get(&self, key: String) -> BoxFut<'_, Option<CacheEntry>>;
    fn put(&self, key: String, entry: CacheEntry) -> BoxFut<'_, ()>;
}

// Where requests go - the key never lives in the binary (spec 9.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeySource {
    Proxy { base_url: String },
    User { key: String },
    None,
}

impl KeySource {
    pub fn kind(&self) -> &'static str {
        match self {
            KeySource::Proxy { .. } => "proxy",
            KeySource::User { .. } => "user",
            KeySource::None => "none",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawgConfig {
    pub user_agent: String,
    pub timeout: Duration,
    pub api_base: String,
    pub max_concurrent: usize,
}

impl Default for RawgConfig {
    fn default() -> Self {
        Self {
            user_agent: format!("MooDeX/{}", env!("CARGO_PKG_VERSION")),
            timeout: Duration::from_secs(15),
            api_base: "https://api.rawg.io/api".into(),
            max_concurrent: 4,
        }
    }
}

// A cacheable request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Endpoint {
    Search { query: String, page: u32 },
    Trending { page: u32 },
    Upcoming { page: u32 },
    Popular { page: u32 },
    Details { id: i64 },
}

impl Endpoint {
    fn ttl(&self) -> Duration {
        match self {
            Endpoint::Search { .. } => Duration::from_secs(15 * 60),
            Endpoint::Trending { .. } | Endpoint::Popular { .. } => Duration::from_secs(6 * 3600),
            Endpoint::Upcoming { .. } | Endpoint::Details { .. } => Duration::from_secs(24 * 3600),
        }
    }

    // Path and query, without any credential.
    fn request(&self, today: Date) -> (String, Vec<(String, String)>) {
        let fmt = format_description!("[year]-[month]-[day]");
        let d = |x: Date| x.format(&fmt).unwrap_or_default();
        let page = |p: &u32| ("page".to_string(), p.max(&1).to_string());
        let size = ("page_size".to_string(), PAGE_SIZE.to_string());
        match self {
            Endpoint::Search { query, page: p } => (
                "games".into(),
                vec![("search".into(), query.clone()), ("search_precise".into(), "true".into()), page(p), size],
            ),
            Endpoint::Trending { page: p } => {
                let from = today.checked_sub(time::Duration::days(182)).unwrap_or(today);
                (
                    "games".into(),
                    vec![
                        ("dates".into(), format!("{},{}", d(from), d(today))),
                        ("ordering".into(), "-added".into()),
                        page(p),
                        size,
                    ],
                )
            }
            Endpoint::Upcoming { page: p } => {
                let from = today.next_day().unwrap_or(today);
                let to = today.checked_add(time::Duration::days(365)).unwrap_or(today);
                (
                    "games".into(),
                    vec![
                        ("dates".into(), format!("{},{}", d(from), d(to))),
                        ("ordering".into(), "-added".into()),
                        page(p),
                        size,
                    ],
                )
            }
            Endpoint::Popular { page: p } => {
                ("games".into(), vec![("ordering".into(), "-added".into()), page(p), size])
            }
            Endpoint::Details { id } => (format!("games/{id}"), vec![]),
        }
    }

    fn cache_key(&self, today: Date) -> String {
        let (path, mut q) = self.request(today);
        q.sort();
        let qs: Vec<String> = q.into_iter().map(|(k, v)| format!("{k}={v}")).collect();
        format!("rawg:{path}?{}", qs.join("&"))
    }
}

enum Fetched {
    Body { body: Vec<u8>, etag: Option<String> },
    NotModified,
}

struct Inner {
    http: reqwest::Client,
    cfg: RawgConfig,
    source: RwLock<KeySource>,
    sem: Semaphore,
    cache: Arc<dyn HttpCache>,
    updates: broadcast::Sender<Endpoint>,
    revalidating: Mutex<HashSet<String>>,
}

#[derive(Clone)]
pub struct RawgClient {
    inner: Arc<Inner>,
}

impl RawgClient {
    pub fn new(cfg: RawgConfig, cache: Arc<dyn HttpCache>, source: KeySource) -> Result<Self, RawgError> {
        let http = reqwest::Client::builder()
            .user_agent(cfg.user_agent.clone())
            .timeout(cfg.timeout)
            .connect_timeout(Duration::from_secs(8))
            .build()
            .map_err(|e| RawgError::Http(e.to_string()))?;
        let (updates, _) = broadcast::channel(32);
        let sem = Semaphore::new(cfg.max_concurrent.max(1));
        Ok(Self {
            inner: Arc::new(Inner {
                http,
                cfg,
                source: RwLock::new(source),
                sem,
                cache,
                updates,
                revalidating: Mutex::new(HashSet::new()),
            }),
        })
    }

    // Swap the key source at runtime (settings change).
    pub fn set_source(&self, s: KeySource) {
        match self.inner.source.write() {
            Ok(mut g) => *g = s,
            Err(p) => *p.into_inner() = s,
        }
    }

    pub fn source_kind(&self) -> &'static str {
        self.source().kind()
    }

    fn source(&self) -> KeySource {
        match self.inner.source.read() {
            Ok(g) => g.clone(),
            Err(p) => p.into_inner().clone(),
        }
    }

    // Endpoints whose cached content changed after background revalidation.
    pub fn subscribe_updates(&self) -> broadcast::Receiver<Endpoint> {
        self.inner.updates.subscribe()
    }

    pub async fn search(&self, query: &str, page: u32) -> Result<Page<RawgGame>, RawgError> {
        let q = query.trim();
        if q.is_empty() {
            return Ok(Page::default());
        }
        self.page(Endpoint::Search { query: q.to_string(), page }).await
    }

    pub async fn trending(&self, page: u32) -> Result<Page<RawgGame>, RawgError> {
        self.page(Endpoint::Trending { page }).await
    }

    pub async fn upcoming(&self, page: u32) -> Result<Page<RawgGame>, RawgError> {
        self.page(Endpoint::Upcoming { page }).await
    }

    pub async fn popular(&self, page: u32) -> Result<Page<RawgGame>, RawgError> {
        self.page(Endpoint::Popular { page }).await
    }

    pub async fn details(&self, rawg_id: i64) -> Result<RawgGameDetail, RawgError> {
        let w: WireDetail = self.get_json(Endpoint::Details { id: rawg_id }).await?;
        Ok(w.into())
    }

    pub async fn page(&self, ep: Endpoint) -> Result<Page<RawgGame>, RawgError> {
        let w: WirePage = self.get_json(ep).await?;
        Ok(w.into())
    }

    // Serve from cache when present; revalidate stale entries in the background.
    async fn get_json<T: DeserializeOwned>(&self, ep: Endpoint) -> Result<T, RawgError> {
        let today = OffsetDateTime::now_utc().date();
        let key = ep.cache_key(today);
        let now = OffsetDateTime::now_utc();
        if let Some(entry) = self.inner.cache.get(key.clone()).await {
            match serde_json::from_slice::<T>(&entry.body) {
                Ok(v) => {
                    if now >= entry.expires_at {
                        self.spawn_revalidate(ep, key, entry);
                    }
                    return Ok(v);
                }
                Err(e) => tracing::warn!(%key, error = %e, "discarding unreadable cache entry"),
            }
        }
        match self.fetch(&ep, today, None).await? {
            Fetched::Body { body, etag } => {
                let v = serde_json::from_slice::<T>(&body).map_err(|e| RawgError::Decode(e.to_string()))?;
                let entry = self.entry(body, etag, &ep);
                self.inner.cache.put(key, entry).await;
                Ok(v)
            }
            Fetched::NotModified => Err(RawgError::Decode("304 without a cached copy".into())),
        }
    }

    fn entry(&self, body: Vec<u8>, etag: Option<String>, ep: &Endpoint) -> CacheEntry {
        let now = OffsetDateTime::now_utc();
        let ttl = time::Duration::try_from(ep.ttl()).unwrap_or(time::Duration::hours(1));
        CacheEntry { body, etag, fetched_at: now, expires_at: now + ttl }
    }

    fn spawn_revalidate(&self, ep: Endpoint, key: String, old: CacheEntry) {
        {
            let mut set = match self.inner.revalidating.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            if !set.insert(key.clone()) {
                return;
            }
        }
        let this = self.clone();
        tokio::spawn(async move {
            let today = OffsetDateTime::now_utc().date();
            let res = this.fetch(&ep, today, old.etag.clone()).await;
            match res {
                Ok(Fetched::NotModified) => {
                    let e = this.entry(old.body, old.etag, &ep);
                    this.inner.cache.put(key.clone(), e).await;
                }
                Ok(Fetched::Body { body, etag }) => {
                    let changed = body != old.body;
                    let e = this.entry(body, etag, &ep);
                    this.inner.cache.put(key.clone(), e).await;
                    if changed && this.inner.updates.send(ep).is_err() {
                        tracing::debug!("no listeners for discover updates");
                    }
                }
                Err(e) => tracing::debug!(%key, error = %e, "background revalidation failed; keeping stale copy"),
            }
            let mut set = match this.inner.revalidating.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            set.remove(&key);
        });
    }

    async fn fetch(&self, ep: &Endpoint, today: Date, etag: Option<String>) -> Result<Fetched, RawgError> {
        let (path, mut query) = ep.request(today);
        let url = match self.source() {
            KeySource::Proxy { base_url } if !base_url.trim().is_empty() => {
                format!("{}/{}", base_url.trim().trim_end_matches('/'), path)
            }
            KeySource::User { key } if !key.trim().is_empty() => {
                query.push(("key".into(), key.trim().to_string()));
                format!("{}/{}", self.inner.cfg.api_base.trim_end_matches('/'), path)
            }
            _ => return Err(RawgError::NoKey),
        };
        let _permit = self.inner.sem.acquire().await.map_err(|e| RawgError::Http(e.to_string()))?;
        let mut req = self.inner.http.get(&url).query(&query);
        if let Some(t) = etag {
            req = req.header(reqwest::header::IF_NONE_MATCH, t);
        }
        let span = tracing::debug_span!("rawg_request", %path);
        let resp = tracing::Instrument::instrument(req.send(), span).await.map_err(|e| {
            if e.is_connect() || e.is_timeout() {
                RawgError::Offline
            } else {
                RawgError::Http(e.without_url().to_string())
            }
        })?;
        let status = resp.status();
        if status == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(Fetched::NotModified);
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let secs = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok())
                .unwrap_or(30);
            return Err(RawgError::RateLimited(Duration::from_secs(secs)));
        }
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            tracing::warn!(status = status.as_u16(), "RAWG rejected the key");
        }
        if !status.is_success() {
            return Err(RawgError::Upstream(status.as_u16()));
        }
        let etag = resp.headers().get(reqwest::header::ETAG).and_then(|v| v.to_str().ok()).map(str::to_string);
        let body = resp.bytes().await.map_err(|e| RawgError::Http(e.without_url().to_string()))?;
        Ok(Fetched::Body { body: body.to_vec(), etag })
    }
}

#[cfg(test)]
mod tests;
