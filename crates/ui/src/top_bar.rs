use app_core::ContentKind;
use gpui::{div, px, svg, Div, FontWeight, ParentElement, Styled, Svg};

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    All,
    Text,
    Code,
    Urls,
    Images,
    Audio,
    Video,
    Files,
    Colors,
}

impl FilterKind {
    pub const ALL: [FilterKind; 9] = [
        FilterKind::All,
        FilterKind::Text,
        FilterKind::Code,
        FilterKind::Urls,
        FilterKind::Images,
        FilterKind::Audio,
        FilterKind::Video,
        FilterKind::Files,
        FilterKind::Colors,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            FilterKind::All => "All",
            FilterKind::Text => "Text",
            FilterKind::Code => "Code",
            FilterKind::Urls => "Links",
            FilterKind::Images => "Images",
            FilterKind::Audio => "Audio",
            FilterKind::Video => "Video",
            FilterKind::Files => "Files",
            FilterKind::Colors => "Colors",
        }
    }

    pub fn matches(&self, kind: ContentKind) -> bool {
        match self {
            FilterKind::All => true,
            FilterKind::Text => kind == ContentKind::Text || kind == ContentKind::Html,
            FilterKind::Code => kind == ContentKind::Code,
            FilterKind::Urls => kind == ContentKind::Url,
            FilterKind::Images => kind == ContentKind::Image,
            FilterKind::Audio => kind == ContentKind::Audio,
            FilterKind::Video => kind == ContentKind::Video,
            FilterKind::Files => kind == ContentKind::Files,
            FilterKind::Colors => kind == ContentKind::Color,
        }
    }

    pub fn to_content_kind(&self) -> Option<ContentKind> {
        match self {
            FilterKind::All => None,
            FilterKind::Text => Some(ContentKind::Text),
            FilterKind::Code => Some(ContentKind::Code),
            FilterKind::Urls => Some(ContentKind::Url),
            FilterKind::Images => Some(ContentKind::Image),
            FilterKind::Audio => Some(ContentKind::Audio),
            FilterKind::Video => Some(ContentKind::Video),
            FilterKind::Files => Some(ContentKind::Files),
            FilterKind::Colors => Some(ContentKind::Color),
        }
    }
}

pub fn render_search_icon() -> Svg {
    svg()
        .path("icons/search.svg")
        .w(px(14.0))
        .h(px(14.0))
        .text_color(Theme::TEXT_SECONDARY)
}

pub struct TopBarView<'a> {
    pub query: &'a str,
    pub active_filter: FilterKind,
    pub is_searching: bool,
}

impl<'a> TopBarView<'a> {
    pub fn new(query: &'a str, active_filter: FilterKind, is_searching: bool) -> Self {
        Self {
            query,
            active_filter,
            is_searching,
        }
    }

    pub fn render(self) -> Div {
        div()
            .h(px(Theme::TOP_BAR_HEIGHT))
            .w_full()
            .px(px(16.0))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(Theme::DIVIDER_COLOR)
            .child(self.render_search_bar())
            .child(self.render_filter_chips())
    }

    fn render_search_bar(&self) -> Div {
        let border_color = if self.is_searching {
            Theme::SEARCH_BORDER_FOCUSED
        } else {
            Theme::SEARCH_BORDER
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .px(px(10.0))
            .h(px(28.0))
            .w(px(220.0))
            .rounded(px(14.0))
            .bg(Theme::SEARCH_BG)
            .border_1()
            .border_color(border_color)
            .child(render_search_icon())
            .child(
                if self.query.is_empty() {
                    let mut text_box = div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(2.0));
                    if self.is_searching {
                        text_box = text_box.child(
                            div()
                                .w(px(1.5))
                                .h(px(13.0))
                                .rounded_full()
                                .bg(Theme::ACCENT_TEXT),
                        );
                    }
                    text_box.child(
                        div()
                            .text_size(px(11.5))
                            .text_color(Theme::SEARCH_PLACEHOLDER)
                            .child("Search history..."),
                    )
                } else {
                    let mut text_box = div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(Theme::SEARCH_TEXT)
                                .child(self.query.to_string()),
                        );
                    if self.is_searching {
                        text_box = text_box.child(
                            div()
                                .w(px(1.5))
                                .h(px(13.0))
                                .ml(px(1.0))
                                .rounded_full()
                                .bg(Theme::ACCENT_TEXT),
                        );
                    }
                    text_box
                },
            )
    }

    fn render_filter_chips(&self) -> Div {
        let mut chips = div().flex().flex_row().items_center().gap(px(6.0));

        for filter in FilterKind::ALL {
            let is_active = filter == self.active_filter;
            let (bg, text_color, weight) = if is_active {
                (
                    Theme::CHIP_ACTIVE_BG,
                    Theme::CHIP_ACTIVE_TEXT,
                    FontWeight::SEMIBOLD,
                )
            } else {
                (
                    Theme::CHIP_INACTIVE_BG,
                    Theme::CHIP_INACTIVE_TEXT,
                    FontWeight::NORMAL,
                )
            };

            chips = chips.child(
                div()
                    .px(px(9.0))
                    .py(px(3.0))
                    .rounded(px(10.0))
                    .bg(bg)
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(weight)
                            .text_color(text_color)
                            .child(filter.label()),
                    ),
            );
        }

        chips
    }
}
