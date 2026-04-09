use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::error::{CoreError, CoreResult};
use crate::fs_atomic::write_file_atomic;
use crate::handlers;

/// Application settings persisted as `config.toml` (replaces legacy `config.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// UI / CLI locale: `zh-Hans`, `zh-Hant`, or `en`.
    pub locale: String,

    pub bangumi_dir: String,
    pub temp_dir: String,
    pub classify_bangumi: bool,
    pub classify_season: bool,
    pub check_frequency: u32,
    pub download_cd: u32,
    pub parse_sn_cd: u32,
    pub download_resolution: String,
    pub lock_resolution: bool,
    pub only_use_vip: bool,
    /// `latest` or `all` (legacy also had episode modes per sn_list entry).
    pub default_download_mode: String,
    pub use_copyfile_method: bool,
    #[serde(rename = "multi-thread")]
    pub multi_thread: u32,
    pub multi_upload: u32,
    pub segment_download_mode: bool,
    pub multi_downloading_segment: u32,
    pub segment_max_retry: i32,
    pub add_bangumi_name_to_video_filename: bool,
    pub add_resolution_to_video_filename: bool,
    pub customized_video_filename_prefix: String,
    pub customized_bangumi_name_suffix: String,
    pub customized_video_filename_suffix: String,
    pub video_filename_extension: String,
    pub zerofill: u32,
    pub ua: String,
    pub use_proxy: bool,
    pub proxy: String,
    pub no_proxy_akamai: bool,
    pub upload_to_server: bool,
    pub ftp: FtpConfig,
    pub user_command: String,
    pub coolq_notify: bool,
    pub coolq_settings: CoolqSettings,
    pub telebot_notify: bool,
    pub telebot_token: String,
    pub telebot_use_chat_id: bool,
    pub telebot_chat_id: String,
    pub discord_notify: bool,
    pub discord_token: String,
    pub plex_refresh: bool,
    pub plex_url: String,
    pub plex_token: String,
    pub plex_section: String,
    pub plex_naming: bool,
    pub faststart_movflags: bool,
    pub audio_language: bool,
    pub use_mobile_api: bool,
    pub danmu: bool,
    pub danmu_ban_words: Vec<String>,
    pub check_latest_version: bool,
    pub read_sn_list_when_checking_update: bool,
    pub read_config_when_checking_update: bool,
    pub ads_time: u32,
    pub mobile_ads_time: u32,
    pub use_dashboard: bool,
    pub dashboard: DashboardConfig,
    pub save_logs: bool,
    pub quantity_of_logs: u32,
    pub config_version: f64,
    pub database_version: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FtpConfig {
    pub server: String,
    pub port: String,
    pub user: String,
    pub pwd: String,
    pub tls: bool,
    pub cwd: String,
    pub show_error_detail: bool,
    pub max_retry_num: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CoolqSettings {
    pub msg_argument_name: String,
    pub message_suffix: String,
    pub query: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DashboardConfig {
    pub host: String,
    pub port: u16,
    #[serde(rename = "SSL")]
    pub ssl: bool,
    #[serde(rename = "BasicAuth")]
    pub basic_auth: bool,
    pub username: String,
    pub password: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            locale: "zh-Hans".to_string(),
            bangumi_dir: String::new(),
            temp_dir: String::new(),
            classify_bangumi: true,
            classify_season: false,
            check_frequency: 5,
            download_cd: 60,
            parse_sn_cd: 5,
            download_resolution: "1080".to_string(),
            lock_resolution: false,
            only_use_vip: false,
            default_download_mode: "latest".to_string(),
            use_copyfile_method: false,
            multi_thread: 1,
            multi_upload: 3,
            segment_download_mode: true,
            multi_downloading_segment: 2,
            segment_max_retry: 8,
            add_bangumi_name_to_video_filename: true,
            add_resolution_to_video_filename: true,
            customized_video_filename_prefix: "【動畫瘋】".to_string(),
            customized_bangumi_name_suffix: String::new(),
            customized_video_filename_suffix: String::new(),
            video_filename_extension: "mp4".to_string(),
            zerofill: 1,
            ua: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36".to_string(),
            use_proxy: false,
            proxy: "http://user:passwd@example.com:1000".to_string(),
            no_proxy_akamai: false,
            upload_to_server: false,
            ftp: FtpConfig::default(),
            user_command: "shutdown -s -t 60".to_string(),
            coolq_notify: false,
            coolq_settings: CoolqSettings::default(),
            telebot_notify: false,
            telebot_token: String::new(),
            telebot_use_chat_id: false,
            telebot_chat_id: String::new(),
            discord_notify: false,
            discord_token: String::new(),
            plex_refresh: false,
            plex_url: String::new(),
            plex_token: String::new(),
            plex_section: String::new(),
            plex_naming: false,
            faststart_movflags: false,
            audio_language: false,
            use_mobile_api: false,
            danmu: false,
            danmu_ban_words: Vec::new(),
            check_latest_version: true,
            read_sn_list_when_checking_update: true,
            read_config_when_checking_update: true,
            ads_time: 25,
            mobile_ads_time: 25,
            use_dashboard: true,
            dashboard: DashboardConfig::default(),
            save_logs: true,
            quantity_of_logs: 7,
            config_version: 17.2,
            database_version: 2.0,
        }
    }
}

impl Default for FtpConfig {
    fn default() -> Self {
        Self {
            server: String::new(),
            port: String::new(),
            user: String::new(),
            pwd: String::new(),
            tls: true,
            cwd: String::new(),
            show_error_detail: false,
            max_retry_num: 15,
        }
    }
}

impl Default for CoolqSettings {
    fn default() -> Self {
        Self {
            msg_argument_name: "message".to_string(),
            message_suffix: "追加的資訊".to_string(),
            query: Vec::new(),
        }
    }
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 5000,
            ssl: false,
            basic_auth: false,
            username: "admin".to_string(),
            password: "admin".to_string(),
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), CoreError> {
        handlers::validate_locale(&self.locale)?;
        Ok(())
    }

    pub async fn load_from_path(path: impl AsRef<Path>) -> CoreResult<Self> {
        let path = path.as_ref();
        if !path.exists() {
            let cfg = AppConfig::default();
            cfg.validate()?;
            return Ok(cfg);
        }
        let text = fs::read_to_string(path).await?;
        let cfg: AppConfig = toml::from_str(&text)?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub async fn save_to_path(&self, path: impl AsRef<Path>) -> CoreResult<()> {
        self.validate()?;
        let text = toml::to_string_pretty(self)?;
        write_file_atomic(path, text).await?;
        Ok(())
    }
}
