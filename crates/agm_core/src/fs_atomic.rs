use std::path::Path;

use tokio::fs;

use crate::CoreError;

/// Writes `contents` to `path` via a same-directory `*.tmp` file and [`rename`](tokio::fs::rename)
/// (atomic replace on common Unix setups).
pub async fn write_file_atomic(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<(), CoreError> {
    let path = path.as_ref();
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let pid = std::process::id();
    let tmp = dir.join(format!(".{file_name}.{pid}.tmp"));
    fs::write(&tmp, contents.as_ref()).await?;
    fs::rename(&tmp, path).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn atomic_write_roundtrip() {
        let dir = std::env::temp_dir().join(format!("agm_core_atomic_{}", std::process::id()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let p = dir.join("config.toml");
        write_file_atomic(&p, b"key = 1\n").await.unwrap();
        let s = tokio::fs::read_to_string(&p).await.unwrap();
        assert_eq!(s, "key = 1\n");
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
