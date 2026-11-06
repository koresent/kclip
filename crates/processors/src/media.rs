use std::path::Path;
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MediaMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_secs: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub is_audio: bool,
    pub is_video: bool,
}

#[derive(Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
    tags: Option<FfprobeTags>,
}

#[derive(Deserialize)]
struct FfprobeTags {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
}

#[derive(Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Deserialize)]
struct FfprobeOutput {
    format: Option<FfprobeFormat>,
    streams: Option<Vec<FfprobeStream>>,
}

pub fn probe_media_file(path: &Path) -> Option<MediaMetadata> {
    if !path.exists() {
        return None;
    }

    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout).ok()?;
    let mut meta = MediaMetadata::default();

    if let Some(format) = parsed.format {
        if let Some(dur_str) = format.duration {
            meta.duration_secs = dur_str.parse::<f64>().ok();
        }
        if let Some(tags) = format.tags {
            meta.title = tags.title;
            meta.artist = tags.artist;
            meta.album = tags.album;
        }
    }

    if let Some(streams) = parsed.streams {
        for s in streams {
            if s.codec_type.as_deref() == Some("video") {
                meta.is_video = true;
                if meta.width.is_none() {
                    meta.width = s.width;
                    meta.height = s.height;
                }
            } else if s.codec_type.as_deref() == Some("audio") && !meta.is_video {
                meta.is_audio = true;
            }
        }
    }

    Some(meta)
}

pub fn extract_media_thumbnail(media_path: &Path, out_thumb_path: &Path) -> bool {
    if !media_path.exists() {
        return false;
    }

    // Try video thumbnail at 1s or 0s
    let video_cmd = Command::new("ffmpeg")
        .args([
            "-y",
            "-ss", "00:00:01",
            "-i", media_path.to_str().unwrap_or_default(),
            "-vframes", "1",
            "-vf", "scale=300:-1",
            out_thumb_path.to_str().unwrap_or_default(),
        ])
        .output();

    if let Ok(res) = video_cmd {
        if res.status.success() && out_thumb_path.exists() {
            return true;
        }
    }

    // Try audio cover art extraction
    let audio_cmd = Command::new("ffmpeg")
        .args([
            "-y",
            "-i", media_path.to_str().unwrap_or_default(),
            "-an",
            "-vcodec", "png",
            out_thumb_path.to_str().unwrap_or_default(),
        ])
        .output();

    if let Ok(res) = audio_cmd {
        if res.status.success() && out_thumb_path.exists() {
            return true;
        }
    }

    false
}

pub fn extract_video_preview_frames(media_path: &Path, frames_dir: &Path, count: usize) -> Option<usize> {
    if !media_path.exists() {
        return None;
    }
    let _ = std::fs::create_dir_all(frames_dir);
    let out_pattern = frames_dir.join("frame_%d.png");
    let cmd = Command::new("ffmpeg")
        .args([
            "-y",
            "-ss", "00:00:00",
            "-t", "4",
            "-i", media_path.to_str().unwrap_or_default(),
            "-vf", "fps=4,scale=300:-1",
            "-vframes", &count.to_string(),
            out_pattern.to_str().unwrap_or_default(),
        ])
        .output()
        .ok()?;

    if !cmd.status.success() {
        return None;
    }

    let mut actual = 0;
    while frames_dir.join(format!("frame_{}.png", actual)).exists() {
        actual += 1;
    }

    if actual > 0 {
        Some(actual)
    } else {
        None
    }
}
