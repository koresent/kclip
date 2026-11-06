use crate::metadata::ContentMetadata;
use crate::representation::ClipboardRepresentation;
use app_core::{ContentKind, ItemId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: Option<i64>,
    pub uuid: ItemId,
    pub source_app: Option<String>,
    pub kind: ContentKind,
    pub hash: String,
    pub byte_size: usize,
    pub created_at: i64,
    pub last_used_at: i64,
    pub is_pinned: bool,
    pub is_favorite: bool,
    pub representations: Vec<ClipboardRepresentation>,
    pub metadata: ContentMetadata,
}

impl ClipboardItem {
    pub fn new(
        source_app: Option<String>,
        kind: ContentKind,
        hash: String,
        byte_size: usize,
        timestamp: i64,
    ) -> Self {
        Self {
            id: None,
            uuid: ItemId::new(),
            source_app,
            kind,
            hash,
            byte_size,
            created_at: timestamp,
            last_used_at: timestamp,
            is_pinned: false,
            is_favorite: false,
            representations: Vec::new(),
            metadata: ContentMetadata::default(),
        }
    }

    pub fn display_title(&self) -> &str {
        if let Some(ref t) = self.metadata.title {
            return t;
        }
        if let Some(ref s) = self.metadata.preview_text {
            return s;
        }
        self.kind.label()
    }
}
