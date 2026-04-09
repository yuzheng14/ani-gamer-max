//! Core library for the aniGamerPlus Rust rewrite: shared DTOs, errors, config, and handlers.
//!
//! Boundary crates (`agm_cli`, `agm_server`, `agm_desktop`) should stay thin and call into here.

pub mod config;
pub mod cookie;
pub mod error;
pub mod fs_atomic;
pub mod handlers;
pub mod watch_list;

pub use config::AppConfig;
pub use error::{ApiError, ApiErrorCode, CoreError, CoreResult};
pub use watch_list::{DownloadMode, SnListFile, WatchEntry};
