// tests.rs - Image pipeline: fetch once, two variants, memory tier, GC budget.

use super::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbImage::from_fn(w, h, |x, y| image::Rgb([(x % 255) as u8, (y % 255) as u8, 90]));
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img).write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

fn tmp_root() -> (tempfile::TempDir, Utf8PathBuf) {
    let d = tempfile::tempdir().unwrap();
    let p = Utf8PathBuf::from_path_buf(d.path().join("covers")).unwrap();
    (d, p)
}

#[test]
fn hash_is_stable_sha256() {
    let h = hash_url("https://media.rawg.io/a.jpg");
    assert_eq!(h.0.len(), 64);
    assert_eq!(h, hash_url("  https://media.rawg.io/a.jpg "));
    assert_ne!(h, hash_url("https://media.rawg.io/b.jpg"));
}

#[tokio::test]
async fn fetches_once_and_serves_variants() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/cover.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(png(1600, 900)))
        .expect(2)
        .mount(&server)
        .await;
    let (_d, root) = tmp_root();
    let cache = ImageCache::new(root, 50 * 1024 * 1024).unwrap();
    let url = format!("{}/cover.png", server.uri());
    let h = cache.register(&url);

    // One download per variant; repeated loads hit memory/disk.
    let (a, b) = tokio::join!(cache.load(&h.0, Variant::Thumb), cache.load(&h.0, Variant::Hero));
    let thumb = a.unwrap();
    let hero = b.unwrap();
    let t = image::load_from_memory(&thumb).unwrap();
    let hh = image::load_from_memory(&hero).unwrap();
    assert_eq!(t.width(), 400);
    assert_eq!(hh.width(), 1280);
    assert_eq!(&thumb[..4], b"RIFF");

    // Second round: no network (the mock allows exactly two hits).
    cache.load(&h.0, Variant::Thumb).await.unwrap();
    cache.load(&h.0, Variant::Hero).await.unwrap();
    assert_eq!(cache.get(&url, Variant::Thumb).await.unwrap(), CacheHit::Ready(h.clone()));
    assert!(cache.is_cached(&h));
}

#[tokio::test]
async fn small_images_are_not_upscaled() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(png(200, 100)))
        .mount(&server)
        .await;
    let (_d, root) = tmp_root();
    let cache = ImageCache::new(root, 50 * 1024 * 1024).unwrap();
    let h = cache.register(&format!("{}/s.png", server.uri()));
    let hero = cache.load(&h.0, Variant::Hero).await.unwrap();
    assert_eq!(image::load_from_memory(&hero).unwrap().width(), 200);
}

#[tokio::test]
async fn pending_then_ready_event() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(png(400, 300)))
        .mount(&server)
        .await;
    let (_d, root) = tmp_root();
    let cache = ImageCache::new(root, 50 * 1024 * 1024).unwrap();
    let mut ready = cache.subscribe_ready();
    let url = format!("{}/p.png", server.uri());
    assert_eq!(cache.get(&url, Variant::Thumb).await.unwrap(), CacheHit::Pending);
    let (u, h) = tokio::time::timeout(Duration::from_secs(10), ready.recv()).await.unwrap().unwrap();
    assert_eq!(u, url);
    assert_eq!(h, hash_url(&url));
}

#[tokio::test]
async fn bad_inputs_are_typed_errors() {
    let (_d, root) = tmp_root();
    let cache = ImageCache::new(root, 1024).unwrap();
    assert!(matches!(cache.get("file:///c:/x.png", Variant::Thumb).await, Err(ImageError::BadUrl(_))));
    assert!(matches!(cache.load("../../etc", Variant::Thumb).await, Err(ImageError::Unknown(_))));
    assert!(matches!(cache.load(&"a".repeat(64), Variant::Thumb).await, Err(ImageError::Unknown(_))));
}

#[tokio::test]
async fn undecodable_body_is_decode_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200).set_body_string("<html>")).mount(&server).await;
    let (_d, root) = tmp_root();
    let cache = ImageCache::new(root, 1024 * 1024).unwrap();
    let h = cache.register(&format!("{}/x", server.uri()));
    assert!(matches!(cache.load(&h.0, Variant::Thumb).await, Err(ImageError::Decode(_))));
}

#[tokio::test]
async fn gc_evicts_to_budget() {
    let (_d, root) = tmp_root();
    let cache = ImageCache::new(root.clone(), 10_000).unwrap();
    for i in 0..10 {
        let p = root.join("ab").join(format!("{:064}-thumb.webp", i));
        write_atomic(&p, &vec![0u8; 2_000]).unwrap();
    }
    std::fs::write(root.join("ab").join("readme.txt"), b"not ours").unwrap();
    let r = cache.gc().await.unwrap();
    assert_eq!(r.bytes_before, 20_000);
    assert!(r.bytes_after <= 9_000, "{r:?}");
    assert!(root.join("ab").join("readme.txt").exists());
}

#[test]
fn rawg_urls_use_cdn_resizing() {
    let u = "https://media.rawg.io/media/games/86f/86f2.jpg";
    assert_eq!(
        sized_source(u, Variant::Thumb).as_deref(),
        Some("https://media.rawg.io/media/resize/420/-/games/86f/86f2.jpg")
    );
    assert_eq!(
        sized_source(u, Variant::Hero).as_deref(),
        Some("https://media.rawg.io/media/resize/1280/-/games/86f/86f2.jpg")
    );
    assert_eq!(sized_source("https://media.rawg.io/media/resize/640/-/games/a.jpg", Variant::Thumb), None);
    assert_eq!(sized_source("https://example.com/a.jpg", Variant::Thumb), None);
}
