// cache_bridge.rs - RAWG's HTTP cache routed through the actor, so SQLite keeps a single owner.

use crate::LibraryHandle;
use moodex_rawg::{CacheEntry, HttpCache};
use std::future::Future;
use std::pin::Pin;

pub struct ActorCache(pub LibraryHandle);

impl HttpCache for ActorCache {
    fn get(&self, key: String) -> Pin<Box<dyn Future<Output = Option<CacheEntry>> + Send + '_>> {
        Box::pin(async move { self.0.cache_get(key).await })
    }

    fn put(&self, key: String, entry: CacheEntry) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        self.0.cache_put(key, entry);
        Box::pin(async {})
    }
}
