//! moodex-images - Content-addressed cover cache: memory LRU over disk, WebP variants, 6 fetches max (spec 10).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))] // tests may unwrap (spec 4.2)

use camino::{Utf8Path, Utf8PathBuf};
use image::{DynamicImage, ImageReader, Limits};
use moodex_core::ContentHash;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tokio::sync::{broadcast, OnceCell, Semaphore};

const MAX_DOWNLOAD: usize = 25 * 1024 * 1024;
const WEBP_QUALITY: f32 = 82.0;
const THUMB_QUALITY: f32 = 68.0;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ImageError {
    #[error("unknown image {0}")]
    Unknown(String),
    #[error("unsupported URL: {0}")]
    BadUrl(String),
    #[error("download failed: {0}")]
    Fetch(String),
    #[error("could not decode image: {0}")]
    Decode(String),
    #[error("i/o error: {0}")]
    Io(String),
}

impl From<std::io::Error> for ImageError {
    fn from(e: std::io::Error) -> Self {
        ImageError::Io(e.to_string())
    }
}

// Pre-generated sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Variant {
    Thumb,
    Hero,
}

impl Variant {
    fn width(self) -> u32 {
        match self {
            Variant::Thumb => 400,
            Variant::Hero => 1280,
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Variant::Thumb => "thumb",
            Variant::Hero => "hero",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "thumb" => Some(Variant::Thumb),
            "hero" => Some(Variant::Hero),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheHit {
    Ready(ContentHash),
    Pending,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct GcReport {
    pub files_before: usize,
    pub bytes_before: u64,
    pub removed: usize,
    pub bytes_after: u64,
}

// SHA-256 of the source URL - the content address (not MD5, spec 10).
pub fn hash_url(url: &str) -> ContentHash {
    ContentHash(hex::encode(Sha256::digest(url.trim().as_bytes())))
}

type Flight = Arc<OnceCell<Result<Arc<Vec<u8>>, ImageError>>>;

struct Inner {
    root: Utf8PathBuf,
    budget_bytes: u64,
    http: reqwest::Client,
    mem: moka::sync::Cache<(String, Variant), Arc<Vec<u8>>>,
    sem: Semaphore,
    urls: Mutex<HashMap<String, String>>,
    inflight: Mutex<HashMap<String, Flight>>,
    ready: broadcast::Sender<(String, ContentHash)>,
}

#[derive(Clone)]
pub struct ImageCache {
    inner: Arc<Inner>,
}

impl ImageCache {
    pub fn new(root: Utf8PathBuf, budget_bytes: u64) -> Result<Self, ImageError> {
        Self::with_memory(root, budget_bytes, 128 * 1024 * 1024)
    }

    pub fn with_memory(root: Utf8PathBuf, budget_bytes: u64, mem_bytes: u64) -> Result<Self, ImageError> {
        std::fs::create_dir_all(&root)?;
        let http = reqwest::Client::builder()
            .user_agent(format!("MooDeX/{}", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| ImageError::Fetch(e.to_string()))?;
        let mem = moka::sync::Cache::builder()
            .weigher(|_k: &(String, Variant), v: &Arc<Vec<u8>>| u32::try_from(v.len()).unwrap_or(u32::MAX))
            .max_capacity(mem_bytes)
            .build();
        let (ready, _) = broadcast::channel(256);
        Ok(Self {
            inner: Arc::new(Inner {
                root,
                budget_bytes,
                http,
                mem,
                sem: Semaphore::new(6),
                urls: Mutex::new(HashMap::new()),
                inflight: Mutex::new(HashMap::new()),
                ready,
            }),
        })
    }

    // Remember a URL so the protocol handler can fetch by hash; returns its address.
    pub fn register(&self, url: &str) -> ContentHash {
        let h = hash_url(url);
        let mut m = lock(&self.inner.urls);
        m.entry(h.0.clone()).or_insert_with(|| url.trim().to_string());
        h
    }

    // Fired as (url, hash) whenever a cover finishes caching.
    pub fn subscribe_ready(&self) -> broadcast::Receiver<(String, ContentHash)> {
        self.inner.ready.subscribe()
    }

    fn file_for(&self, hash: &str, v: Variant) -> Utf8PathBuf {
        let shard = hash.get(..2).unwrap_or("00");
        self.inner.root.join(shard).join(format!("{hash}-{}.webp", v.suffix()))
    }

    pub fn is_cached(&self, hash: &ContentHash) -> bool {
        self.inner.mem.contains_key(&(hash.0.clone(), Variant::Thumb))
            || self.file_for(&hash.0, Variant::Thumb).exists()
    }

    // Immediate answer: Ready if cached, else schedule a bounded fetch and return Pending.
    pub async fn get(&self, url: &str, variant: Variant) -> Result<CacheHit, ImageError> {
        check_url(url)?;
        let h = self.register(url);
        if self.inner.mem.contains_key(&(h.0.clone(), variant)) || self.file_for(&h.0, variant).exists() {
            return Ok(CacheHit::Ready(h));
        }
        let this = self.clone();
        let hash = h.0.clone();
        tokio::spawn(async move {
            if let Err(e) = this.ensure(&hash, variant).await {
                tracing::debug!(%hash, error = %e, "background cover fetch failed");
            }
        });
        Ok(CacheHit::Pending)
    }

    // Warm a batch, respecting the semaphore.
    pub async fn prefetch(&self, urls: &[String], variant: Variant) {
        let mut tasks = Vec::new();
        for u in urls {
            if check_url(u).is_err() {
                continue;
            }
            let h = self.register(u);
            if self.file_for(&h.0, variant).exists() {
                continue;
            }
            let this = self.clone();
            tasks.push(tokio::spawn(async move { this.ensure(&h.0, variant).await }));
        }
        for t in tasks {
            match t.await {
                Ok(Err(e)) => tracing::debug!(error = %e, "prefetch failed"),
                Err(e) => tracing::warn!(error = %e, "prefetch task failed"),
                Ok(Ok(_)) => {}
            }
        }
    }

    // Bytes for the protocol handler: memory, then disk, then a deduplicated fetch.
    pub async fn load(&self, hash: &str, variant: Variant) -> Result<Arc<Vec<u8>>, ImageError> {
        if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ImageError::Unknown(hash.to_string()));
        }
        let key = (hash.to_string(), variant);
        if let Some(b) = self.inner.mem.get(&key) {
            return Ok(b);
        }
        let path = self.file_for(hash, variant);
        if !path.exists() {
            if let Some(b) = self.ensure(hash, variant).await? {
                return Ok(b);
            }
        }
        let bytes = Arc::new(tokio::fs::read(&path).await?);
        touch(&path);
        if variant == Variant::Thumb {
            self.inner.mem.insert(key, Arc::clone(&bytes));
        }
        Ok(bytes)
    }

    // One variant on disk; concurrent callers share one fetch. Thumbs never wait for heroes.
    async fn ensure(&self, hash: &str, variant: Variant) -> Result<Option<Arc<Vec<u8>>>, ImageError> {
        if self.file_for(hash, variant).exists() {
            return Ok(None);
        }
        let url = lock(&self.inner.urls).get(hash).cloned().ok_or_else(|| ImageError::Unknown(hash.to_string()))?;
        let flight_key = format!("{hash}:{}", variant.suffix());
        let cell = {
            let mut m = lock(&self.inner.inflight);
            Arc::clone(m.entry(flight_key.clone()).or_insert_with(|| Arc::new(OnceCell::new())))
        };
        let res = cell.get_or_init(|| self.fetch_and_store(url.clone(), hash.to_string(), variant)).await.clone();
        lock(&self.inner.inflight).remove(&flight_key);
        match res {
            Ok(bytes) => {
                if self.inner.ready.send((url, ContentHash(hash.to_string()))).is_err() {
                    tracing::trace!("no cover-ready listeners");
                }
                Ok(Some(bytes))
            }
            Err(e) => Err(e),
        }
    }

    async fn download(&self, url: &str) -> Result<Vec<u8>, ImageError> {
        let resp = self.inner.http.get(url).send().await.map_err(|e| ImageError::Fetch(e.without_url().to_string()))?;
        if !resp.status().is_success() {
            return Err(ImageError::Fetch(format!("HTTP {}", resp.status().as_u16())));
        }
        if resp.content_length().is_some_and(|n| n as usize > MAX_DOWNLOAD) {
            return Err(ImageError::Fetch("image too large".into()));
        }
        let body = resp.bytes().await.map_err(|e| ImageError::Fetch(e.without_url().to_string()))?;
        if body.len() > MAX_DOWNLOAD {
            return Err(ImageError::Fetch("image too large".into()));
        }
        Ok(body.to_vec())
    }

    async fn fetch_and_store(&self, url: String, hash: String, variant: Variant) -> Result<Arc<Vec<u8>>, ImageError> {
        let _permit = self.inner.sem.acquire().await.map_err(|e| ImageError::Fetch(e.to_string()))?;
        // Ask the CDN for a pre-shrunk copy first: a ~30 KB thumb instead of a multi-MB original.
        let sized = sized_source(&url, variant);
        let body = match &sized {
            Some(s) => match self.download(s).await {
                Ok(b) => b,
                Err(e) => {
                    tracing::debug!(error = %e, "resized source failed; using original");
                    self.download(&url).await?
                }
            },
            None => self.download(&url).await?,
        };
        // Decode and encode are CPU-bound: keep them off the async workers.
        let bytes = tokio::task::spawn_blocking(move || transcode(&body, variant))
            .await
            .map_err(|e| ImageError::Decode(e.to_string()))??;
        write_atomic(&self.file_for(&hash, variant), &bytes)?;
        let bytes = Arc::new(bytes);
        if variant == Variant::Thumb {
            self.inner.mem.insert((hash, variant), Arc::clone(&bytes));
        }
        Ok(bytes)
    }

    // LRU eviction (by file mtime) down to the byte budget.
    pub async fn gc(&self) -> Result<GcReport, ImageError> {
        let root = self.inner.root.clone();
        let budget = self.inner.budget_bytes;
        tokio::task::spawn_blocking(move || gc_dir(&root, budget)).await.map_err(|e| ImageError::Io(e.to_string()))?
    }

    pub fn root(&self) -> &Utf8Path {
        &self.inner.root
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match m.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

fn check_url(url: &str) -> Result<(), ImageError> {
    let u = url.trim().to_ascii_lowercase();
    if u.starts_with("https://") || u.starts_with("http://") {
        Ok(())
    } else {
        Err(ImageError::BadUrl(url.to_string()))
    }
}

// Mark as recently used for LRU.
fn touch(path: &Utf8Path) {
    let res = std::fs::File::options().write(true).open(path).and_then(|f| f.set_modified(SystemTime::now()));
    if let Err(e) = res {
        tracing::trace!(%path, error = %e, "could not touch cached image");
    }
}

// RAWG serves resized copies at /media/resize/<w>/-/...; other hosts get the original.
pub fn sized_source(url: &str, variant: Variant) -> Option<String> {
    let marker = "media.rawg.io/media/";
    let at = url.find(marker)? + marker.len();
    let rest = &url[at..];
    if rest.starts_with("resize/") || rest.starts_with("crop/") {
        return None;
    }
    let w = match variant {
        Variant::Thumb => 420,
        Variant::Hero => 1280,
    };
    Some(format!("{}resize/{w}/-/{rest}", &url[..at]))
}

// Decode, shrink (never upscale) and encode one WebP variant; thumbs trade quality for speed.
fn transcode(bytes: &[u8], variant: Variant) -> Result<Vec<u8>, ImageError> {
    let mut reader =
        ImageReader::new(Cursor::new(bytes)).with_guessed_format().map_err(|e| ImageError::Decode(e.to_string()))?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(10_000);
    limits.max_image_height = Some(10_000);
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);
    let img = reader.decode().map_err(|e| ImageError::Decode(e.to_string()))?;
    let sized = resize_to_width(&img, variant.width());
    let rgba = DynamicImage::ImageRgba8(sized.to_rgba8());
    let enc = webp::Encoder::from_image(&rgba).map_err(|e| ImageError::Decode(e.to_string()))?;
    let quality = match variant {
        Variant::Thumb => THUMB_QUALITY,
        Variant::Hero => WEBP_QUALITY,
    };
    Ok(enc.encode(quality).to_vec())
}

fn resize_to_width(img: &DynamicImage, w: u32) -> DynamicImage {
    if img.width() <= w {
        return img.clone();
    }
    let h = ((u64::from(img.height()) * u64::from(w)) / u64::from(img.width().max(1))).max(1) as u32;
    img.resize_exact(w, h, image::imageops::FilterType::Triangle)
}

// Temp file then persist: a half-written cover is impossible (spec 10).
fn write_atomic(path: &Utf8Path, bytes: &[u8]) -> Result<(), ImageError> {
    let dir = path.parent().ok_or_else(|| ImageError::Io(format!("no parent for {path}")))?;
    std::fs::create_dir_all(dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(bytes)?;
    tmp.persist(path.as_std_path()).map_err(|e| ImageError::Io(e.error.to_string()))?;
    Ok(())
}

fn gc_dir(root: &Utf8Path, budget: u64) -> Result<GcReport, ImageError> {
    let mut files: Vec<(SystemTime, u64, std::path::PathBuf)> = Vec::new();
    for shard in std::fs::read_dir(root)? {
        let shard = shard?;
        if !shard.file_type()?.is_dir() {
            continue;
        }
        for f in std::fs::read_dir(shard.path())? {
            let f = f?;
            let md = f.metadata()?;
            let is_ours = f.file_name().to_str().is_some_and(|n| n.ends_with(".webp"));
            if md.is_file() && is_ours {
                files.push((md.modified().unwrap_or(SystemTime::UNIX_EPOCH), md.len(), f.path()));
            }
        }
    }
    let total: u64 = files.iter().map(|f| f.1).sum();
    let mut report = GcReport { files_before: files.len(), bytes_before: total, removed: 0, bytes_after: total };
    if total <= budget {
        return Ok(report);
    }
    // Oldest first; stop at 90% of budget so we do not churn every day.
    files.sort_by_key(|f| f.0);
    let target = budget / 10 * 9;
    for (_, size, path) in files {
        if report.bytes_after <= target {
            break;
        }
        std::fs::remove_file(&path)?;
        report.bytes_after = report.bytes_after.saturating_sub(size);
        report.removed += 1;
    }
    Ok(report)
}

#[cfg(test)]
mod tests;
