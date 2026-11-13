//! moodex-rawg - RAWG client: key never in the binary, cached, bounded (spec 9).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))] // tests may unwrap (spec 4.2)

mod client;
mod error;
mod wire;

pub use client::{CacheEntry, Endpoint, HttpCache, KeySource, RawgClient, RawgConfig};
pub use error::RawgError;
pub use wire::{Page, RawgGameDetail};
