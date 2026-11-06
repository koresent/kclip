use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read configuration file at '{path}': {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("configuration parse error in '{path}': {source}")]
    Parse {
        path: PathBuf,
        source: serde_yaml::Error,
    },

    #[error("configuration parse error: {0}")]
    ParseString(#[from] serde_yaml::Error),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    #[serde(default)]
    pub app: GeneralConfig,

    #[serde(default)]
    pub clipboard: ClipboardConfig,

    #[serde(default)]
    pub storage: StorageConfig,

    #[serde(default)]
    pub ui: UiConfig,

    #[serde(default)]
    pub features: FeaturesConfig,

    #[serde(default)]
    pub security: SecurityConfig,

    #[serde(default)]
    pub logging: LoggingConfig,
}

impl AppConfig {
    pub fn from_yaml(yaml: &str) -> Result<Self, ConfigError> {
        serde_yaml::from_str(yaml).map_err(ConfigError::ParseString)
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        serde_yaml::from_str(&content).map_err(|e| ConfigError::Parse {
            path: path.to_path_buf(),
            source: e,
        })
    }

    pub fn default_config_path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "kclip").map(|dirs| dirs.config_dir().join("config.yaml"))
    }

    pub fn default_data_dir() -> Option<PathBuf> {
        ProjectDirs::from("", "", "kclip").map(|dirs| dirs.data_dir().to_path_buf())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralConfig {
    #[serde(default = "default_true")]
    pub start_hidden: bool,

    #[serde(default)]
    pub socket_path: Option<PathBuf>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            start_hidden: true,
            socket_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClipboardConfig {
    #[serde(default = "default_history_limit")]
    pub history_limit: usize,

    #[serde(default = "default_true")]
    pub ignore_duplicates: bool,

    #[serde(default = "default_true")]
    pub capture_primary_selection: bool,

    #[serde(default = "default_max_item_size_mb")]
    pub max_item_size_mb: usize,

    #[serde(default = "default_debounce_quiet_ms")]
    pub debounce_quiet_ms: u64,

    #[serde(default = "default_debounce_burst_ms")]
    pub debounce_burst_ms: u64,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            history_limit: 10_000,
            ignore_duplicates: true,
            capture_primary_selection: true,
            max_item_size_mb: 50,
            debounce_quiet_ms: 100,
            debounce_burst_ms: 500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StorageConfig {
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,

    #[serde(default = "default_max_storage_mb")]
    pub max_storage_mb: usize,

    #[serde(default)]
    pub db_path: Option<PathBuf>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            retention_days: 90,
            max_storage_mb: 2048,
            db_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UiConfig {
    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default = "default_width")]
    pub width: f32,

    #[serde(default = "default_height")]
    pub height: f32,

    #[serde(default = "default_card_size")]
    pub card_size: f32,

    #[serde(default = "default_card_spacing")]
    pub card_spacing: f32,

    #[serde(default = "default_true")]
    pub backdrop_blur: bool,

    #[serde(default = "default_true")]
    pub show_quick_paste_badges: bool,

    #[serde(default = "default_max_text_lines")]
    pub max_text_lines: usize,

    #[serde(default = "default_animation_duration_ms")]
    pub animation_duration_ms: u64,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            width: 780.0,
            height: 320.0,
            card_size: 200.0,
            card_spacing: 16.0,
            backdrop_blur: true,
            show_quick_paste_badges: true,
            max_text_lines: 6,
            animation_duration_ms: 120,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FeaturesConfig {
    #[serde(default = "default_true")]
    pub syntax_highlighting: bool,

    #[serde(default = "default_true")]
    pub color_preview: bool,

    #[serde(default = "default_true")]
    pub url_cleaner: bool,

    #[serde(default = "default_true")]
    pub thumbnail_generation: bool,
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            syntax_highlighting: true,
            color_preview: true,
            url_cleaner: true,
            thumbnail_generation: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SecurityConfig {
    #[serde(default = "default_true")]
    pub detect_sensitive_content: bool,

    #[serde(default = "default_true")]
    pub exclude_password_managers: bool,

    #[serde(default = "default_ignored_apps")]
    pub ignored_apps: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            detect_sensitive_content: true,
            exclude_password_managers: true,
            ignored_apps: default_ignored_apps(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,

    #[serde(default = "default_true")]
    pub redact_sensitive: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            redact_sensitive: true,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_history_limit() -> usize {
    10_000
}

fn default_max_item_size_mb() -> usize {
    50
}

fn default_debounce_quiet_ms() -> u64 {
    100
}

fn default_debounce_burst_ms() -> u64 {
    500
}

fn default_retention_days() -> u32 {
    90
}

fn default_max_storage_mb() -> usize {
    2048
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_width() -> f32 {
    780.0
}

fn default_height() -> f32 {
    320.0
}

fn default_card_size() -> f32 {
    200.0
}

fn default_card_spacing() -> f32 {
    16.0
}

fn default_max_text_lines() -> usize {
    6
}

fn default_animation_duration_ms() -> u64 {
    120
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_ignored_apps() -> Vec<String> {
    vec![
        "org.keepassxc.KeePassXC".to_string(),
        "bitwarden".to_string(),
        "com.bitwarden.desktop".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_yaml() {
        let yaml = include_str!("../../../config/default.yaml");
        let config = AppConfig::from_yaml(yaml).expect("default.yaml must be valid");
        assert!(config.app.start_hidden);
        assert!(config.clipboard.capture_primary_selection);
        assert_eq!(config.ui.width, 780.0);
        assert_eq!(config.ui.height, 320.0);
        assert_eq!(config.clipboard.history_limit, 10000);
    }

    #[test]
    fn reject_unknown_fields() {
        let invalid_yaml = r#"
app:
  start_hidden: true
  unknown_property: 42
"#;
        let err = AppConfig::from_yaml(invalid_yaml);
        assert!(err.is_err());
    }
}
