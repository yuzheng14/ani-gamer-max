use crate::config::AppConfig;
use crate::error::{CoreError, CoreResult};
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
