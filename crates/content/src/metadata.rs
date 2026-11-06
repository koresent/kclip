use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentMetadata {
    pub title: Option<String>,
    pub preview_text: Option<String>,
    pub line_count: Option<usize>,
    pub char_count: Option<usize>,
    pub img_width: Option<u32>,
    pub img_height: Option<u32>,
    pub extra_json: Option<String>,
}
