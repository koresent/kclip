#[cfg(test)]
mod tests {
    use crate::theme::format_relative_time;
    use crate::top_bar::FilterKind;
    use app_core::ContentKind;

    #[test]
    fn test_relative_time_formatting() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        assert_eq!(format_relative_time(now), "Just now");
        assert_eq!(format_relative_time(now - 120_000), "2m ago");
        assert_eq!(format_relative_time(now - 7_200_000), "2h ago");
        assert_eq!(format_relative_time(now - 172_800_000), "2d ago");
    }

    #[test]
    fn test_filter_kind_matching() {
        assert!(FilterKind::All.matches(ContentKind::Text));
        assert!(FilterKind::All.matches(ContentKind::Image));
        assert!(FilterKind::All.matches(ContentKind::Url));

        assert!(FilterKind::Text.matches(ContentKind::Text));
        assert!(FilterKind::Text.matches(ContentKind::Html));
        assert!(!FilterKind::Text.matches(ContentKind::Image));

        assert!(FilterKind::Images.matches(ContentKind::Image));
        assert!(!FilterKind::Images.matches(ContentKind::Text));

        assert!(FilterKind::Urls.matches(ContentKind::Url));
        assert!(!FilterKind::Urls.matches(ContentKind::Code));

        assert!(FilterKind::Code.matches(ContentKind::Code));
        assert!(!FilterKind::Code.matches(ContentKind::Color));

        assert!(FilterKind::Colors.matches(ContentKind::Color));
        assert!(!FilterKind::Colors.matches(ContentKind::Files));

        assert!(FilterKind::Files.matches(ContentKind::Files));
        assert!(!FilterKind::Files.matches(ContentKind::Text));

        assert!(FilterKind::Audio.matches(ContentKind::Audio));
        assert!(!FilterKind::Audio.matches(ContentKind::Video));
        assert!(FilterKind::Video.matches(ContentKind::Video));
        assert!(!FilterKind::Video.matches(ContentKind::Audio));
        assert!(!FilterKind::Video.matches(ContentKind::Text));
    }
}
