//! moodex-core - domain types and pure logic; performs no I/O (spec 4.1).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))] // tests may unwrap (spec 4.2)

pub mod error;
pub mod matching;
pub mod model;
pub mod paths;
pub mod playtime;
pub mod query;
pub mod scan;
pub mod session;
pub mod stats;
pub mod text;

pub use error::CoreError;
pub use model::*;
