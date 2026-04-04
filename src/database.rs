use crate::types::ContentType;
use chrono::{DateTime, Local, Utc};
use log::{debug, error, info, warn};
use rusqlite::{Connection, Result, params};

// --- SQL Constants ---
const QUERY_CREATE_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS history (
        id INTEGER PRIMARY KEY,
        content_type TEXT NOT NULL,
        text_content TEXT,
        image_blob BLOB,
        is_pinned BOOLEAN NOT NULL DEFAULT 0,
        created_at INTEGER DEFAULT (unixepoch())
    )";

const INSERT_QUERY: &str = "
    INSERT INTO history (content_type, text_content, image_blob)
    VALUES (?1, ?2, ?3)";

const GET_ENTRY_QUERY: &str = "
    SELECT id, content_type, text_content, image_blob, created_at
    FROM history WHERE id = ?1";

const GET_PINNED_QUERY: &str = "
    SELECT id, content_type, text_content, image_blob, created_at
    FROM history
    WHERE is_pinned = 1
    ORDER BY created_at DESC";

const SET_PIN_QUERY: &str = "UPDATE history SET is_pinned = ?1 WHERE id = ?2";
const TRUNCATE_QUERY: &str = "DELETE FROM history";
const VACUUM_QUERY: &str = "VACUUM";
const TIME_FORMAT: &str = "%m-%d-%Y %H:%M:%S";

pub(crate) fn to_readable_time(ts: i64) -> String {
    let naive = DateTime::from_timestamp(ts, 0).unwrap_or_default();
    let local_time: DateTime<Local> = DateTime::from(naive);
    let time_str = local_time.format(TIME_FORMAT).to_string();
    time_str
}

pub struct HistoryEntry {
    pub id: i64,
    pub content_type: ContentType,
    pub text_content: Option<String>,
    pub image_blob: Option<Vec<u8>>,
    pub created_at: i64,
}

pub struct ClipboardDb {
    conn: Connection,
}

impl ClipboardDb {
    pub fn new() -> Result<Self> {
        debug!("Opening database connection: clipboard_history.db");
        let conn = Connection::open("clipboard_history.db")?;
        conn.execute(QUERY_CREATE_TABLE, [])?;
        Ok(Self { conn })
    }

    pub fn insert_entry(
        &self,
        c_type: ContentType,
        text: Option<&str>,
        img: Option<&[u8]>,
    ) -> Result<()> {
        debug!("Inserting new {:?} entry into database", c_type);
        self.conn
            .execute(INSERT_QUERY, params![c_type.as_str(), text, img])?;
        debug!("Database: Successfully saved new {:?}", c_type);
        Ok(())
    }

    pub fn set_pin_status(&self, id: i64, pinned: bool) -> Result<()> {
        let status = if pinned { "pinned" } else { "unpinned" };
        debug!("Updating pin status for ID: {} to {}", id, status);

        self.conn
            .execute(SET_PIN_QUERY, params![pinned as i32, id])?;
        debug!(
            "Item {} {}",
            id,
            if pinned { "pinned 📌" } else { "unpinned" }
        );
        Ok(())
    }

    pub fn get_pinned_items(&self) -> Result<Vec<HistoryEntry>, Box<dyn std::error::Error>> {
        debug!("Fetching pinned items from database");
        let mut stmt = self.conn.prepare(GET_PINNED_QUERY)?;

        // 1. Map the rows to HistoryEntry structs
        let item_iter = stmt.query_map([], |row| {
            let ct_raw: String = row.get(1)?;
            Ok(HistoryEntry {
                id: row.get(0)?,
                content_type: ContentType::from_str(&ct_raw).unwrap_or(ContentType::Text),
                text_content: row.get(2)?,
                image_blob: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        let items: Result<Vec<HistoryEntry>, rusqlite::Error> = item_iter.collect();

        Ok(items?)
    }
    pub fn get_entry(&self, id: i64) -> Result<HistoryEntry> {
        self.conn.query_row(GET_ENTRY_QUERY, [id], |row| {
            let ct_raw: String = row.get(1)?;
            Ok(HistoryEntry {
                id: row.get(0)?,
                content_type: ContentType::from_str(&ct_raw).unwrap_or(ContentType::Text),
                text_content: row.get(2)?,
                image_blob: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
    }

    pub fn clear_history(&self) -> Result<()> {
        warn!("Clearing all clipboard history from database!");
        self.conn.execute(TRUNCATE_QUERY, [])?;
        self.conn.execute(VACUUM_QUERY, [])?;
        info!("History cleared and database vacuumed.");
        Ok(())
    }
}
