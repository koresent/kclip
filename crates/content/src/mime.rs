use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MimeType(String);

impl MimeType {
    pub const TEXT_PLAIN_UTF8: &'static str = "text/plain;charset=utf-8";
    pub const TEXT_PLAIN: &'static str = "text/plain";
    pub const UTF8_STRING: &'static str = "UTF8_STRING";
    pub const TEXT_HTML: &'static str = "text/html";
    pub const TEXT_RTF: &'static str = "text/rtf";
    pub const APPLICATION_RTF: &'static str = "application/rtf";
    pub const URI_LIST: &'static str = "text/uri-list";
    pub const GNOME_COPIED_FILES: &'static str = "x-special/gnome-copied-files";
    pub const IMAGE_PNG: &'static str = "image/png";
    pub const IMAGE_JPEG: &'static str = "image/jpeg";
    pub const IMAGE_WEBP: &'static str = "image/webp";

    pub fn new(mime: impl Into<String>) -> Self {
        Self(mime.into().trim().to_ascii_lowercase())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_text(&self) -> bool {
        self.0.starts_with("text/") || self.0 == Self::UTF8_STRING
    }

    pub fn is_image(&self) -> bool {
        self.0.starts_with("image/")
    }

    pub fn is_audio(&self) -> bool {
        self.0.starts_with("audio/")
    }

    pub fn is_video(&self) -> bool {
        self.0.starts_with("video/")
    }

    pub fn is_uri_list(&self) -> bool {
        self.0 == Self::URI_LIST || self.0 == Self::GNOME_COPIED_FILES
    }
}

impl<T: Into<String>> From<T> for MimeType {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for MimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for MimeType {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
