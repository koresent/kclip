use crate::error::StorageError;
use rusqlite::Connection;

pub fn initialize_schema(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS clipboard_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            source_app TEXT,
            kind INTEGER NOT NULL,
            hash TEXT NOT NULL,
            byte_size INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            last_used_at INTEGER NOT NULL,
            is_pinned INTEGER NOT NULL DEFAULT 0,
            is_favorite INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS clipboard_representations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            item_id INTEGER NOT NULL REFERENCES clipboard_items(id) ON DELETE CASCADE,
            mime_type TEXT NOT NULL,
            byte_size INTEGER NOT NULL,
            inline_data BLOB,
            blob_hash TEXT,
            UNIQUE(item_id, mime_type)
        );

        CREATE TABLE IF NOT EXISTS content_metadata (
            item_id INTEGER PRIMARY KEY REFERENCES clipboard_items(id) ON DELETE CASCADE,
            title TEXT,
            preview_text TEXT,
            line_count INTEGER,
            char_count INTEGER,
            img_width INTEGER,
            img_height INTEGER,
            extra_json TEXT
        );

        CREATE TABLE IF NOT EXISTS preview_assets (
            item_id INTEGER PRIMARY KEY REFERENCES clipboard_items(id) ON DELETE CASCADE,
            thumbnail_hash TEXT NOT NULL,
            thumb_width INTEGER NOT NULL,
            thumb_height INTEGER NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS clipboard_items_fts USING fts5(
            preview_text,
            title,
            source_app,
            content='content_metadata',
            content_rowid='item_id',
            tokenize='unicode61 remove_diacritics 2'
        );

        CREATE TRIGGER IF NOT EXISTS trg_content_metadata_ai AFTER INSERT ON content_metadata BEGIN
            INSERT INTO clipboard_items_fts(rowid, preview_text, title, source_app)
            VALUES (
                new.item_id,
                new.preview_text,
                new.title,
                (SELECT source_app FROM clipboard_items WHERE id = new.item_id)
            );
        END;

        CREATE TRIGGER IF NOT EXISTS trg_content_metadata_ad AFTER DELETE ON content_metadata BEGIN
            INSERT INTO clipboard_items_fts(clipboard_items_fts, rowid, preview_text, title, source_app)
            VALUES('delete', old.item_id, old.preview_text, old.title, '');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_content_metadata_au AFTER UPDATE ON content_metadata BEGIN
            INSERT INTO clipboard_items_fts(clipboard_items_fts, rowid, preview_text, title, source_app)
            VALUES('delete', old.item_id, old.preview_text, old.title, '');
            INSERT INTO clipboard_items_fts(rowid, preview_text, title, source_app)
            VALUES (
                new.item_id,
                new.preview_text,
                new.title,
                (SELECT source_app FROM clipboard_items WHERE id = new.item_id)
            );
        END;

        CREATE INDEX IF NOT EXISTS idx_items_last_used ON clipboard_items(last_used_at DESC);
        CREATE INDEX IF NOT EXISTS idx_items_kind_last_used ON clipboard_items(kind, last_used_at DESC);
        CREATE INDEX IF NOT EXISTS idx_items_hash_kind ON clipboard_items(hash, kind);
        CREATE INDEX IF NOT EXISTS idx_items_pinned ON clipboard_items(is_pinned, last_used_at DESC);
        CREATE INDEX IF NOT EXISTS idx_reps_item_id ON clipboard_representations(item_id);
        "#,
    )?;

    Ok(())
}
