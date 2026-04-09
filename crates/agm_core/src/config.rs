use serde::{Deserialize, Serialize};

/// FTP 上传相关配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FtpConfig {
    /// FTP 服务器地址。
    pub server: String,
    /// 端口（配置里可为空字符串）。
    pub port: String,
    /// 用户名。
    pub user: String,
    /// 密码。
    pub pwd: String,
    /// 是否使用 FTP over TLS。
    pub tls: bool,
    /// 登录后首先进入的目录。
    pub cwd: String,
    /// 是否显示详细错误信息。
    pub show_error_detail: bool,
    /// 最大重传次数，支持断点续传。
    pub max_retry_num: u32,
}

/// 酷 Q 推送完成消息时的参数与 URL 列表。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoolqSettings {
    /// 请求体里消息字段的参数名。
    pub msg_argument_name: String,
    /// 追加在消息末尾的说明文字。
    pub message_suffix: String,
    /// 要请求的完整 URL 列表（含 query）。
    pub query: Vec<String>,
}

/// Web 控制台（Dashboard）监听与认证配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// 监听地址；需外网访问时可设为 `0.0.0.0`。
    pub host: String,
    /// 监听端口。
    pub port: u16,
    /// 是否启用 SSL（证书位于 Dashboard/sslkey）。
    #[serde(rename = "SSL")]
    pub ssl: bool,
    /// 是否启用 HTTP Basic 认证（密码明文传输，建议配合 SSL）。
    #[serde(rename = "BasicAuth")]
    pub basic_auth: bool,
    /// Basic 认证用户名。
    pub username: String,
    /// Basic 认证密码。
    pub password: String,
}

/// aniGamerPlus 主配置文件结构，对应 `config.json`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// 下载存放目录；动画按番剧分文件夹存放。
    pub bangumi_dir: String,
    /// 临时目录；v9.0 起下载中文件先放此处，完成后再移到番剧目录；留空则默认程序目录下 `temp`。
    pub temp_dir: String,
    /// 是否为每个番剧建立独立文件夹。
    pub classify_bangumi: bool,
    /// 是否在番剧目录下再建季度子目录。
    #[serde(default)]
    pub classify_season: bool,
    /// 检查更新频率，单位：分钟。
    pub check_frequency: u32,
    /// 两次下载之间的冷却时间，单位：秒。
    pub download_cd: u32,
    /// 解析 sn 播放页之间的冷却时间，单位：秒。
    pub parse_sn_cd: u32,
    /// 目标清晰度；不存在时选最接近的可用值。可选：360、480、540、576、720、1080。
    pub download_resolution: String,
    /// 为 true 时若指定清晰度不存在则放弃下载，不再降级。
    pub lock_resolution: bool,
    /// 是否仅使用 VIP 账号线路下载。
    pub only_use_vip: bool,
    /// 默认下载模式：`latest` 仅最后一集，`all` 全部集数，`largest-sn` 最近上传的一集。
    pub default_download_mode: String,
    /// 移到番剧目录时是否用“复制”而非移动（适合 rclone 等挂载盘）。
    pub use_copyfile_method: bool,
    /// 最大并发下载任务数（程序侧上限通常为 5，超出会被重置）。
    #[serde(rename = "multi-thread")]
    pub multi_thread: u32,
    /// 最大并发上传数。
    pub multi_upload: u32,
    /// 是否启用分段下载（更快、容错更好）。
    pub segment_download_mode: bool,
    /// 每个视频同时下载的分段数上限；仅 `segment_download_mode` 为 true 时有效（上限通常 5）。
    pub multi_downloading_segment: u32,
    /// 分段下载时每个分段的最大重试次数；`-1` 表示无限重试（若需支持负值可改为 `i32`）。
    pub segment_max_retry: i32,
    /// 文件名中是否包含番剧名；为 false 时通常只有剧集名，个位数会按 `zerofill` 补零。
    pub add_bangumi_name_to_video_filename: bool,
    /// 文件名中是否带清晰度，例如 `[1080P]`。
    pub add_resolution_to_video_filename: bool,
    /// 视频文件名前缀。
    pub customized_video_filename_prefix: String,
    /// 文件名里番剧名与剧集名之间的后缀片段。
    pub customized_bangumi_name_suffix: String,
    /// 视频文件名后缀。
    pub customized_video_filename_suffix: String,
    /// 输出容器扩展名；非 mp4 时 `faststart_movflags` 会被强制关闭。
    pub video_filename_extension: String,
    /// 剧集序号补零位数（如 2 → `01`，3 → `001`）。
    pub zerofill: u32,
    /// HTTP User-Agent，建议与获取 Cookie 的浏览器一致。
    pub ua: String,
    /// 是否使用代理。
    pub use_proxy: bool,
    /// 代理 URL，例如 `http://user:pass@host:port`。
    pub proxy: String,
    /// 为 true 时访问 Akamai 相关地址不走代理。
    pub no_proxy_akamai: bool,
    /// 是否在上传完成后把文件传到服务器（FTP）。
    pub upload_to_server: bool,
    /// FTP 详细配置。
    pub ftp: FtpConfig,
    /// 命令行模式 `-u`：全部任务结束后执行的系统命令。
    pub user_command: String,
    /// 是否在下载完成后向酷 Q 推送消息。
    pub coolq_notify: bool,
    pub coolq_settings: CoolqSettings,
    /// 是否为 mp4 写入 `faststart`（metadata 前置，利于在线秒开）。
    pub faststart_movflags: bool,
    /// 是否为音轨写入语言标签。
    pub audio_language: bool,
    /// 是否使用移动端 API 解析视频地址。
    pub use_mobile_api: bool,
    /// 是否下载弹幕（含站点内置关键词过滤）。
    pub danmu: bool,
    /// 额外弹幕过滤关键词（Python 正则语法，英文不区分大小写）。
    #[serde(default)]
    pub danmu_ban_words: Vec<String>,
    /// 是否适配 PLEX 命名规则。
    #[serde(default)]
    pub plex_naming: bool,
    /// 是否检查程序更新。
    pub check_latest_version: bool,
    /// 检查更新时是否重新读取 `sn_list.txt`（改列表后下次检查即可生效而无需重启）。
    pub read_sn_list_when_checking_update: bool,
    /// 检查更新时是否重新读取配置文件。
    pub read_config_when_checking_update: bool,
    /// 非 VIP 广告等待时间（秒）；不足时程序可能再追加等待（有上限）。
    pub ads_time: u32,
    /// 使用移动端 API 时的广告等待时间（秒）。
    pub mobile_ads_time: u32,
    /// 是否启用 Web 控制台。
    pub use_dashboard: bool,
    pub dashboard: DashboardConfig,
    /// 是否按天写入日志文件。
    pub save_logs: bool,
    /// 保留的日志文件份数，须 ≥ 1。
    pub quantity_of_logs: u32,
    /// 配置文件格式版本。
    pub config_version: f64,
    /// 本地数据库结构版本。
    pub database_version: f64,
}
