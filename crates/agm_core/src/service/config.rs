use std::path::{Path, PathBuf};

use tokio::fs;

use crate::entity::error::Result;
use crate::entity::{config::Config, error::AgmCoreError};

static DEFAULT_CONFIG_PATH: &str = "./data/config.toml";

pub struct ConfigService;

impl ConfigService {
    fn resolve_path<P: AsRef<Path>>(config_path: Option<P>) -> PathBuf {
        config_path
            .map(|p| p.as_ref().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH))
    }

    async fn ensure_parent_dir_exists(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        Ok(())
    }

    /// 写入配置文件，使用临时文件 + rename 保证原子写。
    pub async fn write_config<P: AsRef<Path>>(config: &Config, config_path: Option<P>) -> Result<()> {
        let path = Self::resolve_path(config_path);
        Self::ensure_parent_dir_exists(&path).await?;

        let serialized = toml::to_string_pretty(config)?;

        // Write to a sibling temp file then atomically rename.
        let tmp_path = path.with_extension("toml.tmp");
        fs::write(&tmp_path, &serialized).await?;
        fs::rename(&tmp_path, &path).await.map_err(|e| {
            AgmCoreError::Custom(format!(
                "配置文件原子写入失败 ({} -> {}): {}",
                tmp_path.display(),
                path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// 读取配置文件。配置文件不存在时，则创建默认配置文件。
    pub async fn read_config<P: AsRef<Path>>(config_path: Option<P>) -> Result<Config> {
        let path = Self::resolve_path(config_path);

        if !path.exists() || !path.is_file() {
            let config = Config::default();
            Self::write_config(&config, Some(&path)).await?;
            return Ok(config);
        }

        let raw = fs::read_to_string(&path).await?;
        Ok(toml::from_str(&raw)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn read_config_creates_default_when_file_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        assert!(!path.exists());

        let cfg = ConfigService::read_config(Some(path.as_path())).await.unwrap();

        assert_eq!(cfg.bangumi_dir, Config::default().bangumi_dir);
        assert!(path.is_file());
    }

    #[tokio::test]
    async fn write_config_and_read_config_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut cfg = Config::default();
        cfg.check_frequency = 42;
        cfg.bangumi_dir = "custom_bangumi".to_string();

        ConfigService::write_config(&cfg, Some(path.as_path())).await.unwrap();
        let loaded = ConfigService::read_config(Some(path.as_path())).await.unwrap();

        assert_eq!(loaded.check_frequency, 42);
        assert_eq!(loaded.bangumi_dir, "custom_bangumi");
    }

    #[tokio::test]
    async fn read_config_invalid_toml_returns_err() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "not valid toml [[[").await.unwrap();

        let err = ConfigService::read_config(Some(path.as_path())).await;
        assert!(err.is_err());
    }
}
