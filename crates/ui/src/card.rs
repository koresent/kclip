use std::path::PathBuf;

use app_core::ContentKind;
use content::ClipboardItem;
use gpui::{
    div, img, px, rgba, svg, Div, ElementId, FontWeight, InteractiveElement, ObjectFit, ParentElement,
    Rgba, Stateful, Styled, StyledImage,
};

use crate::theme::{format_relative_time, rgba_hex, with_alpha, Theme};

pub struct CardView<'a> {
    pub item: &'a ClipboardItem,
    pub is_selected: bool,
    pub quick_paste_num: Option<usize>,
    pub card_index: usize,
    pub elapsed_secs: f32,
    pub is_playing: bool,
    pub is_paused: bool,
    pub media_pos: f64,
    pub media_dur: f64,
    pub text_scroll_line: usize,
}

impl<'a> CardView<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        item: &'a ClipboardItem,
        is_selected: bool,
        quick_paste_num: Option<usize>,
        card_index: usize,
        elapsed_secs: f32,
        is_playing: bool,
        is_paused: bool,
        media_pos: f64,
        media_dur: f64,
        text_scroll_line: usize,
    ) -> Self {
        Self {
            item,
            is_selected,
            quick_paste_num,
            card_index,
            elapsed_secs,
            is_playing,
            is_paused,
            media_pos,
            media_dur,
            text_scroll_line,
        }
    }

    pub fn render(self) -> Stateful<Div> {
        let is_selected = self.is_selected;
        let item = self.item;
        let quick_paste = self.quick_paste_num;

        let border_color = if is_selected {
            Theme::CARD_BORDER_SELECTED
        } else {
            Theme::CARD_BORDER
        };

        let border_width = if is_selected { 2.0 } else { 1.0 };

        div()
            .id(ElementId::NamedInteger("card".into(), self.card_index as u64))
            .w(px(Theme::CARD_WIDTH))
            .h(px(Theme::CARD_HEIGHT))
            .flex()
            .flex_col()
            .flex_shrink_0()
            .cursor_pointer()
            .rounded(px(Theme::CARD_RADIUS))
            .border(px(border_width))
            .border_color(border_color)
            .hover(move |s| {
                if !is_selected {
                    s.border_color(Theme::CARD_BORDER_HOVER)
                } else {
                    s
                }
            })
            .bg(Theme::CARD_CONTENT_BG)
            .overflow_hidden()
            .child(self.render_header(item, quick_paste))
            .child(self.render_body(item))
    }

    fn render_playback_indicator(&self) -> Div {
        if self.is_playing && !self.is_paused {
            let pulse_alpha = 0.35 + 0.65 * (self.elapsed_secs * std::f32::consts::PI * 0.8).sin().abs();
            let pulse_size = 5.0 + 2.0 * (self.elapsed_secs * std::f32::consts::PI * 0.8).sin().abs();
            let alpha_byte = (255.0 * pulse_alpha) as u32;
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(
                    div()
                        .text_size(px(8.5))
                        .font_weight(FontWeight::BOLD)
                        .text_color(Theme::TEXT_PRIMARY)
                        .child("Playing"),
                )
                .child(
                    div()
                        .w(px(10.0))
                        .h(px(10.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .w(px(pulse_size))
                                .h(px(pulse_size))
                                .rounded_full()
                                .bg(rgba(0x0a84ff00 | alpha_byte)),
                        ),
                )
        } else if self.is_playing && self.is_paused {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .child(
                    div()
                        .text_size(px(8.5))
                        .text_color(Theme::TEXT_MUTED)
                        .child("Paused"),
                )
                .child(
                    div()
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded_full()
                        .bg(rgba_hex(0xffffff40)),
                )
        } else {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .child(
                    div()
                        .text_size(px(8.5))
                        .text_color(Theme::TEXT_MUTED)
                        .child("Ready"),
                )
        }
    }

    fn render_header(&self, item: &ClipboardItem, quick_paste: Option<usize>) -> Div {
        let (kind_label, kind_accent) = match item.kind {
            ContentKind::Text => ("TXT", Theme::ACCENT_TEXT),
            ContentKind::Code => ("CODE", Theme::ACCENT_CODE),
            ContentKind::Url => ("URL", Theme::ACCENT_URL),
            ContentKind::Image => ("IMG", Theme::ACCENT_IMAGE),
            ContentKind::Color => ("HEX", Theme::ACCENT_COLOR),
            ContentKind::Files => ("FILE", Theme::ACCENT_FILES),
            ContentKind::Html => ("HTML", Theme::ACCENT_TEXT),
            ContentKind::Binary => ("BIN", Theme::TEXT_MUTED),
            ContentKind::Audio => ("AUD", Theme::ACCENT_AUDIO),
            ContentKind::Video => ("VID", Theme::ACCENT_VIDEO),
        };

        let time_str = format_relative_time(item.created_at);

        let mut header = div()
            .h(px(Theme::CARD_HEADER_HEIGHT))
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(10.0))
            .rounded_t(px(Theme::CARD_RADIUS - 1.0))
            .bg(Theme::CARD_HEADER_BG)
            .border_b_1()
            .border_color(Theme::DIVIDER_COLOR);

        let left_info = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .child(
                div()
                    .px(px(5.0))
                    .py(px(1.5))
                    .rounded(px(4.0))
                    .bg(with_alpha(kind_accent, 0.18))
                    .child(
                        div()
                            .text_size(px(9.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(kind_accent)
                            .child(kind_label),
                    ),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(Theme::TEXT_SECONDARY)
                    .child(time_str),
            );

        header = header.child(left_info);

        if let Some(num) = quick_paste {
            header = header.child(
                div()
                    .px(px(6.0))
                    .py(px(1.5))
                    .rounded(px(4.0))
                    .bg(Theme::BADGE_BG)
                    .border_1()
                    .border_color(Theme::BADGE_BORDER)
                    .child(
                        div()
                            .text_size(px(9.5))
                            .font_weight(FontWeight::BOLD)
                            .text_color(Theme::BADGE_TEXT)
                            .child(format!("{}", num)),
                    ),
            );
        } else if item.is_pinned {
            header = header.child(
                div()
                    .flex()
                    .items_center()
                    .child(
                        svg()
                            .path("icons/star.svg")
                            .size(px(11.0))
                            .text_color(Theme::ACCENT_CODE),
                    ),
            );
        }

        header
    }

    fn render_body(&self, item: &ClipboardItem) -> Div {
        let content_box = div()
            .h(px(Theme::CARD_CONTENT_HEIGHT))
            .w_full()
            .p(px(10.0))
            .rounded_b(px(Theme::CARD_RADIUS - 1.0))
            .flex()
            .flex_col();

        match item.kind {
            ContentKind::Color => {
                let hex_text = item
                    .metadata
                    .preview_text
                    .as_deref()
                    .unwrap_or("#000000");

                let parsed_color = parse_hex_color(hex_text).unwrap_or(rgba(0x000000ff));

                content_box
                    .items_center()
                    .justify_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .w(px(120.0))
                            .h(px(64.0))
                            .rounded(px(8.0))
                            .bg(parsed_color)
                            .border_1()
                            .border_color(rgba(0xffffff20)),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_size(px(13.0))
                            .text_color(Theme::TEXT_PRIMARY)
                            .child(hex_text.to_uppercase()),
                    )
            }
            ContentKind::Url => {
                let url_str = item
                    .metadata
                    .preview_text
                    .as_deref()
                    .unwrap_or("https://...");

                let domain = extract_domain(url_str);
                let title = item.metadata.title.as_deref().unwrap_or(&domain);
                let thumb_path = get_thumbnail_path(&item.hash);

                let display_title = {
                    let clean = title.trim();
                    if clean.chars().count() > 48 {
                        let mut s: String = clean.chars().take(45).collect();
                        s.push_str("...");
                        s
                    } else {
                        clean.to_string()
                    }
                };

                if let Some(path) = thumb_path {
                    content_box
                        .overflow_hidden()
                        .p(px(8.0))
                        .gap(px(6.0))
                        .child(
                            div()
                                .w_full()
                                .h(px(94.0))
                                .flex_shrink_0()
                                .rounded(px(6.0))
                                .overflow_hidden()
                                .bg(rgba_hex(0x00000040))
                                .border_1()
                                .border_color(rgba_hex(0xffffff15))
                                .child(img(path).size_full().object_fit(ObjectFit::Cover)),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .overflow_hidden()
                                .child(
                                    div()
                                        .font_weight(FontWeight::BOLD)
                                        .text_size(px(11.0))
                                        .line_height(px(14.0))
                                        .max_h(px(28.0))
                                        .text_color(Theme::TEXT_PRIMARY)
                                        .overflow_hidden()
                                        .child(display_title),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap(px(4.0))
                                        .child(
                                            svg()
                                                .path("icons/globe.svg")
                                                .size(px(11.0))
                                                .text_color(Theme::ACCENT_URL),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(9.5))
                                                .text_color(Theme::TEXT_SECONDARY)
                                                .child(domain),
                                        ),
                                ),
                        )
                } else {
                    content_box
                        .p(px(10.0))
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .w_full()
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(
                                            svg()
                                                .path("icons/globe.svg")
                                                .size(px(15.0))
                                                .text_color(Theme::ACCENT_URL),
                                        )
                                        .child(
                                            div()
                                                .font_weight(FontWeight::BOLD)
                                                .text_size(px(12.5))
                                                .text_color(Theme::TEXT_PRIMARY)
                                                .child(domain),
                                        ),
                                )
                                .child(
                                    svg()
                                        .path("icons/external.svg")
                                        .size(px(12.0))
                                        .text_color(Theme::TEXT_MUTED),
                                ),
                        )
                        .child(
                            div()
                                .size_full()
                                .p(px(8.0))
                                .rounded(px(6.0))
                                .bg(rgba_hex(0x00000030))
                                .border_1()
                                .border_color(rgba_hex(0xffffff0c))
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .line_height(px(14.0))
                                        .text_color(Theme::TEXT_SECONDARY)
                                        .overflow_hidden()
                                        .child(url_str.to_string()),
                                ),
                        )
                }
            }
            ContentKind::Code => {
                let code_text = item
                    .metadata
                    .preview_text
                    .as_deref()
                    .unwrap_or("// code");

                let raw_lines = wrap_to_lines(code_text, 26);
                let total_lines = raw_lines.len().max(1);
                let visible_count = 8;
                let scroll_line = self.text_scroll_line.min(total_lines.saturating_sub(1));
                let end_line = (scroll_line + visible_count).min(total_lines);
                let slice = raw_lines[scroll_line..end_line].join("\n");

                let thumb_h = ((visible_count as f32 / total_lines as f32).clamp(0.2, 1.0) * 140.0).max(20.0);
                let thumb_top = if total_lines > visible_count {
                    (scroll_line as f32 / (total_lines - visible_count) as f32) * (140.0 - thumb_h)
                } else {
                    0.0
                };

                let has_scrollbar = total_lines > visible_count;
                let text_w = if has_scrollbar { px(150.0) } else { px(164.0) };

                let mut row = div()
                    .size_full()
                    .flex()
                    .flex_row()
                    .gap(px(4.0))
                    .child(
                        div()
                            .w(text_w)
                            .h_full()
                            .whitespace_normal()
                            .text_size(px(10.5))
                            .line_height(px(15.0))
                            .text_color(Theme::TEXT_PRIMARY)
                            .child(slice),
                    );

                if has_scrollbar {
                    row = row.child(
                        div()
                            .w(px(8.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .w(px(3.0))
                                    .h_full()
                                    .rounded_full()
                                    .bg(rgba_hex(0xffffff10))
                                    .relative()
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(thumb_top))
                                            .w_full()
                                            .h(px(thumb_h))
                                            .rounded_full()
                                            .bg(rgba_hex(0xffffff45))
                                            .hover(|s| s.bg(rgba_hex(0xffffff80))),
                                    ),
                            ),
                    );
                }

                content_box
                    .p(px(8.0))
                    .bg(Theme::CODE_BG)
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(rgba(0xffffff08))
                    .child(row)
            }
            ContentKind::Image => {
                let dim_text = match (item.metadata.img_width, item.metadata.img_height) {
                    (Some(w), Some(h)) => format!("{} × {} px", w, h),
                    _ => "Image".to_string(),
                };

                let mut current_frame: Option<PathBuf> = None;
                if let Some(ref extra_json) = item.metadata.extra_json {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(extra_json) {
                        if val.get("animated").and_then(|v| v.as_bool()).unwrap_or(false) {
                            if let (Some(count), Some(dir)) = (
                                val.get("frame_count").and_then(|v| v.as_u64()),
                                val.get("frames_dir").and_then(|v| v.as_str()),
                            ) {
                                if count > 0 {
                                    let idx = ((self.elapsed_secs * 6.0) as usize) % (count as usize);
                                    let frame_p = PathBuf::from(dir).join(format!("frame_{}.png", idx));
                                    if frame_p.exists() {
                                        current_frame = Some(frame_p);
                                    }
                                }
                            }
                        }
                    }
                }

                let image_path = current_frame.or_else(|| get_thumbnail_path(&item.hash))
                    .or_else(|| {
                        item.metadata
                            .preview_text
                            .as_deref()
                            .map(PathBuf::from)
                            .filter(|p| p.exists())
                    });

                if let Some(path) = image_path {
                    content_box
                        .p(px(6.0))
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .w_full()
                                .h(px(124.0))
                                .rounded(px(8.0))
                                .overflow_hidden()
                                .bg(rgba_hex(0x00000040))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    img(path)
                                        .size_full()
                                        .object_fit(ObjectFit::Contain),
                                ),
                        )
                        .child(
                            div()
                                .mt(px(4.0))
                                .text_size(px(10.0))
                                .text_color(Theme::TEXT_SECONDARY)
                                .child(dim_text),
                        )
                } else {
                    content_box
                        .items_center()
                        .justify_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .w(px(54.0))
                                .h(px(54.0))
                                .rounded(px(10.0))
                                .bg(with_alpha(Theme::ACCENT_IMAGE, 0.15))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    svg()
                                        .path("icons/image.svg")
                                        .size(px(24.0))
                                        .text_color(Theme::ACCENT_IMAGE),
                                )
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_size(px(12.0))
                                .text_color(Theme::TEXT_PRIMARY)
                                .child(dim_text),
                        )
                }
            }
            ContentKind::Audio => {
                let meta: Option<processors::MediaMetadata> = item
                    .metadata
                    .extra_json
                    .as_deref()
                    .and_then(|j| serde_json::from_str(j).ok());

                let title = meta.as_ref()
                    .and_then(|m| m.title.as_deref())
                    .or(item.metadata.title.as_deref())
                    .unwrap_or("Audio Track");

                let artist = meta.as_ref()
                    .and_then(|m| m.artist.as_deref())
                    .unwrap_or("Audio File");

                let duration_str = meta.as_ref()
                    .and_then(|m| m.duration_secs)
                    .map(|d| {
                        let mins = (d / 60.0) as u32;
                        let secs = (d % 60.0) as u32;
                        format!("{}:{:02}", mins, secs)
                    })
                    .unwrap_or_else(|| "Audio".to_string());

                let thumb_path = get_thumbnail_path(&item.hash);

                let mut card_body = content_box
                    .p(px(8.0))
                    .gap(px(6.0))
                    .items_center()
                    .justify_center()
                    .child(
                        if let Some(ref path) = thumb_path {
                            div()
                                .w(px(64.0))
                                .h(px(64.0))
                                .rounded(px(8.0))
                                .overflow_hidden()
                                .border_1()
                                .border_color(rgba_hex(0xffffff20))
                                .child(img(path.clone()).size_full().object_fit(ObjectFit::Cover))
                        } else {
                            div()
                                .w(px(64.0))
                                .h(px(64.0))
                                .rounded(px(12.0))
                                .bg(with_alpha(Theme::ACCENT_AUDIO, 0.15))
                                .border_1()
                                .border_color(with_alpha(Theme::ACCENT_AUDIO, 0.3))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    svg()
                                        .path("icons/audio.svg")
                                        .size(px(24.0))
                                        .text_color(Theme::ACCENT_AUDIO),
                                )
                        }
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(1.0))
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_size(px(11.0))
                                    .text_color(Theme::TEXT_PRIMARY)
                                    .overflow_hidden()
                                    .child(title.to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(9.5))
                                    .text_color(Theme::TEXT_SECONDARY)
                                    .child(format!("{} • {}", artist, duration_str)),
                            ),
                    );

                let total_dur = if self.media_dur > 0.0 {
                    self.media_dur
                } else {
                    meta.as_ref().and_then(|m| m.duration_secs).unwrap_or(0.0)
                };

                if total_dur > 0.0 {
                    let progress_frac = if self.is_playing && total_dur > 0.0 {
                        (self.media_pos / total_dur).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let mins_curr = (self.media_pos / 60.0) as u32;
                    let secs_curr = (self.media_pos % 60.0) as u32;
                    let mins_tot = (total_dur / 60.0) as u32;
                    let secs_tot = (total_dur % 60.0) as u32;
                    let time_status = if self.is_playing {
                        format!("{}:{:02} / {}:{:02}", mins_curr, secs_curr, mins_tot, secs_tot)
                    } else {
                        format!("0:00 / {}:{:02}", mins_tot, secs_tot)
                    };

                    card_body = card_body.child(
                        div()
                            .w_full()
                            .mt(px(2.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .w_full()
                                    .h(px(4.0))
                                    .rounded_full()
                                    .bg(rgba_hex(0xffffff20))
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .h_full()
                                            .w(px(progress_frac as f32 * 164.0))
                                            .rounded_full()
                                            .bg(Theme::ACCENT_AUDIO),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .justify_between()
                                    .text_size(px(8.5))
                                    .text_color(Theme::TEXT_SECONDARY)
                                    .child(time_status)
                                    .child(self.render_playback_indicator()),
                            ),
                    );
                }

                card_body
            }
            ContentKind::Video => {
                let meta: Option<processors::MediaMetadata> = item
                    .metadata
                    .extra_json
                    .as_deref()
                    .and_then(|j| serde_json::from_str(j).ok());

                let title = meta.as_ref()
                    .and_then(|m| m.title.as_deref())
                    .or(item.metadata.title.as_deref())
                    .unwrap_or("Video Clip");

                let duration_str = meta.as_ref()
                    .and_then(|m| m.duration_secs)
                    .map(|d| {
                        let mins = (d / 60.0) as u32;
                        let secs = (d % 60.0) as u32;
                        format!("{}:{:02}", mins, secs)
                    })
                    .unwrap_or_else(|| "Video".to_string());

                let res_str = match (item.metadata.img_width, item.metadata.img_height) {
                    (Some(w), Some(h)) => format!("{} × {}", w, h),
                    _ => duration_str.clone(),
                };

                let total_dur = if self.media_dur > 0.0 {
                    self.media_dur
                } else {
                    meta.as_ref().and_then(|m| m.duration_secs).unwrap_or(0.0)
                };

                let mut current_frame: Option<PathBuf> = None;
                if let Some(ref extra_json) = item.metadata.extra_json {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(extra_json) {
                        if let (Some(count), Some(dir)) = (
                            val.get("frame_count").and_then(|v| v.as_u64()),
                            val.get("frames_dir").and_then(|v| v.as_str()),
                        ) {
                            if count > 0 {
                                let idx = if self.is_playing && total_dur > 0.0 {
                                    let frac = (self.media_pos / total_dur).clamp(0.0, 1.0);
                                    ((frac * (count.saturating_sub(1) as f64)).round() as usize).min(count as usize - 1)
                                } else {
                                    0
                                };
                                let frame_p = PathBuf::from(dir).join(format!("frame_{}.png", idx));
                                if frame_p.exists() {
                                    current_frame = Some(frame_p);
                                }
                            }
                        }
                    }
                }

                let thumb_path = current_frame.or_else(|| get_thumbnail_path(&item.hash));

                let mut card_body = content_box
                    .p(px(8.0))
                    .gap(px(4.0))
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w_full()
                            .h(px(if self.is_playing { 84.0 } else { 96.0 }))
                            .rounded(px(8.0))
                            .overflow_hidden()
                            .bg(rgba_hex(0x00000060))
                            .relative()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                if let Some(ref path) = thumb_path {
                                    div()
                                        .size_full()
                                        .child(img(path.clone()).size_full().object_fit(ObjectFit::Cover))
                                } else {
                                    div()
                                        .size_full()
                                        .bg(with_alpha(Theme::ACCENT_VIDEO, 0.15))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            svg()
                                                .path("icons/video.svg")
                                                .size(px(28.0))
                                                .text_color(Theme::ACCENT_VIDEO),
                                        )
                                },
                            )
                            .child(
                                div()
                                    .absolute()
                                    .w(px(32.0))
                                    .h(px(32.0))
                                    .rounded_full()
                                    .bg(rgba_hex(0x00000085))
                                    .border_1()
                                    .border_color(rgba_hex(0xffffff40))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        svg()
                                            .path(if self.is_playing && !self.is_paused { "icons/pause.svg" } else { "icons/play.svg" })
                                            .size(px(14.0))
                                            .text_color(Theme::TEXT_PRIMARY),
                                    ),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .bottom(px(4.0))
                                    .right(px(6.0))
                                    .px(px(4.0))
                                    .py(px(1.0))
                                    .rounded(px(3.0))
                                    .bg(rgba_hex(0x000000b0))
                                    .text_size(px(9.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(Theme::TEXT_PRIMARY)
                                    .child(duration_str),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_size(px(11.0))
                                    .text_color(Theme::TEXT_PRIMARY)
                                    .overflow_hidden()
                                    .child(title.to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(9.5))
                                    .text_color(Theme::TEXT_MUTED)
                                    .child(res_str),
                            ),
                    );

                if total_dur > 0.0 {
                    let progress_frac = if self.is_playing && total_dur > 0.0 {
                        (self.media_pos / total_dur).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let mins_curr = (self.media_pos / 60.0) as u32;
                    let secs_curr = (self.media_pos % 60.0) as u32;
                    let mins_tot = (total_dur / 60.0) as u32;
                    let secs_tot = (total_dur % 60.0) as u32;
                    let time_status = if self.is_playing {
                        format!("{}:{:02} / {}:{:02}", mins_curr, secs_curr, mins_tot, secs_tot)
                    } else {
                        format!("0:00 / {}:{:02}", mins_tot, secs_tot)
                    };

                    card_body = card_body.child(
                        div()
                            .w_full()
                            .mt(px(2.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .w_full()
                                    .h(px(4.0))
                                    .rounded_full()
                                    .bg(rgba_hex(0xffffff20))
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .h_full()
                                            .w(px(progress_frac as f32 * 164.0))
                                            .rounded_full()
                                            .bg(Theme::ACCENT_VIDEO),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .justify_between()
                                    .text_size(px(8.5))
                                    .text_color(Theme::TEXT_SECONDARY)
                                    .child(time_status)
                                    .child(self.render_playback_indicator()),
                            ),
                    );
                }

                card_body
            }
            ContentKind::Files => {
                let path_text = item
                    .metadata
                    .preview_text
                    .as_deref()
                    .unwrap_or("/path/to/file");

                let p = std::path::Path::new(path_text);
                let file_name = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path_text);

                let ext = p.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("FILE")
                    .to_ascii_uppercase();

                let file_size_str = std::fs::metadata(p).ok().map(|m| {
                    let b = m.len();
                    if b < 1024 {
                        format!("{} B", b)
                    } else if b < 1024 * 1024 {
                        format!("{:.1} KB", b as f64 / 1024.0)
                    } else {
                        format!("{:.1} MB", b as f64 / (1024.0 * 1024.0))
                    }
                }).unwrap_or_else(|| "File".to_string());

                content_box
                    .p(px(10.0))
                    .gap(px(6.0))
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w(px(52.0))
                            .h(px(52.0))
                            .rounded(px(10.0))
                            .bg(with_alpha(Theme::ACCENT_FILES, 0.15))
                            .border_1()
                            .border_color(with_alpha(Theme::ACCENT_FILES, 0.35))
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap(px(2.0))
                            .child(
                                svg()
                                    .path("icons/folder.svg")
                                    .size(px(20.0))
                                    .text_color(Theme::ACCENT_FILES),
                            )
                            .child(
                                div()
                                    .text_size(px(8.5))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(Theme::ACCENT_FILES)
                                    .child(ext),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_size(px(11.5))
                                    .text_color(Theme::TEXT_PRIMARY)
                                    .overflow_hidden()
                                    .child(file_name.to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(9.5))
                                    .text_color(Theme::TEXT_MUTED)
                                    .child(file_size_str),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(9.0))
                            .text_color(Theme::TEXT_SECONDARY)
                            .line_height(px(12.0))
                            .overflow_hidden()
                            .child(path_text.to_string()),
                    )
            }
            _ => {
                let text = item.metadata.preview_text.as_deref().unwrap_or("");
                let raw_lines = wrap_to_lines(text, 28);
                let total_lines = raw_lines.len().max(1);
                let visible_count = 9;
                let scroll_line = self.text_scroll_line.min(total_lines.saturating_sub(1));
                let end_line = (scroll_line + visible_count).min(total_lines);
                let slice = raw_lines[scroll_line..end_line].join("\n");

                let thumb_h = ((visible_count as f32 / total_lines as f32).clamp(0.2, 1.0) * 140.0).max(20.0);
                let thumb_top = if total_lines > visible_count {
                    (scroll_line as f32 / (total_lines - visible_count) as f32) * (140.0 - thumb_h)
                } else {
                    0.0
                };

                let has_scrollbar = total_lines > visible_count;
                let text_w = if has_scrollbar { px(150.0) } else { px(164.0) };

                let mut row = div()
                    .id(ElementId::NamedInteger("card-text-container".into(), self.card_index as u64))
                    .size_full()
                    .flex()
                    .flex_row()
                    .gap(px(4.0))
                    .child(
                        div()
                            .w(text_w)
                            .h_full()
                            .whitespace_normal()
                            .text_size(px(11.0))
                            .line_height(px(16.0))
                            .text_color(Theme::TEXT_PRIMARY)
                            .child(slice),
                    );

                if has_scrollbar {
                    row = row.child(
                        div()
                            .w(px(8.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .w(px(3.0))
                                    .h_full()
                                    .rounded_full()
                                    .bg(rgba_hex(0xffffff10))
                                    .relative()
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(thumb_top))
                                            .w_full()
                                            .h(px(thumb_h))
                                            .rounded_full()
                                            .bg(rgba_hex(0xffffff45))
                                            .hover(|s| s.bg(rgba_hex(0xffffff80))),
                                    ),
                            ),
                    );
                }

                content_box
                    .p(px(8.0))
                    .child(row)
            }
        }
    }
}

fn get_thumbnail_path(hash: &str) -> Option<PathBuf> {
    let base = directories::ProjectDirs::from("com", "koresent", "kclip")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("/home/koresent/.local/share/kclip"));
    let p = base.join("thumbnails").join(format!("{}.png", hash));
    if p.exists() {
        Some(p)
    } else {
        let blob = base.join("blobs").join(hash);
        if blob.exists() {
            Some(blob)
        } else {
            None
        }
    }
}

fn parse_hex_color(hex: &str) -> Option<Rgba> {
    let clean = hex.trim().trim_start_matches('#');
    match clean.len() {
        3 => {
            let r = u8::from_str_radix(&clean[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&clean[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&clean[2..3].repeat(2), 16).ok()?;
            Some(rgba(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 0xff))
        }
        6 => {
            let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
            let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
            let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
            Some(rgba(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 0xff))
        }
        8 => {
            let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
            let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
            let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
            let a = u8::from_str_radix(&clean[6..8], 16).ok()?;
            Some(rgba(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)))
        }
        _ => None,
    }
}

fn extract_domain(url_str: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url_str) {
        if let Some(host) = parsed.host_str() {
            return host.to_string();
        }
    }
    url_str.to_string()
}

pub fn wrap_to_lines(text: &str, max_chars_per_line: usize) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            out.push(String::new());
            continue;
        }
        let words: Vec<&str> = line.split(' ').collect();
        let mut cur = String::new();
        for w in words {
            if cur.is_empty() {
                cur.push_str(w);
            } else if cur.chars().count() + 1 + w.chars().count() <= max_chars_per_line {
                cur.push(' ');
                cur.push_str(w);
            } else {
                out.push(cur);
                cur = w.to_string();
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

pub fn get_text_total_lines(item: &ClipboardItem) -> usize {
    let text = item.metadata.preview_text.as_deref().unwrap_or("");
    let max_chars = if item.kind == ContentKind::Code { 26 } else { 28 };
    wrap_to_lines(text, max_chars).len().max(1)
}
