use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

pub struct KclipAssetSource;

const SEARCH_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
  <circle cx="6.5" cy="6.5" r="4.5"/>
  <path d="M10 10l4.2 4.2"/>
</svg>"#;

const CLOSE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
  <path d="M4 4l8 8M12 4l-8 8"/>
</svg>"#;

const PLAY_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
  <polygon points="5,3 13,8 5,13"/>
</svg>"#;

const PAUSE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
  <rect x="4" y="3" width="3" height="10" rx="1"/>
  <rect x="9" y="3" width="3" height="10" rx="1"/>
</svg>"#;

const GLOBE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  <circle cx="8" cy="8" r="6.5"/>
  <line x1="1.5" y1="8" x2="14.5" y2="8"/>
  <ellipse cx="8" cy="8" rx="3.5" ry="6.5"/>
</svg>"#;

const FOLDER_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  <path d="M2 3.5a1.5 1.5 0 011.5-1.5h3.2a1.5 1.5 0 011.06.44L9.3 4h4.2A1.5 1.5 0 0115 5.5v7a1.5 1.5 0 01-1.5 1.5h-11A1.5 1.5 0 011 12.5v-8A1.5 1.5 0 012 3.5z"/>
</svg>"#;

const IMAGE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  <rect x="2" y="2.5" width="12" height="11" rx="2"/>
  <circle cx="5.5" cy="5.5" r="1.2"/>
  <path d="M14 11l-3.5-4-3 3.5L5.5 9 2 12.5"/>
</svg>"#;

const AUDIO_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  <circle cx="4.5" cy="11.5" r="2.5"/>
  <circle cx="11.5" cy="9.5" r="2.5"/>
  <path d="M7 11.5V3.5l7-2V9.5"/>
</svg>"#;

const VIDEO_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  <rect x="1.5" y="3" width="10" height="10" rx="2"/>
  <path d="M11.5 6.5l3-2v7l-3-2"/>
</svg>"#;

const CLIPBOARD_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  <path d="M5.5 2.5h5A1.5 1.5 0 0112 4v9a1.5 1.5 0 01-1.5 1.5h-5A1.5 1.5 0 014 13V4a1.5 1.5 0 011.5-1.5z"/>
  <path d="M6 2a1 1 0 011-1h2a1 1 0 011 1v1H6V2z"/>
</svg>"#;

const STAR_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
  <polygon points="8,1.5 10,6 15,6.5 11.2,10 12.3,15 8,12.3 3.7,15 4.8,10 1,6.5 6,6"/>
</svg>"#;

const EXTERNAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
  <path d="M6 3h7v7M13 3L4 12"/>
</svg>"#;

const LOGO_SVG: &str = include_str!("../../../assets/logo.svg");

impl AssetSource for KclipAssetSource {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match path {
            "icons/logo.svg" => Ok(Some(Cow::Borrowed(LOGO_SVG.as_bytes()))),
            "icons/search.svg" => Ok(Some(Cow::Borrowed(SEARCH_SVG.as_bytes()))),
            "icons/close.svg" => Ok(Some(Cow::Borrowed(CLOSE_SVG.as_bytes()))),
            "icons/play.svg" => Ok(Some(Cow::Borrowed(PLAY_SVG.as_bytes()))),
            "icons/pause.svg" => Ok(Some(Cow::Borrowed(PAUSE_SVG.as_bytes()))),
            "icons/globe.svg" => Ok(Some(Cow::Borrowed(GLOBE_SVG.as_bytes()))),
            "icons/folder.svg" => Ok(Some(Cow::Borrowed(FOLDER_SVG.as_bytes()))),
            "icons/image.svg" => Ok(Some(Cow::Borrowed(IMAGE_SVG.as_bytes()))),
            "icons/audio.svg" => Ok(Some(Cow::Borrowed(AUDIO_SVG.as_bytes()))),
            "icons/video.svg" => Ok(Some(Cow::Borrowed(VIDEO_SVG.as_bytes()))),
            "icons/clipboard.svg" => Ok(Some(Cow::Borrowed(CLIPBOARD_SVG.as_bytes()))),
            "icons/star.svg" => Ok(Some(Cow::Borrowed(STAR_SVG.as_bytes()))),
            "icons/external.svg" => Ok(Some(Cow::Borrowed(EXTERNAL_SVG.as_bytes()))),
            _ => Ok(None),
        }
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(vec![
            "icons/logo.svg".into(),
            "icons/search.svg".into(),
            "icons/close.svg".into(),
            "icons/play.svg".into(),
            "icons/pause.svg".into(),
            "icons/globe.svg".into(),
            "icons/folder.svg".into(),
            "icons/image.svg".into(),
            "icons/audio.svg".into(),
            "icons/video.svg".into(),
            "icons/clipboard.svg".into(),
            "icons/star.svg".into(),
            "icons/external.svg".into(),
        ])
    }
}
