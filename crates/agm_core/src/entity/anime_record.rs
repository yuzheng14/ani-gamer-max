/// Full record as stored in the database.
#[derive(Debug, Clone)]
pub struct AnimeRecord {
    pub sn: u32,
    pub title: String,
    pub anime_name: String,
    pub episode: String,
    /// `true` = downloaded successfully.
    pub downloaded: bool,
    /// `true` = uploaded to remote server.
    pub uploaded: bool,
    /// Vertical resolution in pixels (e.g. 1080).
    pub resolution: u32,
    /// File size in MB.
    pub file_size: u32,
    pub local_file_path: Option<String>,
}

/// Data needed to create a new entry before a download starts.
#[derive(Debug, Clone)]
pub struct NewAnimeRecord {
    pub sn: u32,
    pub title: String,
    pub anime_name: String,
    pub episode: String,
}

/// Fields written back after a download (and optional upload) completes.
#[derive(Debug, Clone)]
pub struct AnimeDownloadResult {
    pub sn: u32,
    /// `true` if the file size is above the valid threshold (> 5 MB).
    pub downloaded: bool,
    /// `true` if the upload to the remote server succeeded.
    pub uploaded: bool,
    pub resolution: u32,
    pub file_size: u32,
    pub local_file_path: Option<String>,
}
