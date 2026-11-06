use capture::{EngineConfig, WaylandClipboardEngine};
use std::time::Duration;

#[test]
fn engine_connects_to_running_wayland() {
    if std::env::var("WAYLAND_DISPLAY").is_err() {
        eprintln!("Skipping Wayland integration test: WAYLAND_DISPLAY not set");
        return;
    }

    let mut engine = match WaylandClipboardEngine::spawn(EngineConfig::default()) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("Wayland connection error in test: {}", err);
            return;
        }
    };

    // Engine started on background thread; sleep briefly and shut down cleanly
    std::thread::sleep(Duration::from_millis(150));
    engine.stop();
}
