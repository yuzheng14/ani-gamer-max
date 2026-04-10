use crate::config::AppConfig;
use crate::error::{CoreError, CoreResult};
use crate::site::{EpisodeMetadata, GamerSiteClient};
use crate::watch_list::SnListFile;

pub const SUPPORTED_LOCALES: &[&str] = &["zh-Hans", "zh-Hant", "en"];

pub fn validate_locale(locale: &str) -> CoreResult<()> {
    if SUPPORTED_LOCALES.contains(&locale) {
        Ok(())
    } else {
        Err(CoreError::InvalidLocale(locale.to_string()))
    }
}

/// Shared handler: load and validate `config.toml`.
pub async fn load_config(path: impl AsRef<std::path::Path>) -> CoreResult<AppConfig> {
    AppConfig::load_from_path(path).await
}

/// Shared handler: load and validate `sn_list.toml`.
pub async fn load_watch_list(path: impl AsRef<std::path::Path>) -> CoreResult<SnListFile> {
    SnListFile::load_from_path(path).await
}

/// Fetches one episode page metadata (HTML or mobile API per `cfg.use_mobile_api`).
pub async fn fetch_episode_metadata(
    cfg: &AppConfig,
    sn: u32,
    cookie_header: Option<&str>,
) -> CoreResult<EpisodeMetadata> {
    let client = GamerSiteClient::from_config(cfg)?;
    client
        .fetch_episode_metadata(sn, cfg.use_mobile_api, cookie_header)
        .await
}
