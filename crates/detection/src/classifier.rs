use crate::code::detect_code;
use crate::color::detect_color;
use crate::files::detect_file_list;
use crate::url::detect_url;
use app_core::ContentKind;
use content::{ContentMetadata, MimeType};
use std::collections::HashMap;

pub struct ClassificationResult {
    pub kind: ContentKind,
    pub metadata: ContentMetadata,
}

pub fn classify_content(representations: &HashMap<MimeType, Vec<u8>>) -> ClassificationResult {
    // 1. Files
    for (mime, bytes) in representations {
        if mime.is_uri_list()
            && let Ok(text) = std::str::from_utf8(bytes)
            && let Some(files) = detect_file_list(text)
        {
            let preview = if files.count == 1 {
                files.paths[0].display().to_string()
            } else {
                format!("{} files", files.count)
            };

            let kind = if files.count == 1 {
                let ext = files.paths[0]
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();

                if matches!(ext.as_str(), "mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac" | "opus" | "wma") {
                    ContentKind::Audio
                } else if matches!(ext.as_str(), "mp4" | "mkv" | "webm" | "mov" | "avi" | "m4v" | "wmv") {
                    ContentKind::Video
                } else {
                    ContentKind::Files
                }
            } else {
                ContentKind::Files
            };

            let title = match kind {
                ContentKind::Audio | ContentKind::Video | ContentKind::Files if files.count == 1 => {
                    files.paths[0].file_name().and_then(|n| n.to_str()).map(|s| s.to_string())
                }
                _ => Some(format!("{} files", files.count)),
            };

            return ClassificationResult {
                kind,
                metadata: ContentMetadata {
                    title,
                    preview_text: Some(preview),
                    line_count: Some(files.count),
                    char_count: None,
                    img_width: None,
                    img_height: None,
                    extra_json: None,
                },
            };
        }
    }

    // 2. Audio & Video direct MIME
    for mime in representations.keys() {
        if mime.is_audio() {
            return ClassificationResult {
                kind: ContentKind::Audio,
                metadata: ContentMetadata {
                    title: Some("Audio Clip".to_string()),
                    preview_text: None,
                    line_count: None,
                    char_count: None,
                    img_width: None,
                    img_height: None,
                    extra_json: None,
                },
            };
        } else if mime.is_video() {
            return ClassificationResult {
                kind: ContentKind::Video,
                metadata: ContentMetadata {
                    title: Some("Video Clip".to_string()),
                    preview_text: None,
                    line_count: None,
                    char_count: None,
                    img_width: None,
                    img_height: None,
                    extra_json: None,
                },
            };
        }
    }

    // 3. Images
    for mime in representations.keys() {
        if mime.is_image() {
            return ClassificationResult {
                kind: ContentKind::Image,
                metadata: ContentMetadata {
                    title: Some("Image".to_string()),
                    preview_text: None,
                    line_count: None,
                    char_count: None,
                    img_width: None,
                    img_height: None,
                    extra_json: None,
                },
            };
        }
    }

    // 3. Text representations
    for (mime, bytes) in representations {
        if mime.is_text()
            && let Ok(text) = std::str::from_utf8(bytes)
        {
            let trimmed = text.trim();
            let char_count = text.chars().count();
            let line_count = text.lines().count();

            // 3a. Color
            if let Some(color) = detect_color(trimmed) {
                return ClassificationResult {
                    kind: ContentKind::Color,
                    metadata: ContentMetadata {
                        title: Some(color.hex.clone()),
                        preview_text: Some(color.hex.clone()),
                        line_count: Some(1),
                        char_count: Some(color.hex.len()),
                        img_width: None,
                        img_height: None,
                        extra_json: Some(format!(
                            r#"{{"r":{},"g":{},"b":{},"a":{}}}"#,
                            color.r, color.g, color.b, color.a
                        )),
                    },
                };
            }

            // 3b. URL
            if let Some(url) = detect_url(trimmed) {
                let host = url.host.clone().unwrap_or_else(|| "link".to_string());
                return ClassificationResult {
                    kind: ContentKind::Url,
                    metadata: ContentMetadata {
                        title: Some(host),
                        preview_text: Some(url.raw),
                        line_count: Some(1),
                        char_count: Some(char_count),
                        img_width: None,
                        img_height: None,
                        extra_json: None,
                    },
                };
            }

            // 3c. Code
            if let Some(code) = detect_code(trimmed) {
                let preview = text.chars().take(500).collect::<String>();
                let extra = code
                    .language_hint
                    .as_ref()
                    .map(|l| format!(r#"{{"lang":"{}"}}"#, l));
                return ClassificationResult {
                    kind: ContentKind::Code,
                    metadata: ContentMetadata {
                        title: code.language_hint,
                        preview_text: Some(preview),
                        line_count: Some(line_count),
                        char_count: Some(char_count),
                        img_width: None,
                        img_height: None,
                        extra_json: extra,
                    },
                };
            }

            // 3d. Default plain text
            let preview = text.chars().take(2000).collect::<String>();
            return ClassificationResult {
                kind: ContentKind::Text,
                metadata: ContentMetadata {
                    title: None,
                    preview_text: Some(preview),
                    line_count: Some(line_count),
                    char_count: Some(char_count),
                    img_width: None,
                    img_height: None,
                    extra_json: None,
                },
            };
        }
    }

    // 4. HTML
    if representations
        .keys()
        .any(|m| m.as_str() == MimeType::TEXT_HTML)
    {
        return ClassificationResult {
            kind: ContentKind::Html,
            metadata: ContentMetadata {
                title: Some("HTML".to_string()),
                preview_text: None,
                line_count: None,
                char_count: None,
                img_width: None,
                img_height: None,
                extra_json: None,
            },
        };
    }

    // 5. Binary
    ClassificationResult {
        kind: ContentKind::Binary,
        metadata: ContentMetadata::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_color() {
        let mut map = HashMap::new();
        map.insert(MimeType::TEXT_PLAIN_UTF8.into(), b"#b8bb26".to_vec());
        let res = classify_content(&map);
        assert_eq!(res.kind, ContentKind::Color);
        assert_eq!(res.metadata.preview_text.as_deref(), Some("#B8BB26"));
    }

    #[test]
    fn classify_url() {
        let mut map = HashMap::new();
        map.insert(
            MimeType::TEXT_PLAIN_UTF8.into(),
            b"https://example.com/page".to_vec(),
        );
        let res = classify_content(&map);
        assert_eq!(res.kind, ContentKind::Url);
        assert_eq!(res.metadata.title.as_deref(), Some("example.com"));
    }

    #[test]
    fn classify_code() {
        let mut map = HashMap::new();
        map.insert(
            MimeType::TEXT_PLAIN_UTF8.into(),
            b"pub fn main() {\n    println!(\"hi\");\n}".to_vec(),
        );
        let res = classify_content(&map);
        assert_eq!(res.kind, ContentKind::Code);
    }
}
