use content::MimeType;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectionKind {
    Clipboard,
    Primary,
}

impl SelectionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Clipboard => "clipboard",
            Self::Primary => "primary",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawClipboardSnapshot {
    pub sequence: u64,
    pub kind: SelectionKind,
    pub representations: HashMap<MimeType, Vec<u8>>,
    pub timestamp: i64,
}
