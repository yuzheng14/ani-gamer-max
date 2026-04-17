use std::path::{Path, PathBuf};

use sqlx::{
    Row,
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    SqlitePool,
};
use tokio::fs;

use crate::entity::{
    anime_record::{AnimeDownloadResult, AnimeRecord, NewAnimeRecord},
    error::Result,
};

static DEFAULT_DB_PATH: &str = "./data/aniGamer.db";

pub struct DatabaseService {
    pool: SqlitePool,
}

impl DatabaseService {
    pub async fn new<P: AsRef<Path>>(db_path: Option<P>) -> Result<Self> {
        let path = db_path
            .map(|p| p.as_ref().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(DEFAULT_DB_PATH));

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let opts = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);

        let pool = SqlitePool::connect_with(opts).await?;
        let svc = DatabaseService { pool };
        svc.migrate().await?;
        Ok(svc)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS anime (
                sn               INTEGER PRIMARY KEY NOT NULL,
                title            VARCHAR(100) NOT NULL,
                anime_name       VARCHAR(100) NOT NULL,
                episode          VARCHAR(10)  NOT NULL,
                status           TINYINT      NOT NULL DEFAULT 0,
                remote_status    INTEGER      NOT NULL DEFAULT 0,
                resolution       INTEGER      NOT NULL DEFAULT 0,
                file_size        INTEGER      NOT NULL DEFAULT 0,
                local_file_path  VARCHAR(500),
                created_at       TIMESTAMP    NOT NULL DEFAULT (datetime('now','localtime'))
            )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    fn row_to_record(row: &sqlx::sqlite::SqliteRow) -> AnimeRecord {
        AnimeRecord {
            sn: row.get::<i64, _>("sn") as u32,
            title: row.get("title"),
            anime_name: row.get("anime_name"),
            episode: row.get("episode"),
            downloaded: row.get::<i64, _>("status") == 1,
            uploaded: row.get::<i64, _>("remote_status") == 1,
            resolution: row.get::<i64, _>("resolution") as u32,
            file_size: row.get::<i64, _>("file_size") as u32,
            local_file_path: row.get("local_file_path"),
        }
    }

    pub async fn get_all(&self) -> Result<Vec<AnimeRecord>> {
        let rows = sqlx::query("SELECT * FROM anime")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.iter().map(Self::row_to_record).collect())
    }

    pub async fn get(&self, sn: u32) -> Result<Option<AnimeRecord>> {
        let row = sqlx::query("SELECT * FROM anime WHERE sn = ?")
            .bind(sn as i64)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.as_ref().map(Self::row_to_record))
    }

    /// Insert a new entry. Silently ignores duplicate SNs (matches original behaviour).
    pub async fn insert(&self, record: &NewAnimeRecord) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO anime (sn, title, anime_name, episode)
             VALUES (?, ?, ?, ?)",
        )
        .bind(record.sn as i64)
        .bind(&record.title)
        .bind(&record.anime_name)
        .bind(&record.episode)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update the download result fields after a download (and optional upload) completes.
    pub async fn update(&self, result: &AnimeDownloadResult) -> Result<()> {
        sqlx::query(
            "UPDATE anime
             SET status = ?, remote_status = ?, resolution = ?, file_size = ?, local_file_path = ?
             WHERE sn = ?",
        )
        .bind(result.downloaded as i64)
        .bind(result.uploaded as i64)
        .bind(result.resolution as i64)
        .bind(result.file_size as i64)
        .bind(&result.local_file_path)
        .bind(result.sn as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn new_db() -> (DatabaseService, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let svc = DatabaseService::new(Some(dir.path().join("test.db"))).await.unwrap();
        (svc, dir)
    }

    #[tokio::test]
    async fn get_all_empty() {
        let (svc, _dir) = new_db().await;
        assert!(svc.get_all().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn insert_and_get() {
        let (svc, _dir) = new_db().await;
        let record = NewAnimeRecord {
            sn: 10147,
            title: "前進吧！登山少女 第三季 [1]".to_string(),
            anime_name: "前進吧！登山少女 第三季".to_string(),
            episode: "1".to_string(),
        };
        svc.insert(&record).await.unwrap();

        let fetched = svc.get(10147).await.unwrap().unwrap();
        assert_eq!(fetched.sn, 10147);
        assert_eq!(fetched.anime_name, "前進吧！登山少女 第三季");
        assert!(!fetched.downloaded);
        assert!(!fetched.uploaded);
        assert_eq!(fetched.resolution, 0);
    }

    #[tokio::test]
    async fn insert_duplicate_is_ignored() {
        let (svc, _dir) = new_db().await;
        let record = NewAnimeRecord {
            sn: 99,
            title: "T".to_string(),
            anime_name: "A".to_string(),
            episode: "1".to_string(),
        };
        svc.insert(&record).await.unwrap();
        svc.insert(&record).await.unwrap(); // should not error

        assert_eq!(svc.get_all().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn update_download_result() {
        let (svc, _dir) = new_db().await;
        svc.insert(&NewAnimeRecord {
            sn: 200,
            title: "T".to_string(),
            anime_name: "A".to_string(),
            episode: "1".to_string(),
        })
        .await
        .unwrap();

        svc.update(&AnimeDownloadResult {
            sn: 200,
            downloaded: true,
            uploaded: false,
            resolution: 1080,
            file_size: 350,
            local_file_path: Some("/bangumi/A/T.mp4".to_string()),
        })
        .await
        .unwrap();

        let r = svc.get(200).await.unwrap().unwrap();
        assert!(r.downloaded);
        assert!(!r.uploaded);
        assert_eq!(r.resolution, 1080);
        assert_eq!(r.file_size, 350);
        assert_eq!(r.local_file_path.as_deref(), Some("/bangumi/A/T.mp4"));
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let (svc, _dir) = new_db().await;
        assert!(svc.get(99999).await.unwrap().is_none());
    }
}
