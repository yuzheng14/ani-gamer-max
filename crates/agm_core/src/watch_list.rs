use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::error::{CoreError, CoreResult};
use crate::fs_atomic::write_file_atomic;

/// Per-entry download behavior (matches legacy `sn_list.txt` modes where applicable).
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadMode {
    All,
    Latest,
    #[serde(rename = "largest-sn")]
    LargestSn,
    /// Use the global `default_download_mode` from config when resolving at download time.
    #[default]
    Inherit,
}

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

/// Root document for `sn_list.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnListFile {
    #[serde(default)]
    pub watch: Vec<WatchEntry>,
}

impl SnListFile {
    pub fn sn_index(&self) -> HashMap<u32, usize> {
        let mut m = HashMap::with_capacity(self.watch.len());
        for (i, w) in self.watch.iter().enumerate() {
            m.insert(w.sn, i);
        }
        m
    }

    pub async fn load_from_path(path: impl AsRef<Path>) -> CoreResult<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(SnListFile::default());
        }
        let text = fs::read_to_string(path).await?;
        toml::from_str(&text).map_err(CoreError::SnListDeserialize)
    }

    pub async fn save_to_path(&self, path: impl AsRef<Path>) -> CoreResult<()> {
        let text = toml::to_string_pretty(self)?;
        write_file_atomic(path, text).await?;
        Ok(())
    }
}

/// Import legacy `sn_list.txt` content into [`SnListFile`].
///
/// `default_mode` should be the config value `default_download_mode` (`latest` / `all` / …).
pub fn from_legacy_sn_list_txt(text: &str, default_mode: &str) -> SnListFile {
    let default = parse_download_mode_token(default_mode).unwrap_or(DownloadMode::Latest);

    let mut tag = String::new();
    let mut watch = Vec::new();

    for raw in text.lines() {
        let line = raw.trim_end();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix('@') {
            let t = rest.trim();
            if t.is_empty() {
                tag.clear();
            } else {
                tag = t.to_string();
            }
            continue;
        }

        let no_comment = line.split('#').next().unwrap_or("").trim();
        if no_comment.is_empty() {
            continue;
        }

        let collapsed = collapse_ascii_spaces(no_comment);
        let parts: Vec<&str> = collapsed.split(' ').filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            continue;
        }

        let first = parts[0];
        if !first.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let Ok(sn) = first.parse::<u32>() else {
            continue;
        };

        let mode = match parts.get(1).copied() {
            None => default,
            Some("all") => DownloadMode::All,
            Some("latest") => DownloadMode::Latest,
            Some("largest-sn") => DownloadMode::LargestSn,
            // Common typo in user files; treat like `latest`.
            Some("lastest") => DownloadMode::Latest,
            Some(_) => default,
        };

        let rename = extract_angle_rename(no_comment);

        watch.push(WatchEntry {
            sn,
            mode,
            tag: tag.trim_end().to_string(),
            rename,
        });
    }

    SnListFile { watch }
}

fn parse_download_mode_token(s: &str) -> Option<DownloadMode> {
    match s.trim().to_lowercase().as_str() {
        "all" => Some(DownloadMode::All),
        "latest" => Some(DownloadMode::Latest),
        "largest-sn" => Some(DownloadMode::LargestSn),
        _ => None,
    }
}

fn collapse_ascii_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch == ' ' {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            prev_space = false;
            out.push(ch);
        }
    }
    out.trim().to_string()
}

fn extract_angle_rename(line: &str) -> String {
    let Some((_, after)) = line.split_once('<') else {
        return String::new();
    };
    let Some((name, _)) = after.split_once('>') else {
        return String::new();
    };
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_sample_import() {
        let sample = include_str!("../../../original/sn_list-sample.txt");
        let list = from_legacy_sn_list_txt(sample, "latest");
        assert!(!list.watch.is_empty());
        let sn10147 = list.watch.iter().find(|w| w.sn == 10147).expect("10147");
        assert_eq!(sn10147.mode, DownloadMode::All);
        // `@` line comes after this entry in the sample; tag applies to following rows.
        assert!(sn10147.tag.is_empty());

        let sn11285 = list.watch.iter().find(|w| w.sn == 11285).expect("11285");
        assert_eq!(sn11285.tag, "2018十月番");
        assert_eq!(sn11285.rename, "史萊姆");

        let sn11390 = list.watch.iter().find(|w| w.sn == 11390).expect("11390");
        assert_eq!(sn11390.mode, DownloadMode::All);

        let sn11317 = list.watch.iter().find(|w| w.sn == 11317).expect("11317");
        assert_eq!(sn11317.mode, DownloadMode::Latest);
    }
}
