use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info};

use app_core::ContentKind;
use capture::{EngineConfig, WaylandClipboardEngine};
use content::{compute_sha256, ClipboardItem, ClipboardRepresentation, MimeType};
use detection::classify_content;
use kclip_config::AppConfig;
use kclip_ipc::{resolve_socket_path, IpcCommand, IpcResponse, IpcServer};
use processors::{clean_url, generate_thumbnail, sanitize_text};
use storage::{BlobStore, Database};

pub fn run_daemon(config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = config
        .storage
        .db_path
        .as_ref()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .or_else(AppConfig::default_data_dir)
        .unwrap_or_else(|| PathBuf::from("/tmp/kclip-data"));

    fs::create_dir_all(&data_dir)?;
    let db_path = config
        .storage
        .db_path
        .clone()
        .unwrap_or_else(|| data_dir.join("kclip.db"));
    let blobs_dir = data_dir.join("blobs");

    info!("Initializing storage at {}", db_path.display());
    let blob_store = BlobStore::new(&blobs_dir)?;
    let db = Arc::new(Mutex::new(Database::open(&db_path, blob_store.clone())?));

    // Clipboard capture engine configuration
    let mut engine_cfg = EngineConfig::default();
    engine_cfg.capture_primary = config.clipboard.capture_primary_selection;
    engine_cfg.max_item_size_mb = config.clipboard.max_item_size_mb;
    engine_cfg.pipeline.quiet_window = Duration::from_millis(config.clipboard.debounce_quiet_ms);
    engine_cfg.pipeline.burst_quiet_window = Duration::from_millis(config.clipboard.debounce_burst_ms);

    info!("Starting Wayland clipboard engine");
    let engine = Arc::new(WaylandClipboardEngine::spawn(engine_cfg)?);
    let socket_path = resolve_socket_path(config.app.socket_path.as_ref());

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    let db_for_ipc = db.clone();
    let engine_for_ipc = engine.clone();

    // Start IPC Server
    let mut ipc_server = IpcServer::start(&socket_path, move |cmd| match cmd {
        IpcCommand::Toggle => {
            info!("Received IPC: Toggle");
            IpcResponse::Ok {
                message: "toggle requested".to_string(),
            }
        }
        IpcCommand::Show => {
            info!("Received IPC: Show");
            IpcResponse::Ok {
                message: "show requested".to_string(),
            }
        }
        IpcCommand::Hide => {
            info!("Received IPC: Hide");
            IpcResponse::Ok {
                message: "hide requested".to_string(),
            }
        }
        IpcCommand::Status => {
            let count = db_for_ipc
                .lock()
                .map(|db| db.count_items().unwrap_or(0))
                .unwrap_or(0);
            IpcResponse::Status {
                running: true,
                item_count: count,
            }
        }
        IpcCommand::Paste { id } => {
            let item_opt = db_for_ipc
                .lock()
                .ok()
                .and_then(|db| db.get_item(id).ok().flatten());

            if let Some(item) = item_opt {
                let mut reps_map = HashMap::new();
                for rep in item.representations {
                    if let Some(inline) = rep.inline_data {
                        reps_map.insert(rep.mime_type.to_string(), inline);
                    } else if let Some(hash) = rep.blob_hash {
                        if let Ok(Some(bytes)) = blob_store.read_blob(&hash) {
                            reps_map.insert(rep.mime_type.to_string(), bytes);
                        }
                    }
                }
                engine_for_ipc.restore_selection(capture::SelectionKind::Clipboard, reps_map);
                IpcResponse::Ok {
                    message: format!("Restored item {}", id),
                }
            } else {
                IpcResponse::Error {
                    message: format!("Item {} not found", id),
                }
            }
        }
        IpcCommand::Quit => {
            info!("Received IPC: Quit");
            running_clone.store(false, Ordering::Relaxed);
            IpcResponse::Ok {
                message: "daemon shutting down".to_string(),
            }
        }
    })?;

    info!("kclip daemon running. Press Ctrl+C or send 'kclip quit' to exit.");

    // Event Loop for incoming clipboard snapshots
    let snapshot_rx = engine.snapshot_receiver().clone();
    let ignore_duplicates = config.clipboard.ignore_duplicates;
    let clean_urls_enabled = config.features.url_cleaner;

    while running.load(Ordering::Relaxed) {
        match snapshot_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(snapshot) => {
                let classification = classify_content(&snapshot.representations);

                let primary_bytes = snapshot
                    .representations
                    .get(&MimeType::new(MimeType::TEXT_PLAIN_UTF8))
                    .or_else(|| snapshot.representations.get(&MimeType::new(MimeType::TEXT_PLAIN)))
                    .or_else(|| snapshot.representations.get(&MimeType::new(MimeType::UTF8_STRING)))
                    .or_else(|| snapshot.representations.values().next());

                let primary_bytes = match primary_bytes {
                    Some(b) => b,
                    None => continue,
                };

                let hash = compute_sha256(primary_bytes);
                let byte_size = snapshot.representations.values().map(|v| v.len()).sum();

                let mut db_guard = match db.lock() {
                    Ok(g) => g,
                    Err(_) => continue,
                };

                if ignore_duplicates {
                    if let Ok(Some(existing)) = db_guard.find_by_hash(&hash, classification.kind) {
                        if let Some(id) = existing.id {
                            let _ = db_guard.bump_item(id, snapshot.timestamp);
                            info!(
                                "Bumped existing item #{} ({})",
                                id,
                                classification.kind.label()
                            );
                            continue;
                        }
                    }
                }

                let mut item = ClipboardItem::new(
                    None,
                    classification.kind,
                    hash,
                    byte_size,
                    snapshot.timestamp,
                );
                item.metadata = classification.metadata;

                // URL cleaning
                if clean_urls_enabled && item.kind == ContentKind::Url {
                    if let Some(ref raw_url) = item.metadata.preview_text {
                        item.metadata.preview_text = Some(clean_url(raw_url));
                    }
                }

                // Text sanitization
                if item.kind == ContentKind::Text || item.kind == ContentKind::Code {
                    if let Some(ref text) = item.metadata.preview_text {
                        item.metadata.preview_text = Some(sanitize_text(text));
                    }
                }

                let thumbnails_dir = data_dir.join("thumbnails");
                let _ = fs::create_dir_all(&thumbnails_dir);

                // Generate and store image thumbnail
                if item.kind == ContentKind::Image {
                    if let Some((_, img_bytes)) = snapshot.representations.iter().find(|(m, _)| m.is_image()) {
                        let thumb_path = thumbnails_dir.join(format!("{}.png", item.hash));
                        if let Ok(thumb) = generate_thumbnail(img_bytes, 200) {
                            item.metadata.img_width = Some(thumb.original_width);
                            item.metadata.img_height = Some(thumb.original_height);
                            let _ = fs::write(&thumb_path, &thumb.png_bytes);
                            item.metadata.preview_text = Some(thumb_path.display().to_string());
                        } else {
                            let _ = fs::write(&thumb_path, img_bytes);
                            item.metadata.preview_text = Some(thumb_path.display().to_string());
                        }

                        let frames_dir = thumbnails_dir.join(format!("{}_frames", item.hash));
                        if let Some(frame_count) = processors::extract_animation_frames(img_bytes, &frames_dir, 30) {
                            let extra = serde_json::json!({
                                "animated": true,
                                "frame_count": frame_count,
                                "frames_dir": frames_dir.display().to_string(),
                            });
                            item.metadata.extra_json = Some(extra.to_string());
                        }
                    }
                }

                // Generate audio & video thumbnail, probe metadata and extract preview frames
                if item.kind == ContentKind::Audio || item.kind == ContentKind::Video {
                    if let Some(ref path_str) = item.metadata.preview_text {
                        let path = std::path::Path::new(path_str);
                        if path.exists() {
                            let thumb_path = thumbnails_dir.join(format!("{}.png", item.hash));
                            let _ = processors::extract_media_thumbnail(path, &thumb_path);
                            let mut extra_val = serde_json::json!({});
                            if let Some(meta) = processors::probe_media_file(path) {
                                if let Ok(val) = serde_json::to_value(&meta) {
                                    extra_val = val;
                                }
                                if item.metadata.title.is_none() && meta.title.is_some() {
                                    item.metadata.title = meta.title;
                                }
                                if meta.width.is_some() {
                                    item.metadata.img_width = meta.width;
                                    item.metadata.img_height = meta.height;
                                }
                            }
                            if item.kind == ContentKind::Video {
                                let frames_dir = thumbnails_dir.join(format!("{}_frames", item.hash));
                                if let Some(count) = processors::extract_video_preview_frames(path, &frames_dir, 20) {
                                    extra_val["animated"] = serde_json::json!(true);
                                    extra_val["frame_count"] = serde_json::json!(count);
                                    extra_val["frames_dir"] = serde_json::json!(frames_dir.display().to_string());
                                }
                            }
                            item.metadata.extra_json = Some(extra_val.to_string());
                        }
                    }
                }

                // Fetch URL preview metadata & banner image
                if item.kind == ContentKind::Url {
                    if let Some(ref url_str) = item.metadata.preview_text {
                        let thumb_path = thumbnails_dir.join(format!("{}.png", item.hash));
                        if let Some(meta) = processors::fetch_url_metadata(url_str, &thumb_path) {
                            if meta.title.is_some() {
                                item.metadata.title = meta.title.clone();
                            }
                            if let Ok(json) = serde_json::to_string(&meta) {
                                item.metadata.extra_json = Some(json);
                            }
                        }
                    }
                }

                // Process representations and store blobs
                for (mime, bytes) in snapshot.representations {
                    let (rep, blob_opt) = ClipboardRepresentation::from_bytes(mime, bytes);
                    if let Some((_, blob_bytes)) = blob_opt {
                        let _ = db_guard.blob_store().write_blob(&blob_bytes);
                    }
                    item.representations.push(rep);
                }

                match db_guard.insert_item(&mut item) {
                    Ok(id) => {
                        info!(
                            "Captured new clipboard item #{}: {} ({} bytes)",
                            id,
                            item.kind.label(),
                            item.byte_size
                        );
                    }
                    Err(e) => {
                        error!("Failed to store clipboard item: {}", e);
                    }
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }
    }

    ipc_server.stop();
    info!("kclip daemon shutdown complete");
    Ok(())
}
