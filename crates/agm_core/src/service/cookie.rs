use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use tokio::fs;

use crate::entity::{cookie::Cookies, error::Result};

static DEFAULT_COOKIE_PATH: &str = "./data/cookie.txt";

pub struct CookieService;

impl CookieService {
    fn resolve_path<P: AsRef<Path>>(path: Option<P>) -> PathBuf {
        path.map(|p| p.as_ref().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(DEFAULT_COOKIE_PATH))
    }

    async fn ensure_parent_dir_exists(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        Ok(())
    }

    /// 读取 cookie 文件。文件不存在或为空时返回空 Cookies。
    pub async fn read_cookies<P: AsRef<Path>>(path: Option<P>) -> Result<Cookies> {
        let path = Self::resolve_path(path);

        if !path.exists() || !path.is_file() {
            return Ok(Cookies::default());
        }

        let raw = fs::read_to_string(&path).await?;
        // Read only the first non-blank line, matching the original behaviour
        let line = raw.lines().find(|l| !l.trim().is_empty());
        Ok(line.map(|l| Cookies::from(l.to_string())).unwrap_or_default())
    }

    /// 写入 cookie 文件，使用临时文件 + rename 保证原子写。
    pub async fn write_cookies<P: AsRef<Path>>(cookies: &Cookies, path: Option<P>) -> Result<()> {
        let path = Self::resolve_path(path);
        Self::ensure_parent_dir_exists(&path).await?;

        let tmp_path = path.with_extension("txt.tmp");
        fs::write(&tmp_path, cookies.as_str()).await?;
        fs::rename(&tmp_path, &path).await?;

        Ok(())
    }

    /// 将 cookie 文件重命名为 `invalid_cookie.txt`，避免继续使用失效的 cookie。
    pub async fn invalidate<P: AsRef<Path>>(path: Option<P>) -> Result<()> {
        let path = Self::resolve_path(path);

        if !path.exists() {
            return Ok(());
        }

        let invalid_path = path.with_file_name("invalid_cookie.txt");
        if invalid_path.exists() {
            fs::remove_file(&invalid_path).await?;
        }
        fs::rename(&path, &invalid_path).await?;

        Ok(())
    }

    /// 返回 cookie 文件的最后修改时间。
    pub async fn last_modified<P: AsRef<Path>>(path: Option<P>) -> Result<SystemTime> {
        let path = Self::resolve_path(path);
        let metadata = fs::metadata(&path).await?;
        Ok(metadata.modified()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn read_returns_empty_when_file_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cookie.txt");

        let cookies = CookieService::read_cookies(Some(path.as_path())).await.unwrap();
        assert!(cookies.is_empty());
    }

    #[tokio::test]
    async fn read_returns_empty_when_file_is_blank() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cookie.txt");
        fs::write(&path, "   \n\n").await.unwrap();

        let cookies = CookieService::read_cookies(Some(path.as_path())).await.unwrap();
        assert!(cookies.is_empty());
    }

    #[tokio::test]
    async fn write_and_read_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cookie.txt");
        let cookies = Cookies::from("BAHAID=abc123; BAHARUNE=xyz".to_string());

        CookieService::write_cookies(&cookies, Some(path.as_path())).await.unwrap();
        let loaded = CookieService::read_cookies(Some(path.as_path())).await.unwrap();

        assert_eq!(loaded.as_str(), "BAHAID=abc123; BAHARUNE=xyz");
    }

    #[tokio::test]
    async fn read_uses_only_first_non_blank_line() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cookie.txt");
        fs::write(&path, "BAHAID=abc\nBAHARUNE=xyz\n").await.unwrap();

        let cookies = CookieService::read_cookies(Some(path.as_path())).await.unwrap();
        assert_eq!(cookies.as_str(), "BAHAID=abc");
    }

    #[tokio::test]
    async fn invalidate_renames_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cookie.txt");
        fs::write(&path, "BAHAID=abc").await.unwrap();

        CookieService::invalidate(Some(path.as_path())).await.unwrap();

        assert!(!path.exists());
        assert!(dir.path().join("invalid_cookie.txt").exists());
    }

    #[tokio::test]
    async fn invalidate_replaces_existing_invalid_cookie() {
        let dir = tempdir().unwrap();
        let cookie_path = dir.path().join("cookie.txt");
        let invalid_path = dir.path().join("invalid_cookie.txt");
        fs::write(&cookie_path, "BAHAID=new").await.unwrap();
        fs::write(&invalid_path, "BAHAID=old").await.unwrap();

        CookieService::invalidate(Some(cookie_path.as_path())).await.unwrap();

        let content = fs::read_to_string(&invalid_path).await.unwrap();
        assert_eq!(content, "BAHAID=new");
    }

    #[tokio::test]
    async fn last_modified_returns_metadata() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cookie.txt");
        fs::write(&path, "BAHAID=abc").await.unwrap();

        let t = CookieService::last_modified(Some(path.as_path())).await.unwrap();
        assert!(t.elapsed().unwrap().as_secs() < 5);
    }
}
