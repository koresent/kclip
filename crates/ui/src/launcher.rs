use std::fs;
use std::path::PathBuf;
use std::process::Command;
use app_core::ContentKind;
use content::ClipboardItem;
use tracing::info;

#[derive(Debug, Clone)]
pub struct AppChoice {
    pub desktop_id: String,
    pub display_name: String,
    pub icon_name: Option<String>,
}

pub fn open_item_default(item: &ClipboardItem) -> Result<(), String> {
    if let Some(path) = resolve_or_dump_item(item) {
        info!("Opening path with xdg-open: {}", path.display());
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to run xdg-open: {}", e))?;
        Ok(())
    } else {
        Err("Cannot resolve item to file or URL".to_string())
    }
}

pub fn open_item_with_app(item: &ClipboardItem, desktop_id: &str) -> Result<(), String> {
    if let Some(path) = resolve_or_dump_item(item) {
        info!("Launching {} with file: {}", desktop_id, path.display());
        Command::new("gtk-launch")
            .arg(desktop_id)
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to run gtk-launch: {}", e))?;
        Ok(())
    } else {
        Err("Cannot resolve item to file".to_string())
    }
}

pub fn resolve_or_dump_item(item: &ClipboardItem) -> Option<PathBuf> {
    // 1. Direct URL
    if item.kind == ContentKind::Url {
        if let Some(ref url) = item.metadata.preview_text {
            return Some(PathBuf::from(url));
        }
    }

    // 2. Existing file on disk
    if let Some(ref preview) = item.metadata.preview_text {
        let p = PathBuf::from(preview);
        if p.exists() {
            return Some(p);
        }
    }

    // Check blob storage
    let base = directories::ProjectDirs::from("com", "koresent", "kclip")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("/home/koresent/.local/share/kclip"));

    let blob = base.join("blobs").join(&item.hash);
    if blob.exists() {
        return Some(blob);
    }

    // 3. Dump text/code to /tmp
    let dump_dir = PathBuf::from("/tmp/kclip-open");
    let _ = fs::create_dir_all(&dump_dir);

    let ext = match item.kind {
        ContentKind::Text => "txt",
        ContentKind::Code => "rs",
        ContentKind::Html => "html",
        ContentKind::Image => "png",
        ContentKind::Color => "txt",
        ContentKind::Audio => "mp3",
        ContentKind::Video => "mp4",
        _ => "dat",
    };

    let target_path = dump_dir.join(format!("{}.{}", &item.hash[0..8.min(item.hash.len())], ext));

    if let Some(ref text) = item.metadata.preview_text {
        let _ = fs::write(&target_path, text.as_bytes());
        return Some(target_path);
    }

    None
}

pub fn get_registered_apps(item: &ClipboardItem) -> Vec<AppChoice> {
    let mime_str = match item.kind {
        ContentKind::Image => "image/png",
        ContentKind::Audio => "audio/x-wav",
        ContentKind::Video => "video/mp4",
        ContentKind::Url => "x-scheme-handler/https",
        ContentKind::Html => "text/html",
        _ => "text/plain",
    };

    let output = Command::new("gio")
        .args(["mime", mime_str])
        .output()
        .ok();

    let mut apps = Vec::new();

    if let Some(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        let mut in_apps = false;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Registered applications:") || trimmed.starts_with("Recommended applications:") {
                in_apps = true;
                continue;
            }
            if in_apps && trimmed.ends_with(".desktop") {
                let desktop_id = trimmed.to_string();
                let display_name = parse_desktop_name(&desktop_id).unwrap_or_else(|| {
                    desktop_id.trim_end_matches(".desktop").to_string()
                });
                if !apps.iter().any(|a: &AppChoice| a.desktop_id == desktop_id) {
                    apps.push(AppChoice {
                        desktop_id,
                        display_name,
                        icon_name: None,
                    });
                }
            }
        }
    }

    if apps.is_empty() {
        // Fallbacks
        apps.push(AppChoice {
            desktop_id: "zen.desktop".to_string(),
            display_name: "Zen Browser".to_string(),
            icon_name: None,
        });
        apps.push(AppChoice {
            desktop_id: "ghostty.desktop".to_string(),
            display_name: "Ghostty".to_string(),
            icon_name: None,
        });
    }

    apps
}

fn parse_desktop_name(desktop_id: &str) -> Option<String> {
    let search_paths = [
        PathBuf::from("/usr/share/applications").join(desktop_id),
        PathBuf::from("/home/koresent/.local/share/applications").join(desktop_id),
    ];

    for p in search_paths {
        if let Ok(content) = fs::read_to_string(&p) {
            for line in content.lines() {
                if line.starts_with("Name=") {
                    return Some(line.trim_start_matches("Name=").trim().to_string());
                }
            }
        }
    }

    None
}
