use std::fs;
use std::path::Path;
use std::process::Command;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UrlMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_name: Option<String>,
    pub image_url: Option<String>,
}

fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

fn extract_meta_tag(html: &str, property: &str) -> Option<String> {
    // Matches property="property" content="value" or content="value" property="property"
    let p_lower = property.to_ascii_lowercase();
    let lower_html = html.to_ascii_lowercase();

    // Check pattern 1: property="..." content="..."
    let mut search_idx = 0;
    while let Some(pos) = lower_html[search_idx..].find(&format!("property=\"{}\"", p_lower))
        .or_else(|| lower_html[search_idx..].find(&format!("name=\"{}\"", p_lower)))
    {
        let abs_pos = search_idx + pos;
        let tag_start = html[..abs_pos].rfind('<').unwrap_or(abs_pos);
        let tag_end = html[abs_pos..].find('>').map(|e| abs_pos + e).unwrap_or(html.len());
        let tag_str = &html[tag_start..tag_end];

        if let Some(c_pos) = tag_str.to_ascii_lowercase().find("content=\"") {
            let after_content = &tag_str[c_pos + 9..];
            if let Some(quote_end) = after_content.find('"') {
                let content = &after_content[..quote_end];
                let decoded = decode_html_entities(content);
                if !decoded.trim().is_empty() {
                    return Some(decoded.trim().to_string());
                }
            }
        }

        search_idx = tag_end;
        if search_idx >= html.len() {
            break;
        }
    }

    None
}

fn extract_title_tag(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    if let Some(start_pos) = lower.find("<title>") {
        let after_start = &html[start_pos + 7..];
        if let Some(end_pos) = after_start.to_ascii_lowercase().find("</title>") {
            let title = &after_start[..end_pos];
            let decoded = decode_html_entities(title);
            if !decoded.trim().is_empty() {
                return Some(decoded.trim().to_string());
            }
        }
    }
    None
}

pub fn fetch_url_metadata(url_str: &str, out_thumb_path: &Path) -> Option<UrlMetadata> {
    if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
        return None;
    }

    info!("Fetching metadata for URL: {}", url_str);

    // Fetch first 64KB of HTML via curl
    let output = Command::new("curl")
        .args([
            "-sL",
            "--max-time", "4",
            "--max-filesize", "2000000",
            "-A", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            url_str,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let html = String::from_utf8_lossy(&output.stdout);

    let mut meta = UrlMetadata::default();
    meta.title = extract_meta_tag(&html, "og:title")
        .or_else(|| extract_meta_tag(&html, "twitter:title"))
        .or_else(|| extract_title_tag(&html));

    meta.description = extract_meta_tag(&html, "og:description")
        .or_else(|| extract_meta_tag(&html, "twitter:description"))
        .or_else(|| extract_meta_tag(&html, "description"));

    meta.site_name = extract_meta_tag(&html, "og:site_name");

    let img_url = extract_meta_tag(&html, "og:image")
        .or_else(|| extract_meta_tag(&html, "twitter:image"))
        .or_else(|| extract_meta_tag(&html, "twitter:image:src"));

    if let Some(img) = img_url {
        // Resolve relative URL if needed
        let resolved_img_url = if img.starts_with("http://") || img.starts_with("https://") {
            img
        } else if let Ok(base) = url::Url::parse(url_str) {
            base.join(&img).map(|u| u.to_string()).unwrap_or(img)
        } else {
            img
        };

        meta.image_url = Some(resolved_img_url.clone());

        // Download banner image
        let temp_img = std::env::temp_dir().join(format!("kclip_url_img_{}.bin", std::process::id()));
        let dl = Command::new("curl")
            .args([
                "-sL",
                "--max-time", "4",
                "--max-filesize", "5000000",
                "-A", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36",
                "-o", temp_img.to_str().unwrap_or_default(),
                &resolved_img_url,
            ])
            .output();

        if let Ok(res) = dl {
            if res.status.success() && temp_img.exists() {
                if let Ok(dyn_img) = image::open(&temp_img) {
                    let thumb = dyn_img.resize(360, 240, image::imageops::FilterType::Triangle);
                    if let Some(parent) = out_thumb_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = thumb.save(out_thumb_path);
                    debug!("Saved URL banner thumbnail to {}", out_thumb_path.display());
                }
                let _ = fs::remove_file(&temp_img);
            }
        }
    }

    Some(meta)
}
