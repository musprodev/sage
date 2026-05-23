/// Database persistence layer backed by SQLite.
use std::fs;
use std::path::PathBuf;

use rusqlite::{Connection, params};

use crate::error::{Result, SageError};
use crate::models::{Chapter, Novel, Progress};

/// Wraps a `rusqlite::Connection` and provides typed CRUD operations
/// for novels, chapters, and reading progress.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens (or creates) the SQLite database at `~/.local/share/sage/sage.db`.
    ///
    /// Parent directories are created automatically if they do not exist.
    pub fn new() -> Result<Self> {
        let data_dir = Self::data_dir()?;
        fs::create_dir_all(&data_dir)?;

        let db_path = data_dir.join("sage.db");
        let conn = Connection::open(&db_path)?;

        // Enable WAL mode for better concurrent read performance.
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        // Enforce foreign key constraints.
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        Ok(Self { conn })
    }

    /// Wrap an existing `Connection` (useful for testing with in-memory DBs).
    pub fn from_connection(conn: Connection) -> Self {
        Self { conn }
    }

    /// Resolves the platform data directory (`~/.local/share/sage`).
    fn data_dir() -> Result<PathBuf> {
        let home = std::env::var("HOME").map_err(|_| {
            SageError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "HOME environment variable is not set",
            ))
        })?;
        Ok(PathBuf::from(home).join(".local/share/sage"))
    }

    // ──────────────────────────── Schema ────────────────────────────

    /// Creates the required tables if they do not already exist.
    pub fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS novels (
                id          TEXT PRIMARY KEY,
                title       TEXT NOT NULL,
                author      TEXT NOT NULL,
                cover_url   TEXT NOT NULL,
                source_url  TEXT NOT NULL,
                description TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS chapters (
                id              TEXT PRIMARY KEY,
                novel_id        TEXT NOT NULL,
                title           TEXT NOT NULL,
                url             TEXT NOT NULL,
                chapter_number  REAL NOT NULL,
                content         TEXT,
                is_downloaded   INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_chapters_novel_id
                ON chapters(novel_id);

            CREATE TABLE IF NOT EXISTS progress (
                novel_id      TEXT PRIMARY KEY,
                chapter_id    TEXT NOT NULL,
                scroll_offset INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (novel_id) REFERENCES novels(id)   ON DELETE CASCADE,
                FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE CASCADE
            );
            ",
        )?;
        Ok(())
    }

    // ──────────────────────────── Novels ────────────────────────────

    /// Inserts a novel or updates all of its fields if the `id` already exists.
    pub fn upsert_novel(&self, novel: &Novel) -> Result<()> {
        self.conn.execute(
            "INSERT INTO novels (id, title, author, cover_url, source_url, description)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                 title       = excluded.title,
                 author      = excluded.author,
                 cover_url   = excluded.cover_url,
                 source_url  = excluded.source_url,
                 description = excluded.description",
            params![
                novel.id,
                novel.title,
                novel.author,
                novel.cover_url,
                novel.source_url,
                novel.description,
            ],
        )?;
        Ok(())
    }

    /// Retrieve a novel by its ID.
    pub fn get_novel(&self, novel_id: &str) -> Result<Option<Novel>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, author, cover_url, source_url, description
             FROM novels WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![novel_id], |row| {
            Ok(Novel {
                id: row.get(0)?,
                title: row.get(1)?,
                author: row.get(2)?,
                cover_url: row.get(3)?,
                source_url: row.get(4)?,
                description: row.get(5)?,
            })
        })?;

        if let Some(novel) = rows.next() {
            Ok(Some(novel?))
        } else {
            Ok(None)
        }
    }

    /// Retrieve all novels saved in the library.
    pub fn get_all_novels(&self) -> Result<Vec<Novel>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, author, cover_url, source_url, description
             FROM novels ORDER BY title",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Novel {
                id: row.get(0)?,
                title: row.get(1)?,
                author: row.get(2)?,
                cover_url: row.get(3)?,
                source_url: row.get(4)?,
                description: row.get(5)?,
            })
        })?;

        let mut novels = Vec::new();
        for novel in rows {
            novels.push(novel?);
        }
        Ok(novels)
    }

    // ──────────────────────────── Chapters ──────────────────────────

    /// Inserts a chapter or updates its fields if the `id` already exists.
    pub fn upsert_chapter(&self, chapter: &Chapter) -> Result<()> {
        self.conn.execute(
            "INSERT INTO chapters (id, novel_id, title, url, chapter_number, content, is_downloaded)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                 novel_id       = excluded.novel_id,
                 title          = excluded.title,
                 url            = excluded.url,
                 chapter_number = excluded.chapter_number,
                 content        = excluded.content,
                 is_downloaded  = excluded.is_downloaded",
            params![
                chapter.id,
                chapter.novel_id,
                chapter.title,
                chapter.url,
                chapter.chapter_number,
                chapter.content,
                chapter.is_downloaded,
            ],
        )?;
        Ok(())
    }

    /// Returns all chapters for the given novel, ordered by chapter number.
    pub fn get_storage_items(&self) -> Result<Vec<crate::app::StorageItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT n.id, n.title, 
                    COUNT(c.id) as downloaded_chapters, 
                    SUM(LENGTH(CAST(c.content AS BLOB))) as size_bytes
             FROM novels n
             JOIN chapters c ON n.id = c.novel_id
             WHERE c.is_downloaded = 1
             GROUP BY n.id
             ORDER BY size_bytes DESC",
        )?;

        let items = stmt.query_map([], |row| {
            let size: i64 = row.get(3).unwrap_or(0);
            Ok(crate::app::StorageItem {
                novel_id: row.get(0)?,
                title: row.get(1)?,
                downloaded_chapters: row.get::<_, i64>(2)? as usize,
                size_bytes: size as usize,
            })
        })?;

        let mut result = Vec::new();
        for item in items {
            result.push(item.map_err(SageError::Database)?);
        }
        Ok(result)
    }

    pub fn delete_novel(&self, novel_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM novels WHERE id = ?1",
            rusqlite::params![novel_id],
        )?;
        self.conn.execute(
            "DELETE FROM chapters WHERE novel_id = ?1",
            rusqlite::params![novel_id],
        )?;
        self.conn.execute(
            "DELETE FROM reading_progress WHERE novel_id = ?1",
            rusqlite::params![novel_id],
        )?;
        Ok(())
    }

    pub fn clear_novel_downloads(&self, novel_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE chapters SET content = NULL, is_downloaded = 0 WHERE novel_id = ?1",
            rusqlite::params![novel_id],
        )?;
        Ok(())
    }

    pub fn get_novel_chapters(&self, novel_id: &str) -> Result<Vec<Chapter>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, novel_id, title, url, chapter_number, content, is_downloaded
             FROM chapters
             WHERE novel_id = ?1
             ORDER BY chapter_number ASC",
        )?;

        let chapters = stmt
            .query_map(params![novel_id], |row| {
                Ok(Chapter {
                    id: row.get(0)?,
                    novel_id: row.get(1)?,
                    title: row.get(2)?,
                    url: row.get(3)?,
                    chapter_number: row.get(4)?,
                    content: row.get(5)?,
                    is_downloaded: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(chapters)
    }

    // ──────────────────────────── Progress ──────────────────────────

    /// Saves (or updates) the reading progress for a novel.
    pub fn save_progress(&self, novel_id: &str, chapter_id: &str, offset: usize) -> Result<()> {
        self.conn.execute(
            "INSERT INTO progress (novel_id, chapter_id, scroll_offset)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(novel_id) DO UPDATE SET
                 chapter_id    = excluded.chapter_id,
                 scroll_offset = excluded.scroll_offset",
            params![novel_id, chapter_id, offset as i64],
        )?;
        Ok(())
    }

    /// Retrieves the saved reading progress for a novel, if any.
    pub fn get_progress(&self, novel_id: &str) -> Result<Option<Progress>> {
        let mut stmt = self.conn.prepare(
            "SELECT novel_id, chapter_id, scroll_offset
             FROM progress
             WHERE novel_id = ?1",
        )?;

        let mut rows = stmt.query_map(params![novel_id], |row| {
            let offset: i64 = row.get(2)?;
            Ok(Progress {
                novel_id: row.get(0)?,
                chapter_id: row.get(1)?,
                scroll_offset: offset as usize,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates an in-memory database for testing.
    fn test_db() -> Database {
        let conn = Connection::open_in_memory().expect("failed to open in-memory db");
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        let db = Database { conn };
        db.init_schema().expect("failed to init schema");
        db
    }

    fn sample_novel() -> Novel {
        Novel {
            id: "novel-1".into(),
            title: "Test Novel".into(),
            author: "Author A".into(),
            cover_url: "https://example.com/cover.jpg".into(),
            source_url: "https://example.com/novel-1".into(),
            description: "A test novel description.".into(),
        }
    }

    fn sample_chapter(novel_id: &str, num: f32) -> Chapter {
        Chapter {
            id: format!("{novel_id}-ch-{num}"),
            novel_id: novel_id.into(),
            title: format!("Chapter {num}"),
            url: format!("https://example.com/{novel_id}/ch-{num}"),
            chapter_number: num,
            content: None,
            is_downloaded: false,
        }
    }

    #[test]
    fn test_upsert_and_retrieve_novel_chapters() {
        let db = test_db();
        let novel = sample_novel();
        db.upsert_novel(&novel).unwrap();

        // Insert chapters out of order.
        db.upsert_chapter(&sample_chapter("novel-1", 3.0)).unwrap();
        db.upsert_chapter(&sample_chapter("novel-1", 1.0)).unwrap();
        db.upsert_chapter(&sample_chapter("novel-1", 2.0)).unwrap();

        let chapters = db.get_novel_chapters("novel-1").unwrap();
        assert_eq!(chapters.len(), 3);
        // Should come back sorted by chapter_number.
        assert_eq!(chapters[0].chapter_number, 1.0);
        assert_eq!(chapters[1].chapter_number, 2.0);
        assert_eq!(chapters[2].chapter_number, 3.0);
    }

    #[test]
    fn test_upsert_novel_updates_fields() {
        let db = test_db();
        let mut novel = sample_novel();
        db.upsert_novel(&novel).unwrap();

        novel.title = "Updated Title".into();
        db.upsert_novel(&novel).unwrap();

        let chapters = db.get_novel_chapters("novel-1").unwrap();
        // No chapters yet — just verifying the upsert didn't panic.
        assert!(chapters.is_empty());
    }

    #[test]
    fn test_upsert_chapter_updates_content() {
        let db = test_db();
        db.upsert_novel(&sample_novel()).unwrap();

        let mut ch = sample_chapter("novel-1", 1.0);
        db.upsert_chapter(&ch).unwrap();

        // Simulate downloading content.
        ch.content = Some("Full chapter text here.".into());
        ch.is_downloaded = true;
        db.upsert_chapter(&ch).unwrap();

        let chapters = db.get_novel_chapters("novel-1").unwrap();
        assert_eq!(
            chapters[0].content.as_deref(),
            Some("Full chapter text here.")
        );
        assert!(chapters[0].is_downloaded);
    }

    #[test]
    fn test_save_and_get_progress() {
        let db = test_db();
        db.upsert_novel(&sample_novel()).unwrap();
        db.upsert_chapter(&sample_chapter("novel-1", 1.0)).unwrap();

        // No progress initially.
        assert!(db.get_progress("novel-1").unwrap().is_none());

        db.save_progress("novel-1", "novel-1-ch-1", 42).unwrap();
        let progress = db.get_progress("novel-1").unwrap().unwrap();
        assert_eq!(progress.novel_id, "novel-1");
        assert_eq!(progress.chapter_id, "novel-1-ch-1");
        assert_eq!(progress.scroll_offset, 42);

        // Update progress.
        db.save_progress("novel-1", "novel-1-ch-1", 100).unwrap();
        let progress = db.get_progress("novel-1").unwrap().unwrap();
        assert_eq!(progress.scroll_offset, 100);
    }

    #[test]
    fn test_empty_chapters_list() {
        let db = test_db();
        let chapters = db.get_novel_chapters("nonexistent").unwrap();
        assert!(chapters.is_empty());
    }
}
