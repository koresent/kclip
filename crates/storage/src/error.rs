use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("I/O error at '{path}': {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("blob data corrupted or hash mismatch for '{hash}'")]
    HashMismatch { hash: String },

    #[error("item not found with id: {0}")]
    NotFound(i64),

    #[error("core domain error: {0}")]
    Core(#[from] app_core::error::CoreError),
}
