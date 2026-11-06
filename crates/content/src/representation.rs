use crate::mime::MimeType;
use serde::{Deserialize, Serialize};

pub const INLINE_THRESHOLD_BYTES: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardRepresentation {
    pub mime_type: MimeType,
    pub byte_size: usize,
    pub inline_data: Option<Vec<u8>>,
    pub blob_hash: Option<String>,
}

impl ClipboardRepresentation {
    pub fn from_bytes(
        mime_type: impl Into<MimeType>,
        bytes: Vec<u8>,
    ) -> (Self, Option<(String, Vec<u8>)>) {
        let mime = mime_type.into();
        let byte_size = bytes.len();

        if byte_size <= INLINE_THRESHOLD_BYTES {
            (
                Self {
                    mime_type: mime,
                    byte_size,
                    inline_data: Some(bytes),
                    blob_hash: None,
                },
                None,
            )
        } else {
            let hash = crate::hash::compute_sha256(&bytes);
            (
                Self {
                    mime_type: mime,
                    byte_size,
                    inline_data: None,
                    blob_hash: Some(hash.clone()),
                },
                Some((hash, bytes)),
            )
        }
    }
}
