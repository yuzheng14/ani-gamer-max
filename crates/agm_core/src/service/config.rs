use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::entity::error::Result;
use crate::entity::{config::Config, error::AgmCoreError};

static DEFAULT_CONFIG_PATH: &str = "./data/config.toml";

pub struct ConfigService;

impl ConfigService {
    /// 统一解析配置文件路径，返回 PathBuf。
    /// 未传入时使用默认值 ./data/config.toml。
    fn resolve_path<P: AsRef<Path>>(config_path: Option<P>) -> PathBuf {
        config_path
            .map(|p| p.as_ref().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH))
    }

    /// 确保配置文件存在。如果不存在，则创建父目录与空文件。
    fn ensure_file_exists(path: &PathBuf) -> Result<()> {
        if !path.exists() || !path.is_file() {
            if let Some(parent) = path.parent() {
                // TODO async
                fs::create_dir_all(parent)?;
                fs::File::create(&path)?;
            } else {
                return Err(AgmCoreError::Custom(format!(
                    "无法创建配置文件，因为无法找到父路径: {}",
                    path.display()
                )));
            }
        }
        Ok(())
    }

    /// 写入配置文件。
    pub fn write_config<P: AsRef<Path>>(config: &Config, config_path: Option<P>) -> Result<()> {
        let path = Self::resolve_path(config_path);
        Self::ensure_file_exists(&path)?;
        // TODO async
        fs::write(path, toml::to_string_pretty(config)?)?;
        Ok(())
    }

    /// 读取配置文件。配置文件不存在时，则创建默认配置文件。
    pub fn read_config<P: AsRef<Path>>(config_path: Option<P>) -> Result<Config> {
        let path = Self::resolve_path(config_path);

        // 配置文件不存在，则创建默认配置文件。
        if !path.exists() || !path.is_file() {
            Self::ensure_file_exists(&path)?;
            let config = Config::default();
            Self::write_config(&config, Some(path))?;
            return Ok(config);
        }

        // TODO async
        // 配置文件存在，则读取配置文件。
        let raw = fs::read_to_string(path)?;
        Ok(toml::from_str(&raw)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn read_config_creates_default_when_file_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        assert!(!path.exists());

        let cfg = ConfigService::read_config(Some(path.as_path())).unwrap();

        assert_eq!(cfg.bangumi_dir, Config::default().bangumi_dir);
        assert!(path.is_file());
    }

    #[test]
    fn write_config_and_read_config_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut cfg = Config::default();
        cfg.check_frequency = 42;
        cfg.bangumi_dir = "custom_bangumi".to_string();

        ConfigService::write_config(&cfg, Some(path.as_path())).unwrap();
        let loaded = ConfigService::read_config(Some(path.as_path())).unwrap();

        assert_eq!(loaded.check_frequency, 42);
        assert_eq!(loaded.bangumi_dir, "custom_bangumi");
    }

    #[test]
    fn read_config_invalid_toml_returns_err() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "not valid toml [[[").unwrap();

        let err = ConfigService::read_config(Some(path.as_path()));
        assert!(err.is_err());
    }
}
