# Configuration Specification: kclip

This document defines the configuration schema, format, validation rules, and file paths for **kclip**.

---

## 1. Principles & Format

- **Format**: YAML exclusively.
- **No GUI Settings Window**: kclip does not provide a visual preferences pane. All configuration is managed via a single well-commented YAML file.
- **Strict Validation**: Unknown fields or invalid types trigger explicit error diagnostics rather than silent fallbacks.
- **Non-Destructive**: kclip will NEVER automatically overwrite or modify the user's configuration file.
- **Safe Hot Reload**: Modifying the file triggers live reload without terminating or restarting the daemon. Invalid edits preserve the last known good configuration.

---

## 2. File Location

Primary configuration path:
```text
~/.config/kclip/config.yaml
```

Fallback lookup:
1. `$XDG_CONFIG_HOME/kclip/config.yaml`
2. `~/.config/kclip/config.yaml`
3. Embedded default configuration if no file is present on first run.

---

## 3. Configuration Schema & Full Example

```yaml
# kclip configuration file
# Schema version: 1.0

app:
  # Launch kclip minimized/hidden in the background
  start_hidden: true
  
  # UNIX domain socket path for IPC control (defaults to $XDG_RUNTIME_DIR/kclip.sock)
  socket_path: null

clipboard:
  # Maximum number of history items to retain
  history_limit: 10000
  
  # When true, copying identical content bumps the existing item instead of creating a duplicate
  ignore_duplicates: true
  
  # Capture X11/Wayland primary selection (middle-click clipboard)
  capture_primary_selection: false
  
  # Maximum payload size in megabytes before rejecting an item
  max_item_size_mb: 50
  
  # Quiet window (ms) to debounce burst writes from tools like flameshot
  debounce_quiet_ms: 100
  
  # Burst window (ms) when multiple consecutive writes occur
  debounce_burst_ms: 500

storage:
  # Retention duration in days for unpinned items (0 = indefinite)
  retention_days: 90
  
  # Maximum disk storage for binary blobs in megabytes
  max_storage_mb: 2048
  
  # Database path (null uses default ~/.local/share/kclip/kclip.db)
  db_path: null

ui:
  # Theme mode: "dark", "light", or "system"
  theme: "dark"
  
  # Main floating window dimensions
  width: 780
  height: 320
  
  # Card sizing in horizontal carousel
  card_size: 200
  card_spacing: 16
  
  # Enable backdrop blur on supporting compositors (Umbriel window_rule)
  backdrop_blur: true
  
  # Show quick paste badge numbers [1..9] on the first 9 cards
  show_quick_paste_badges: true
  
  # Maximum lines of text displayed on a standard text card
  max_text_lines: 6
  
  # Animation durations in milliseconds
  animation_duration_ms: 120

features:
  # Syntax highlighting for recognized source code snippets
  syntax_highlighting: true
  
  # Automatic color swatch rendering for hex, rgb, and hsl strings
  color_preview: true
  
  # Clean tracking parameters (utm_*, fbclid) from copied URLs
  url_cleaner: true
  
  # Generate and cache image thumbnails
  thumbnail_generation: true

security:
  # Detect sensitive patterns (API keys, JWT, passwords) and mark them
  detect_sensitive_content: true
  
  # Automatically exclude password manager app_ids (e.g. 1password, bitwarden, keepass)
  exclude_password_managers: true
  
  # Blacklist of application app_ids to ignore
  ignored_apps:
    - "org.keepassxc.KeePassXC"
    - "bitwarden"
    - "com.bitwarden.desktop"

logging:
  # Log level: "trace", "debug", "info", "warn", "error"
  level: "info"
  
  # Never log raw clipboard text or image bytes
  redact_sensitive: true
```

---

## 4. Rust Data Structures (Serde)

The configuration maps to strongly typed Rust structs:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub app: GeneralConfig,
    pub clipboard: ClipboardConfig,
    pub storage: StorageConfig,
    pub ui: UiConfig,
    pub features: FeaturesConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
}
```
`#[serde(deny_unknown_fields)]` ensures typos in configuration files fail fast with descriptive field paths.
