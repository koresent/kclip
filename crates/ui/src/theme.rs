use gpui::Rgba;

pub const fn rgba_hex(hex: u32) -> Rgba {
    let r = ((hex >> 24) & 0xff) as f32 / 255.0;
    let g = ((hex >> 16) & 0xff) as f32 / 255.0;
    let b = ((hex >> 8) & 0xff) as f32 / 255.0;
    let a = (hex & 0xff) as f32 / 255.0;
    Rgba { r, g, b, a }
}

pub const fn with_alpha(mut c: Rgba, a: f32) -> Rgba {
    c.a = a;
    c
}

pub struct Theme;

impl Theme {
    // Window surface (translucent with backdrop blur in Umbriel)
    pub const WINDOW_BG: Rgba = rgba_hex(0x16161ad8);
    pub const WINDOW_BORDER: Rgba = rgba_hex(0xffffff18);
    pub const WINDOW_RADIUS: f32 = 24.0;

    // TopBar
    pub const TOP_BAR_HEIGHT: f32 = 44.0;
    pub const DIVIDER_COLOR: Rgba = rgba_hex(0xffffff10);
    pub const SEARCH_BG: Rgba = rgba_hex(0x00000050);
    pub const SEARCH_BORDER: Rgba = rgba_hex(0xffffff18);
    pub const SEARCH_BORDER_FOCUSED: Rgba = rgba_hex(0x0a84ff80);
    pub const SEARCH_TEXT: Rgba = rgba_hex(0xf5f5f7ff);
    pub const SEARCH_PLACEHOLDER: Rgba = rgba_hex(0x8e8e93ff);

    // Filter Chips
    pub const CHIP_ACTIVE_BG: Rgba = rgba_hex(0xffffff28);
    pub const CHIP_HOVER_BG: Rgba = rgba_hex(0xffffff18);
    pub const CHIP_INACTIVE_BG: Rgba = rgba_hex(0xffffff0d);
    pub const CHIP_ACTIVE_TEXT: Rgba = rgba_hex(0xffffffff);
    pub const CHIP_INACTIVE_TEXT: Rgba = rgba_hex(0x98989dff);

    // Cards
    pub const CARD_WIDTH: f32 = 200.0;
    pub const CARD_HEIGHT: f32 = 200.0;
    pub const CARD_HEADER_HEIGHT: f32 = 40.0;
    pub const CARD_CONTENT_HEIGHT: f32 = 160.0;
    pub const CARD_GAP: f32 = 14.0;
    pub const CARD_RADIUS: f32 = 14.0;

    pub const CARD_HEADER_BG: Rgba = rgba_hex(0xffffff0e);
    pub const CARD_CONTENT_BG: Rgba = rgba_hex(0x16161c55);
    pub const CARD_BORDER: Rgba = rgba_hex(0xffffff15);
    pub const CARD_BORDER_HOVER: Rgba = rgba_hex(0xffffff38);
    pub const CARD_BORDER_SELECTED: Rgba = rgba_hex(0x0a84ffee); // Accent blue highlight

    // Typography & Content
    pub const TEXT_PRIMARY: Rgba = rgba_hex(0xf2f2f7ff);
    pub const TEXT_SECONDARY: Rgba = rgba_hex(0x98989dff);
    pub const TEXT_MUTED: Rgba = rgba_hex(0x636366ff);
    pub const CODE_BG: Rgba = rgba_hex(0x00000040);

    // Quick Paste Badge
    pub const BADGE_BG: Rgba = rgba_hex(0xffffff18);
    pub const BADGE_BORDER: Rgba = rgba_hex(0xffffff25);
    pub const BADGE_TEXT: Rgba = rgba_hex(0xffffffdd);

    // Kind Badges / Accents
    pub const ACCENT_TEXT: Rgba = rgba_hex(0x64b4ffff);
    pub const ACCENT_URL: Rgba = rgba_hex(0x34c759ff);
    pub const ACCENT_CODE: Rgba = rgba_hex(0xff9f0aff);
    pub const ACCENT_IMAGE: Rgba = rgba_hex(0xaf52deff);
    pub const ACCENT_COLOR: Rgba = rgba_hex(0xff375fff);
    pub const ACCENT_FILES: Rgba = rgba_hex(0x5ac8faff);
    pub const ACCENT_AUDIO: Rgba = rgba_hex(0xff2d55ff);
    pub const ACCENT_VIDEO: Rgba = rgba_hex(0xbf5af2ff);
}

pub fn format_relative_time(epoch_ms: i64) -> String {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(epoch_ms);

    let diff_secs = (now_ms - epoch_ms).max(0) / 1000;

    if diff_secs < 45 {
        "Just now".to_string()
    } else if diff_secs < 3600 {
        let mins = (diff_secs / 60).max(1);
        format!("{}m ago", mins)
    } else if diff_secs < 86400 {
        let hours = diff_secs / 3600;
        format!("{}h ago", hours)
    } else {
        let days = diff_secs / 86400;
        format!("{}d ago", days)
    }
}
