use crate::blob::BlobStore;
use crate::error::StorageError;
use crate::schema::initialize_schema;
use app_core::ContentKind;
use content::{ClipboardItem, ClipboardRepresentation, ContentMetadata, MimeType};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

pub struct Database {
    conn: Connection,
    blob_store: BlobStore,
}

impl Database {
    pub fn open(path: impl AsRef<Path>, blob_store: BlobStore) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
        initialize_schema(&conn)?;
        Ok(Self { conn, blob_store })
    }

    pub fn open_in_memory(blob_store: BlobStore) -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        initialize_schema(&conn)?;
        Ok(Self { conn, blob_store })
    }

    pub fn blob_store(&self) -> &BlobStore {
        &self.blob_store
    }

    pub fn insert_item(&mut self, item: &mut ClipboardItem) -> Result<i64, StorageError> {
        let tx = self.conn.transaction()?;

        tx.execute(
            r#"
            INSERT INTO clipboard_items (
                uuid, source_app, kind, hash, byte_size, created_at, last_used_at, is_pinned, is_favorite
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                item.uuid.to_string(),
                item.source_app,
                item.kind.as_i32(),
                item.hash,
                item.byte_size as i64,
                item.created_at,
                item.last_used_at,
                item.is_pinned as i32,
                item.is_favorite as i32,
            ],
        )?;

        let item_id = tx.last_insert_rowid();
        item.id = Some(item_id);

        for rep in &item.representations {
            tx.execute(
                r#"
                INSERT INTO clipboard_representations (
                    item_id, mime_type, byte_size, inline_data, blob_hash
                ) VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![
                    item_id,
                    rep.mime_type.as_str(),
                    rep.byte_size as i64,
                    rep.inline_data,
                    rep.blob_hash,
                ],
            )?;
        }

        tx.execute(
            r#"
            INSERT INTO content_metadata (
                item_id, title, preview_text, line_count, char_count, img_width, img_height, extra_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                item_id,
                item.metadata.title,
                item.metadata.preview_text,
                item.metadata.line_count.map(|c| c as i64),
                item.metadata.char_count.map(|c| c as i64),
                item.metadata.img_width,
                item.metadata.img_height,
                item.metadata.extra_json,
            ],
        )?;

        tx.commit()?;
        Ok(item_id)
    }

    pub fn find_by_hash(
        &self,
        hash: &str,
        kind: ContentKind,
    ) -> Result<Option<ClipboardItem>, StorageError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, uuid, source_app, kind, hash, byte_size, created_at, last_used_at, is_pinned, is_favorite
            FROM clipboard_items
            WHERE hash = ?1 AND kind = ?2
            ORDER BY last_used_at DESC
            LIMIT 1
            "#,
        )?;

        let item_res = stmt
            .query_row(params![hash, kind.as_i32()], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i32>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i32>(8)? != 0,
                    row.get::<_, i32>(9)? != 0,
                ))
            })
            .optional()?;

        match item_res {
            Some((
                id,
                uuid_str,
                source_app,
                kind_i32,
                hash,
                byte_size,
                created_at,
                last_used_at,
                is_pinned,
                is_favorite,
            )) => {
                let uuid = uuid_str.parse().map_err(StorageError::Core)?;
                let kind = ContentKind::from_i32(kind_i32)?;
                let reps = self.load_representations(id)?;
                let metadata = self.load_metadata(id)?.unwrap_or_default();

                Ok(Some(ClipboardItem {
                    id: Some(id),
                    uuid,
                    source_app,
                    kind,
                    hash,
                    byte_size: byte_size as usize,
                    created_at,
                    last_used_at,
                    is_pinned,
                    is_favorite,
                    representations: reps,
                    metadata,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn bump_item(&mut self, id: i64, timestamp: i64) -> Result<(), StorageError> {
        self.conn.execute(
            "UPDATE clipboard_items SET last_used_at = ?1 WHERE id = ?2",
            params![timestamp, id],
        )?;
        Ok(())
    }

    pub fn get_item(&self, id: i64) -> Result<Option<ClipboardItem>, StorageError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, uuid, source_app, kind, hash, byte_size, created_at, last_used_at, is_pinned, is_favorite
            FROM clipboard_items
            WHERE id = ?1
            "#,
        )?;

        let row_opt = stmt
            .query_row(params![id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i32>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i32>(8)? != 0,
                    row.get::<_, i32>(9)? != 0,
                ))
            })
            .optional()?;

        match row_opt {
            Some((
                id,
                uuid_str,
                source_app,
                kind_i32,
                hash,
                byte_size,
                created_at,
                last_used_at,
                is_pinned,
                is_favorite,
            )) => {
                let uuid = uuid_str.parse().map_err(StorageError::Core)?;
                let kind = ContentKind::from_i32(kind_i32)?;
                let reps = self.load_representations(id)?;
                let metadata = self.load_metadata(id)?.unwrap_or_default();

                Ok(Some(ClipboardItem {
                    id: Some(id),
                    uuid,
                    source_app,
                    kind,
                    hash,
                    byte_size: byte_size as usize,
                    created_at,
                    last_used_at,
                    is_pinned,
                    is_favorite,
                    representations: reps,
                    metadata,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn list_items(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ClipboardItem>, StorageError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, uuid, source_app, kind, hash, byte_size, created_at, last_used_at, is_pinned, is_favorite
            FROM clipboard_items
            ORDER BY last_used_at DESC, id DESC
            LIMIT ?1 OFFSET ?2
            "#,
        )?;

        let rows = stmt.query_map(params![limit as i64, offset as i64], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i32>(8)? != 0,
                row.get::<_, i32>(9)? != 0,
            ))
        })?;

        let mut items = Vec::new();
        for r in rows {
            let (
                id,
                uuid_str,
                source_app,
                kind_i32,
                hash,
                byte_size,
                created_at,
                last_used_at,
                is_pinned,
                is_favorite,
            ) = r?;
            let uuid = uuid_str.parse().map_err(StorageError::Core)?;
            let kind = ContentKind::from_i32(kind_i32)?;
            let reps = self.load_representations(id)?;
            let metadata = self.load_metadata(id)?.unwrap_or_default();

            items.push(ClipboardItem {
                id: Some(id),
                uuid,
                source_app,
                kind,
                hash,
                byte_size: byte_size as usize,
                created_at,
                last_used_at,
                is_pinned,
                is_favorite,
                representations: reps,
                metadata,
            });
        }

        Ok(items)
    }

    pub fn search_items(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, StorageError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return self.list_items(limit, 0);
        }

        let fts_query = format!("{}*", trimmed.replace('"', "\"\""));

        let mut stmt = self.conn.prepare(
            r#"
            SELECT i.id, i.uuid, i.source_app, i.kind, i.hash, i.byte_size, i.created_at, i.last_used_at, i.is_pinned, i.is_favorite
            FROM clipboard_items_fts fts
            JOIN clipboard_items i ON i.id = fts.rowid
            WHERE clipboard_items_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2
            "#,
        )?;

        let rows = stmt.query_map(params![fts_query, limit as i64], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i32>(8)? != 0,
                row.get::<_, i32>(9)? != 0,
            ))
        })?;

        let mut items = Vec::new();
        for r in rows {
            let (
                id,
                uuid_str,
                source_app,
                kind_i32,
                hash,
                byte_size,
                created_at,
                last_used_at,
                is_pinned,
                is_favorite,
            ) = r?;
            let uuid = uuid_str.parse().map_err(StorageError::Core)?;
            let kind = ContentKind::from_i32(kind_i32)?;
            let reps = self.load_representations(id)?;
            let metadata = self.load_metadata(id)?.unwrap_or_default();

            items.push(ClipboardItem {
                id: Some(id),
                uuid,
                source_app,
                kind,
                hash,
                byte_size: byte_size as usize,
                created_at,
                last_used_at,
                is_pinned,
                is_favorite,
                representations: reps,
                metadata,
            });
        }

        Ok(items)
    }

    pub fn delete_item(&mut self, id: i64) -> Result<bool, StorageError> {
        let tx = self.conn.transaction()?;

        let blob_hashes: Vec<String> = {
            let mut stmt = tx.prepare("SELECT blob_hash FROM clipboard_representations WHERE item_id = ?1 AND blob_hash IS NOT NULL")?;
            stmt.query_map(params![id], |r| r.get(0))?
                .collect::<Result<_, _>>()?
        };

        let rows_affected = tx.execute("DELETE FROM clipboard_items WHERE id = ?1", params![id])?;
        tx.commit()?;

        for hash in blob_hashes {
            let _ = self.blob_store.delete_blob(&hash);
        }

        Ok(rows_affected > 0)
    }

    pub fn prune_retention(&mut self, cutoff_timestamp: i64) -> Result<usize, StorageError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT r.blob_hash
            FROM clipboard_items i
            JOIN clipboard_representations r ON r.item_id = i.id
            WHERE i.is_pinned = 0 AND i.is_favorite = 0 AND i.created_at < ?1 AND r.blob_hash IS NOT NULL
            "#,
        )?;

        let blob_hashes: Vec<String> = stmt
            .query_map(params![cutoff_timestamp], |r| r.get(0))?
            .collect::<Result<_, _>>()?;

        let rows_deleted = self.conn.execute(
            "DELETE FROM clipboard_items WHERE is_pinned = 0 AND is_favorite = 0 AND created_at < ?1",
            params![cutoff_timestamp],
        )?;

        for hash in blob_hashes {
            let _ = self.blob_store.delete_blob(&hash);
        }

        Ok(rows_deleted)
    }

    pub fn count_items(&self) -> Result<usize, StorageError> {
        let count: i64 = self
            .conn
            .query_row("SELECT count(*) FROM clipboard_items", [], |r| r.get(0))?;
        Ok(count as usize)
    }

    fn load_representations(
        &self,
        item_id: i64,
    ) -> Result<Vec<ClipboardRepresentation>, StorageError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT mime_type, byte_size, inline_data, blob_hash
            FROM clipboard_representations
            WHERE item_id = ?1
            "#,
        )?;

        let rows = stmt.query_map(params![item_id], |row| {
            let mime_str: String = row.get(0)?;
            let byte_size: i64 = row.get(1)?;
            let inline_data: Option<Vec<u8>> = row.get(2)?;
            let blob_hash: Option<String> = row.get(3)?;

            Ok(ClipboardRepresentation {
                mime_type: MimeType::new(mime_str),
                byte_size: byte_size as usize,
                inline_data,
                blob_hash,
            })
        })?;

        let mut reps = Vec::new();
        for r in rows {
            reps.push(r?);
        }
        Ok(reps)
    }

    fn load_metadata(&self, item_id: i64) -> Result<Option<ContentMetadata>, StorageError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT title, preview_text, line_count, char_count, img_width, img_height, extra_json
            FROM content_metadata
            WHERE item_id = ?1
            "#,
        )?;

        let meta = stmt
            .query_row(params![item_id], |row| {
                Ok(ContentMetadata {
                    title: row.get(0)?,
                    preview_text: row.get(1)?,
                    line_count: row.get::<_, Option<i64>>(2)?.map(|v| v as usize),
                    char_count: row.get::<_, Option<i64>>(3)?.map(|v| v as usize),
                    img_width: row.get(4)?,
                    img_height: row.get(5)?,
                    extra_json: row.get(6)?,
                })
            })
            .optional()?;

        Ok(meta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn insert_and_retrieve_item() {
        let dir = tempdir().unwrap();
        let blob_store = BlobStore::new(dir.path()).unwrap();
        let mut db = Database::open_in_memory(blob_store).unwrap();

        let mut item = ClipboardItem::new(
            Some("ghostty".to_string()),
            ContentKind::Text,
            "hash123".to_string(),
            11,
            1000,
        );
        let (rep, _) =
            ClipboardRepresentation::from_bytes(MimeType::TEXT_PLAIN_UTF8, b"hello world".to_vec());
        item.representations.push(rep);
        item.metadata.preview_text = Some("hello world".to_string());

        let id = db.insert_item(&mut item).unwrap();
        assert_eq!(id, 1);

        let retrieved = db.get_item(id).unwrap().expect("item must exist");
        assert_eq!(retrieved.hash, "hash123");
        assert_eq!(retrieved.source_app.as_deref(), Some("ghostty"));
        assert_eq!(
            retrieved.metadata.preview_text.as_deref(),
            Some("hello world")
        );
        assert_eq!(retrieved.representations.len(), 1);

        let found = db.find_by_hash("hash123", ContentKind::Text).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, Some(1));
    }

    #[test]
    fn full_text_search() {
        let dir = tempdir().unwrap();
        let blob_store = BlobStore::new(dir.path()).unwrap();
        let mut db = Database::open_in_memory(blob_store).unwrap();

        let mut item1 = ClipboardItem::new(
            Some("zen".to_string()),
            ContentKind::Text,
            "h1".to_string(),
            20,
            1000,
        );
        item1.metadata.preview_text = Some("rust memory safety and concurrency".to_string());
        db.insert_item(&mut item1).unwrap();

        let mut item2 = ClipboardItem::new(
            Some("zen".to_string()),
            ContentKind::Text,
            "h2".to_string(),
            20,
            2000,
        );
        item2.metadata.preview_text = Some("python dynamic typing and scripting".to_string());
        db.insert_item(&mut item2).unwrap();

        let results = db.search_items("memory", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].hash, "h1");

        let results_python = db.search_items("python", 10).unwrap();
        assert_eq!(results_python.len(), 1);
        assert_eq!(results_python[0].hash, "h2");
    }

    #[test]
    fn bump_and_delete() {
        let dir = tempdir().unwrap();
        let blob_store = BlobStore::new(dir.path()).unwrap();
        let mut db = Database::open_in_memory(blob_store).unwrap();

        let mut item = ClipboardItem::new(None, ContentKind::Text, "h1".to_string(), 5, 100);
        let id = db.insert_item(&mut item).unwrap();

        db.bump_item(id, 200).unwrap();
        let updated = db.get_item(id).unwrap().unwrap();
        assert_eq!(updated.last_used_at, 200);

        assert!(db.delete_item(id).unwrap());
        assert!(db.get_item(id).unwrap().is_none());
    }
}
