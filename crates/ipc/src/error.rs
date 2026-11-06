use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IpcError {
    #[error("I/O error connecting to or serving on '{path}': {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to serialize/deserialize IPC message: {0}")]
    Json(#[from] serde_json::Error),

    #[error("connection timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("daemon returned error: {0}")]
    DaemonError(String),
}
