use serde::{Deserialize, Serialize};

use crate::entity::config::DownloadMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEntry {
    pub sn: u32,
    #[serde(default)]
    pub mode: DownloadMode,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub rename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SnList {
    #[serde(default)]
    pub watch: Vec<WatchEntry>,
}
