use std::{
    path::Path,
    time::SystemTime,
};

use tokio::fs;

use crate::entity::{cookie::Cookies, error::Result};
use crate::service::file_io;

static DEFAULT_COOKIE_PATH: &str = "./data/cookie.txt";

pub struct CookieService;

impl CookieService {
    /// 读取 cookie 文件。文件不存在或为空时返回空 Cookies。
    pub async fn read_cookies<P: AsRef<Path>>(path: Option<P>) -> Result<Cookies> {
        let path = file_io::resolve_path(path, DEFAULT_COOKIE_PATH);

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
        let path = file_io::resolve_path(path, DEFAULT_COOKIE_PATH);
        file_io::atomic_write(&path, cookies.as_str().as_bytes()).await
    }

    /// 将 cookie 文件重命名为 `invalid_cookie.txt`，避免继续使用失效的 cookie。
    pub async fn invalidate<P: AsRef<Path>>(path: Option<P>) -> Result<()> {
        let path = file_io::resolve_path(path, DEFAULT_COOKIE_PATH);

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
        let path = file_io::resolve_path(path, DEFAULT_COOKIE_PATH);
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
