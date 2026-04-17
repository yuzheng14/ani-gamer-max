use std::path::Path;

use tokio::fs;

use crate::entity::error::Result;
use crate::entity::sn_list::SnList;
use crate::service::file_io;

static DEFAULT_SN_LIST_PATH: &str = "./data/sn_list.toml";

pub struct SnListService;

impl SnListService {
    /// 写入追番清单，使用临时文件 + rename 保证原子写。
    pub async fn write_sn_list<P: AsRef<Path>>(sn_list: &SnList, path: Option<P>) -> Result<()> {
        let path = file_io::resolve_path(path, DEFAULT_SN_LIST_PATH);
        let serialized = toml::to_string_pretty(sn_list)?;
        file_io::atomic_write(&path, serialized.as_bytes()).await
    }

    /// 读取追番清单。文件不存在时，创建空清单文件。
    pub async fn read_sn_list<P: AsRef<Path>>(path: Option<P>) -> Result<SnList> {
        let path = file_io::resolve_path(path, DEFAULT_SN_LIST_PATH);

        if !path.exists() || !path.is_file() {
            let sn_list = SnList::default();
            Self::write_sn_list(&sn_list, Some(&path)).await?;
            return Ok(sn_list);
        }

        let raw = fs::read_to_string(&path).await?;
        Ok(toml::from_str(&raw)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{config::DownloadMode, sn_list::WatchEntry};
    use tempfile::tempdir;

    #[tokio::test]
    async fn read_sn_list_creates_empty_when_file_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sn_list.toml");
        assert!(!path.exists());

        let sn_list = SnListService::read_sn_list(Some(path.as_path())).await.unwrap();

        assert!(sn_list.watch.is_empty());
        assert!(path.is_file());
    }

    #[tokio::test]
    async fn write_and_read_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sn_list.toml");

        let sn_list = SnList {
            watch: vec![
                WatchEntry {
                    sn: 10147,
                    mode: DownloadMode::All,
                    tag: "2018十月番".to_string(),
                    rename: String::new(),
                },
                WatchEntry {
                    sn: 11285,
                    mode: DownloadMode::Latest,
                    tag: "2018十月番".to_string(),
                    rename: "史萊姆".to_string(),
                },
            ],
        };

        SnListService::write_sn_list(&sn_list, Some(path.as_path())).await.unwrap();
        let loaded = SnListService::read_sn_list(Some(path.as_path())).await.unwrap();

        assert_eq!(loaded.watch.len(), 2);
        assert_eq!(loaded.watch[0].sn, 10147);
        assert_eq!(loaded.watch[1].rename, "史萊姆");
    }

    #[tokio::test]
    async fn defaults_applied_when_fields_omitted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sn_list.toml");

        // Write TOML with only the required `sn` field
        fs::write(&path, "[[watch]]\nsn = 99999\n").await.unwrap();

        let loaded = SnListService::read_sn_list(Some(path.as_path())).await.unwrap();
        let entry = &loaded.watch[0];

        assert_eq!(entry.sn, 99999);
        assert!(matches!(entry.mode, DownloadMode::Latest));
        assert!(entry.tag.is_empty());
        assert!(entry.rename.is_empty());
    }

    #[tokio::test]
    async fn largest_sn_mode_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sn_list.toml");

        let sn_list = SnList {
            watch: vec![WatchEntry {
                sn: 12345,
                mode: DownloadMode::LargestSn,
                tag: String::new(),
                rename: String::new(),
            }],
        };

        SnListService::write_sn_list(&sn_list, Some(path.as_path())).await.unwrap();
        let loaded = SnListService::read_sn_list(Some(path.as_path())).await.unwrap();

        assert!(matches!(loaded.watch[0].mode, DownloadMode::LargestSn));

        // Verify the on-disk representation uses "largest-sn"
        let raw = fs::read_to_string(&path).await.unwrap();
        assert!(raw.contains("largest-sn"));
    }

    #[tokio::test]
    async fn read_invalid_toml_returns_err() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sn_list.toml");
        fs::write(&path, "not valid toml [[[").await.unwrap();

        let err = SnListService::read_sn_list(Some(path.as_path())).await;
        assert!(err.is_err());
    }
}
