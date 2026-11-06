use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Duration;
use tracing::error;

pub struct MediaPlayer {
    socket_path: PathBuf,
    process: Option<Child>,
    pub current_file: Option<PathBuf>,
    pub current_item_hash: Option<String>,
    pub is_playing: bool,
    pub is_paused: bool,
    pub position: f64,
    pub duration: f64,
    last_status_time: Option<std::time::Instant>,
    last_seek_time: Option<std::time::Instant>,
}

impl Default for MediaPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaPlayer {
    pub fn new() -> Self {
        let socket_path = PathBuf::from(format!("/tmp/kclip-mpv-{}.sock", std::process::id()));
        Self {
            socket_path,
            process: None,
            current_file: None,
            current_item_hash: None,
            is_playing: false,
            is_paused: false,
            position: 0.0,
            duration: 0.0,
            last_status_time: None,
            last_seek_time: None,
        }
    }

    fn ensure_daemon(&mut self) -> bool {
        if self.process.is_some() && self.socket_path.exists() {
            return true;
        }

        let _ = std::fs::remove_file(&self.socket_path);

        match Command::new("mpv")
            .arg("--idle")
            .arg("--no-config")
            .arg("--vo=null")
            .arg("--force-window=no")
            .arg("--audio-display=no")
            .arg("--keep-open=yes")
            .arg("--loop-file=inf")
            .arg("--framedrop=vo")
            .arg("--hr-seek=always")
            .arg(format!("--input-ipc-server={}", self.socket_path.display()))
            .spawn()
        {
            Ok(child) => {
                self.process = Some(child);
                // Give socket a moment to initialize
                for _ in 0..10 {
                    if self.socket_path.exists() {
                        return true;
                    }
                    std::thread::sleep(Duration::from_millis(30));
                }
                self.socket_path.exists()
            }
            Err(e) => {
                error!("Failed to spawn mpv: {}", e);
                false
            }
        }
    }

    fn send_ipc(&self, json_cmd: &str) -> Option<serde_json::Value> {
        let mut stream = UnixStream::connect(&self.socket_path).ok()?;
        stream.set_read_timeout(Some(Duration::from_millis(35))).ok()?;
        stream.set_write_timeout(Some(Duration::from_millis(35))).ok()?;

        stream.write_all(json_cmd.as_bytes()).ok()?;
        stream.write_all(b"\n").ok()?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        while reader.read_line(&mut line).ok()? > 0 {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                // Ignore asynchronous event messages from mpv
                if v.get("event").is_none() {
                    return Some(v);
                }
            }
            line.clear();
        }

        None
    }

    pub fn play(&mut self, path: &Path, hash: String) {
        if !self.ensure_daemon() {
            return;
        }

        // If same item, toggle pause
        if self.current_item_hash.as_deref() == Some(&hash) && self.is_playing {
            self.toggle_pause();
            return;
        }

        let escaped = path.display().to_string().replace('"', "\\\"");
        let cmd = format!("{{\"command\": [\"loadfile\", \"{}\"]}}", escaped);
        if self.send_ipc(&cmd).is_some() {
            self.current_file = Some(path.to_path_buf());
            self.current_item_hash = Some(hash);
            self.is_playing = true;
            self.is_paused = false;
            self.position = 0.0;
            self.duration = 0.0;
        }
    }

    pub fn play_at(&mut self, path: &Path, hash: String, percent: f64) {
        if !self.ensure_daemon() {
            return;
        }

        let clamped = percent.clamp(0.0, 100.0);
        let escaped = path.display().to_string().replace('"', "\\\"");
        let _ = self.send_ipc(&format!("{{\"command\": [\"set_property\", \"start\", \"{:.1}%\"]}}", clamped));
        let cmd = format!("{{\"command\": [\"loadfile\", \"{}\"]}}", escaped);
        if self.send_ipc(&cmd).is_some() {
            self.current_file = Some(path.to_path_buf());
            self.current_item_hash = Some(hash);
            self.is_playing = true;
            self.is_paused = false;
            if self.duration > 0.0 {
                self.position = (clamped / 100.0) * self.duration;
            }
        }
    }

    pub fn toggle_pause(&mut self) {
        if !self.is_playing {
            return;
        }
        if self.duration > 0.0 && self.position >= self.duration - 0.2 {
            self.seek_percent(0.0);
        }
        let cmd = "{\"command\": [\"cycle\", \"pause\"]}";
        if self.send_ipc(cmd).is_some() {
            self.is_paused = !self.is_paused;
        }
    }

    pub fn seek_relative(&mut self, seconds: f64) {
        if !self.is_playing {
            return;
        }
        let cmd = format!("{{\"command\": [\"seek\", {}, \"relative+exact\"]}}", seconds);
        let _ = self.send_ipc(&cmd);
        self.update_status();
    }

    pub fn seek_percent(&mut self, percent: f64) {
        if !self.is_playing {
            return;
        }
        let clamped = percent.clamp(0.0, 100.0);
        if self.duration > 0.0 {
            self.position = (clamped / 100.0) * self.duration;
        }
        if let Some(last) = self.last_seek_time {
            if last.elapsed() < Duration::from_millis(40) {
                return;
            }
        }
        self.last_seek_time = Some(std::time::Instant::now());
        let cmd = format!("{{\"command\": [\"seek\", {:.2}, \"absolute-percent+exact\"]}}", clamped);
        let _ = self.send_ipc(&cmd);
    }

    pub fn stop(&mut self) {
        if self.is_playing {
            let _ = self.send_ipc("{\"command\": [\"stop\"]}");
            self.is_playing = false;
            self.is_paused = false;
            self.current_file = None;
            self.current_item_hash = None;
            self.position = 0.0;
        }
    }

    pub fn update_status(&mut self) {
        if !self.is_playing {
            return;
        }

        if let Some(last) = self.last_status_time {
            if last.elapsed() < Duration::from_millis(60) {
                return;
            }
        }
        self.last_status_time = Some(std::time::Instant::now());

        if let Some(pos_val) = self.send_ipc("{\"command\": [\"get_property\", \"time-pos\"]}") {
            if let Some(data) = pos_val.get("data").and_then(|v| v.as_f64()) {
                self.position = data;
            }
        }

        if self.duration <= 0.0 {
            if let Some(dur_val) = self.send_ipc("{\"command\": [\"get_property\", \"duration\"]}") {
                if let Some(data) = dur_val.get("data").and_then(|v| v.as_f64()) {
                    self.duration = data;
                }
            }
        }

        if let Some(pause_val) = self.send_ipc("{\"command\": [\"get_property\", \"pause\"]}") {
            if let Some(data) = pause_val.get("data").and_then(|v| v.as_bool()) {
                self.is_paused = data;
            }
        }
    }
}

impl Drop for MediaPlayer {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
        }
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
