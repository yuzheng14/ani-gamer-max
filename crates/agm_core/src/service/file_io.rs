use std::path::{Path, PathBuf};

use tokio::fs;

use crate::entity::error::{AgmCoreError, Result};

/// Returns `path` as a `PathBuf`, falling back to `default` when `None`.
pub(crate) fn resolve_path<P: AsRef<Path>>(path: Option<P>, default: &str) -> PathBuf {
    path.map(|p| p.as_ref().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(default))
}

/// Writes `contents` to `path` atomically via a sibling `.tmp` file and `rename`.
/// Creates any missing parent directories before writing.
pub(crate) async fn atomic_write(path: &Path, contents: impl AsRef<[u8]>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    // Write to a sibling temp file then rename so readers never see a partial file.
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, contents).await?;
    fs::rename(&tmp_path, path).await.map_err(|e| {
        AgmCoreError::Custom(format!(
            "原子写入失败 ({} -> {}): {}",
            tmp_path.display(),
            path.display(),
            e
        ))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn atomic_write_creates_file_and_cleans_up_tmp() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("out.txt");

        atomic_write(&path, b"hello").await.unwrap();

        assert_eq!(fs::read_to_string(&path).await.unwrap(), "hello");
        assert!(!dir.path().join("out.tmp").exists());
    }

    #[tokio::test]
    async fn atomic_write_creates_missing_parent_dirs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a/b/c/out.txt");

        atomic_write(&path, b"data").await.unwrap();

        assert_eq!(fs::read_to_string(&path).await.unwrap(), "data");
    }

    #[test]
    fn resolve_path_uses_provided_path() {
        let result = resolve_path(Some("/custom/path.txt"), "./default.txt");
        assert_eq!(result, PathBuf::from("/custom/path.txt"));
    }

    #[test]
    fn resolve_path_falls_back_to_default() {
        let result = resolve_path::<&str>(None, "./default.txt");
        assert_eq!(result, PathBuf::from("./default.txt"));
    }
}
