use rusqlite::{params, Connection, Result};
use crate::types::ContentType;

pub struct HistoryEntry {
    pub content_type: ContentType,
    pub text_content: Option<String>,
    pub image_blob: Option<Vec<u8>>,
}

pub struct ClipboardDb {
    conn: Connection,
}

impl ClipboardDb {
    pub fn new() -> Result<Self> {
        let conn = Connection::open("clipboard_history.db")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY,
                content_type TEXT NOT NULL,
                text_content TEXT,
                image_blob BLOB,
                is_pinned BOOLEAN NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn insert_entry(&self, c_type: ContentType, text: Option<&str>, img: Option<&[u8]>, time: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO history (content_type, text_content, image_blob, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![c_type.as_str(), text, img, time],
        )?;
        Ok(())
    }

    pub fn set_pin_status(&self, id: i64, pin_status: bool) -> Result<()> {
        let pinned_int = if pin_status { 1 } else { 0 };
        let rows_updated = self.conn.execute(
            "UPDATE history SET is_pinned = ?1 WHERE id = ?2",
            params![pinned_int, id],
        )?;

        if rows_updated > 0 {
            println!("Successfully {} entry with ID {}.", if pin_status { "pinned" } else { "unpinned" }, id);
        } else {
            println!("No entry found with ID {}.", id);
        }
        Ok(())
    }

    pub fn print_pinned_items(&self) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content_type, text_content, created_at
             FROM history WHERE is_pinned = 1 ORDER BY created_at DESC"
        )?;

        let pinned_iter = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        println!("\n--- Pinned Items ---");
        for item in pinned_iter {
            if let Ok((id, _c_type, text, time)) = item {
                let display_text = text.unwrap_or_else(|| "[Binary/Image Data]".to_string());
                let short_text = if display_text.len() > 50 {
                    format!("{}...", &display_text[..47]).replace('\n', " ")
                } else {
                    display_text.replace('\n', " ")
                };
                println!("[ID: {}] [{}] - {}", id, time, short_text);
            }
        }
        println!("--------------------\n");
        Ok(())
    }

    pub fn get_entry(&self, id: i64) -> Result<HistoryEntry> {
        let mut stmt = self.conn.prepare(
            "SELECT content_type, text_content, image_blob FROM history WHERE id = ?1"
        )?;

        stmt.query_row([id], |row| {
            let ct_raw: String = row.get(0)?;
            Ok(HistoryEntry {
                content_type: ContentType::from_str(&ct_raw).unwrap_or(ContentType::Text),
                text_content: row.get(1)?,
                image_blob: row.get(2)?,
            })
        })
    }

    pub fn clear_history(&self) -> Result<()> {
        let rows_deleted = self.conn.execute("DELETE FROM history", [])?;
        self.conn.execute("VACUUM", [])?; // Same as purge in oracle to return the size to os.
        //log::info!("Database cleared. Deleted {} entries.", rows_deleted);
        println!("Successfully cleared {} items from history.", rows_deleted);

        Ok(())
    }
}