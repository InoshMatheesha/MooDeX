// tests.rs - Contract tests against a mock RAWG (spec 14): 429, 500, malformed, empty, cache.

use super::*;
use std::collections::HashMap;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Default)]
struct MemCache(Mutex<HashMap<String, CacheEntry>>);

impl HttpCache for MemCache {
    fn get(&self, key: String) -> BoxFut<'_, Option<CacheEntry>> {
        Box::pin(async move { self.0.lock().unwrap().get(&key).cloned() })
    }
    fn put(&self, key: String, entry: CacheEntry) -> BoxFut<'_, ()> {
        Box::pin(async move {
            self.0.lock().unwrap().insert(key, entry);
        })
    }
}

const PAGE: &str = r#"{"count":2,"next":"https://x/?page=2","results":[
  {"id":3328,"name":"The Witcher 3: Wild Hunt","background_image":"https://img/w3.jpg","released":"2015-05-18",
   "rating":4.66,"metacritic":92,"added":20000,"genres":[{"name":"Action"},{"name":"RPG"}]},
  {"id":1,"name":"No Extras"}]}"#;

async fn client_for(server: &MockServer) -> (RawgClient, Arc<MemCache>) {
    let cache = Arc::new(MemCache::default());
    let c = RawgClient::new(
        RawgConfig { api_base: server.uri(), ..RawgConfig::default() },
        cache.clone(),
        KeySource::Proxy { base_url: format!("{}/api/rawg", server.uri()) },
    )
    .unwrap();
    (c, cache)
}

#[tokio::test]
async fn parses_page_with_defaults() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/rawg/games"))
        .and(query_param("search", "witcher"))
        .and(query_param("page_size", "20"))
        .and(header("user-agent", format!("MooDeX/{}", env!("CARGO_PKG_VERSION")).as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_string(PAGE))
        .expect(1)
        .mount(&server)
        .await;
    let (c, _) = client_for(&server).await;
    let p = c.search("witcher", 1).await.unwrap();
    assert!(p.has_next);
    assert_eq!(p.results.len(), 2);
    assert_eq!(p.results[0].genres, vec!["Action".to_string(), "RPG".to_string()]);
    assert_eq!(p.results[1].background_image, None);
    // Second call is served from the fresh cache: the mock expects exactly one hit.
    let again = c.search("witcher", 1).await.unwrap();
    assert_eq!(again, p);
}

#[tokio::test]
async fn rate_limit_honours_retry_after() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "7"))
        .mount(&server)
        .await;
    let (c, _) = client_for(&server).await;
    assert_eq!(c.popular(1).await.unwrap_err(), RawgError::RateLimited(Duration::from_secs(7)));
}

#[tokio::test]
async fn upstream_error_is_typed() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(500)).mount(&server).await;
    let (c, _) = client_for(&server).await;
    assert_eq!(c.trending(1).await.unwrap_err(), RawgError::Upstream(500));
}

#[tokio::test]
async fn malformed_json_is_decode_error_and_not_cached() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{not json"))
        .mount(&server)
        .await;
    let (c, cache) = client_for(&server).await;
    assert!(matches!(c.upcoming(1).await.unwrap_err(), RawgError::Decode(_)));
    assert!(cache.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn empty_results_are_ok_and_distinct_from_errors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"count":0,"next":null,"results":[]}"#))
        .mount(&server)
        .await;
    let (c, _) = client_for(&server).await;
    let p = c.search("zzzz", 1).await.unwrap();
    assert!(p.results.is_empty() && !p.has_next);
}

#[tokio::test]
async fn no_key_is_typed_and_empty_query_needs_no_network() {
    let cache = Arc::new(MemCache::default());
    let c = RawgClient::new(RawgConfig::default(), cache, KeySource::None).unwrap();
    assert_eq!(c.popular(1).await.unwrap_err(), RawgError::NoKey);
    assert!(c.search("   ", 1).await.unwrap().results.is_empty());
}

#[tokio::test]
async fn offline_is_typed() {
    let cache = Arc::new(MemCache::default());
    let c = RawgClient::new(
        RawgConfig { timeout: Duration::from_secs(2), ..RawgConfig::default() },
        cache,
        KeySource::Proxy { base_url: "http://127.0.0.1:9".into() },
    )
    .unwrap();
    assert_eq!(c.popular(1).await.unwrap_err(), RawgError::Offline);
}

#[tokio::test]
async fn user_key_goes_to_query_not_cache_key() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/games"))
        .and(query_param("key", "secret-key"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PAGE))
        .mount(&server)
        .await;
    let cache = Arc::new(MemCache::default());
    let c = RawgClient::new(
        RawgConfig { api_base: server.uri(), ..RawgConfig::default() },
        cache.clone(),
        KeySource::User { key: "secret-key".into() },
    )
    .unwrap();
    c.popular(1).await.unwrap();
    let keys: Vec<String> = cache.0.lock().unwrap().keys().cloned().collect();
    assert_eq!(keys.len(), 1);
    assert!(!keys[0].contains("secret"), "{keys:?}");
}

// Stale entries are served instantly (works offline), then revalidated in the background.
#[tokio::test]
async fn stale_while_revalidate() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("if-none-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304))
        .expect(1)
        .mount(&server)
        .await;
    let (c, cache) = client_for(&server).await;
    let key = Endpoint::Popular { page: 1 }.cache_key(OffsetDateTime::now_utc().date());
    let past = OffsetDateTime::now_utc() - time::Duration::hours(10);
    cache.0.lock().unwrap().insert(
        key.clone(),
        CacheEntry { body: PAGE.as_bytes().to_vec(), etag: Some("\"v1\"".into()), fetched_at: past, expires_at: past },
    );
    let p = c.popular(1).await.unwrap();
    assert_eq!(p.results.len(), 2);
    for _ in 0..50 {
        if cache.0.lock().unwrap()[&key].expires_at > OffsetDateTime::now_utc() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(cache.0.lock().unwrap()[&key].expires_at > OffsetDateTime::now_utc(), "not revalidated");
}

#[tokio::test]
async fn details_parse() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/rawg/games/3328"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"id":3328,"name":"W3","description_raw":"Geralt.","developers":[{"name":"CD PROJEKT RED"}],
               "platforms":[{"platform":{"name":"PC"}}],"playtime":46}"#,
        ))
        .mount(&server)
        .await;
    let (c, _) = client_for(&server).await;
    let d = c.details(3328).await.unwrap();
    assert_eq!(d.description, "Geralt.");
    assert_eq!(d.developers, vec!["CD PROJEKT RED".to_string()]);
    assert_eq!(d.platforms, vec!["PC".to_string()]);
    assert_eq!(d.playtime_hours, 46);
}
