use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info};

use app_core::ContentKind;
use content::ClipboardItem;
use gpui::{
    div, px, svg, AnyElement, App, AppContext, Bounds, Context, ElementId, FocusHandle, FontWeight,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Point, Render, ScrollDelta, ScrollWheelEvent, Size,
    StatefulInteractiveElement, Styled, Window, WindowBackgroundAppearance, WindowBounds,
    WindowKind, WindowOptions,
};
use kclip_config::AppConfig;
use kclip_ipc::{resolve_socket_path, IpcClient, IpcCommand};
use storage::{BlobStore, Database};

use crate::assets::KclipAssetSource;
use crate::card::CardView;
use std::collections::HashMap;

use crate::launcher::{self, AppChoice};
use crate::player::MediaPlayer;
use crate::theme::Theme;
use crate::top_bar::{render_search_icon, FilterKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Search,
    Help,
    OpenWith,
}

pub const CHIP_GAP: f32 = 6.0;

pub fn chip_width(filter: FilterKind) -> f32 {
    match filter {
        FilterKind::All => 34.0,
        FilterKind::Text => 40.0,
        FilterKind::Code => 42.0,
        FilterKind::Urls => 44.0,
        FilterKind::Images => 54.0,
        FilterKind::Audio => 46.0,
        FilterKind::Video => 46.0,
        FilterKind::Files => 44.0,
        FilterKind::Colors => 52.0,
    }
}

fn get_chip_layout(filter: FilterKind) -> (f32, f32) {
    let mut x = 0.0;
    for f in FilterKind::ALL {
        let w = chip_width(f);
        if f == filter {
            return (x, w);
        }
        x += w + CHIP_GAP;
    }
    (0.0, 34.0)
}

#[derive(Debug, Clone)]
pub struct ScrollbarDragState {
    pub hash: String,
    pub total_lines: usize,
    pub drag_start_y: f32,
    pub start_scroll_line: usize,
}

pub struct KclipAppView {
    db: Arc<Mutex<Database>>,
    socket_path: PathBuf,
    search_query: String,
    active_filter: FilterKind,
    selected_index: usize,
    scroll_offset_index: usize,
    scroll_x: f32,
    target_scroll_x: f32,
    wheel_accumulator: f32,
    chip_pill_x: f32,
    chip_pill_w: f32,
    text_scroll_lines: HashMap<String, usize>,
    scrollbar_drag: Option<ScrollbarDragState>,
    media_scrub_drag: Option<usize>,
    was_dragging_scrollbar: bool,
    items: Vec<ClipboardItem>,
    focus_handle: FocusHandle,
    start_time: std::time::Instant,
    mode: AppMode,
    media_player: MediaPlayer,
    open_with_apps: Vec<AppChoice>,
    open_with_selected: usize,
}

impl KclipAppView {
    pub fn new(
        db: Arc<Mutex<Database>>,
        socket_path: PathBuf,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let (init_pill_x, init_pill_w) = get_chip_layout(FilterKind::All);
        let mut view = Self {
            db,
            socket_path,
            search_query: String::new(),
            active_filter: FilterKind::All,
            selected_index: 0,
            scroll_offset_index: 0,
            scroll_x: 0.0,
            target_scroll_x: 0.0,
            wheel_accumulator: 0.0,
            chip_pill_x: init_pill_x,
            chip_pill_w: init_pill_w,
            text_scroll_lines: HashMap::new(),
            scrollbar_drag: None,
            media_scrub_drag: None,
            was_dragging_scrollbar: false,
            items: Vec::new(),
            focus_handle,
            start_time: std::time::Instant::now(),
            mode: AppMode::Normal,
            media_player: MediaPlayer::new(),
            open_with_apps: Vec::new(),
            open_with_selected: 0,
        };
        view.refresh_items();
        view
    }

    pub fn set_filter(&mut self, filter: FilterKind, cx: &mut Context<Self>) {
        if self.active_filter != filter {
            self.active_filter = filter;
            self.selected_index = 0;
            self.scroll_offset_index = 0;
            self.update_target_scroll();
            self.refresh_items();
            cx.notify();
        }
    }

    pub fn cycle_filter(&mut self, delta: isize, cx: &mut Context<Self>) {
        let current_idx = FilterKind::ALL
            .iter()
            .position(|f| *f == self.active_filter)
            .unwrap_or(0) as isize;
        let total = FilterKind::ALL.len() as isize;
        let next_idx = ((current_idx + delta).rem_euclid(total)) as usize;
        self.set_filter(FilterKind::ALL[next_idx], cx);
    }

    fn update_target_scroll(&mut self) {
        let card_pitch = Theme::CARD_WIDTH + Theme::CARD_GAP;
        self.target_scroll_x = (self.scroll_offset_index as f32) * card_pitch;
    }

    pub fn refresh_items(&mut self) {
        let loaded_items = {
            let db_lock = match self.db.lock() {
                Ok(db) => db,
                Err(e) => {
                    error!("Failed to lock database: {}", e);
                    return;
                }
            };

            if self.search_query.trim().is_empty() {
                let all = db_lock.list_items(30, 0).unwrap_or_default();
                if self.active_filter == FilterKind::All {
                    all
                } else {
                    all.into_iter()
                        .filter(|item| self.active_filter.matches(item.kind))
                        .collect()
                }
            } else {
                let results = db_lock.search_items(&self.search_query, 30).unwrap_or_default();
                if self.active_filter == FilterKind::All {
                    results
                } else {
                    results
                        .into_iter()
                        .filter(|item| self.active_filter.matches(item.kind))
                        .collect()
                }
            }
        };

        self.items = loaded_items;
        if self.selected_index >= self.items.len() {
            self.selected_index = self.items.len().saturating_sub(1);
        }
        if self.selected_index < self.scroll_offset_index {
            self.scroll_offset_index = self.selected_index;
        }
        self.update_target_scroll();
    }

    fn select_and_paste(&mut self, index: usize, _window: &mut Window, cx: &mut App) {
        self.media_player.stop();
        if let Some(item) = self.items.get(index) {
            if let Some(id) = item.id {
                info!("Selecting and pasting item #{}", id);
                match IpcClient::send(&self.socket_path, &IpcCommand::Paste { id }, Duration::from_millis(500)) {
                    Ok(resp) => info!("Paste response: {:?}", resp),
                    Err(e) => error!("Failed to send Paste command via IPC: {}", e),
                }
            }
        }
        cx.quit();
    }

    fn toggle_media(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(item) = self.items.get(index) {
            if item.kind == ContentKind::Audio || item.kind == ContentKind::Video {
                if let Some(path) = launcher::resolve_or_dump_item(item) {
                    self.media_player.play(&path, item.hash.clone());
                    cx.notify();
                }
            }
        }
    }

    fn open_xdg_default(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(item) = self.items.get(index) {
            let _ = launcher::open_item_default(item);
            cx.notify();
        }
    }

    fn open_with_menu(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(item) = self.items.get(index) {
            self.open_with_apps = launcher::get_registered_apps(item);
            self.open_with_selected = 0;
            self.mode = AppMode::OpenWith;
            cx.notify();
        }
    }

    fn delete_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(item) = self.items.get(self.selected_index) {
            if let Some(id) = item.id {
                if let Ok(mut db) = self.db.lock() {
                    let _ = db.delete_item(id);
                }
                self.refresh_items();
                cx.notify();
            }
        }
    }

    fn scroll_left(&mut self, cx: &mut Context<Self>) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            if self.selected_index < self.scroll_offset_index {
                self.scroll_offset_index = self.selected_index;
            }
            self.update_target_scroll();
            cx.notify();
        }
    }

    fn scroll_right(&mut self, cx: &mut Context<Self>) {
        if self.selected_index + 1 < self.items.len() {
            self.selected_index += 1;
            if self.selected_index >= self.scroll_offset_index + 3 {
                self.scroll_offset_index = self.selected_index.saturating_sub(2);
            }
            self.update_target_scroll();
            cx.notify();
        }
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();

        match self.mode {
            AppMode::Help => {
                if key == "escape" || key == "?" {
                    self.mode = AppMode::Normal;
                    cx.notify();
                }
            }
            AppMode::OpenWith => {
                match key {
                    "escape" => {
                        self.mode = AppMode::Normal;
                        cx.notify();
                    }
                    "j" | "down" => {
                        if !self.open_with_apps.is_empty() {
                            self.open_with_selected = (self.open_with_selected + 1) % self.open_with_apps.len();
                            cx.notify();
                        }
                    }
                    "k" | "up" => {
                        if !self.open_with_apps.is_empty() {
                            if self.open_with_selected == 0 {
                                self.open_with_selected = self.open_with_apps.len() - 1;
                            } else {
                                self.open_with_selected -= 1;
                            }
                            cx.notify();
                        }
                    }
                    "enter" => {
                        if let Some(app) = self.open_with_apps.get(self.open_with_selected) {
                            if let Some(item) = self.items.get(self.selected_index) {
                                let _ = launcher::open_item_with_app(item, &app.desktop_id);
                            }
                        }
                        self.mode = AppMode::Normal;
                        cx.quit();
                    }
                    _ => {}
                }
            }
            AppMode::Search => {
                match key {
                    "escape" => {
                        self.mode = AppMode::Normal;
                        cx.notify();
                    }
                    "enter" => {
                        if !self.items.is_empty() {
                            self.select_and_paste(self.selected_index, window, cx);
                        }
                    }
                    "tab" => {
                        let delta = if event.keystroke.modifiers.shift { -1 } else { 1 };
                        self.cycle_filter(delta, cx);
                    }
                    "backtab" => {
                        self.cycle_filter(-1, cx);
                    }
                    "backspace" => {
                        if !self.search_query.is_empty() {
                            self.search_query.pop();
                            self.selected_index = 0;
                            self.scroll_offset_index = 0;
                            self.refresh_items();
                            cx.notify();
                        }
                    }
                    _ => {
                        if let Some(ref char_str) = event.keystroke.key_char {
                            if !event.keystroke.modifiers.platform
                                && !event.keystroke.modifiers.control
                                && !char_str.chars().any(|c| c.is_control())
                            {
                                self.search_query.push_str(char_str);
                                self.selected_index = 0;
                                self.scroll_offset_index = 0;
                                self.refresh_items();
                                cx.notify();
                            }
                        }
                    }
                }
            }
            AppMode::Normal => {
                match key {
                    "escape" => {
                        self.media_player.stop();
                        cx.quit();
                    }
                    "enter" => {
                        if !self.items.is_empty() {
                            self.select_and_paste(self.selected_index, window, cx);
                        }
                    }
                    // VIM navigation
                    "h" | "left" => {
                        self.scroll_left(cx);
                    }
                    "l" | "right" => {
                        self.scroll_right(cx);
                    }
                    "j" | "down" => {
                        self.scroll_right(cx);
                    }
                    "k" | "up" => {
                        self.scroll_left(cx);
                    }
                    "0" | "^" => {
                        self.selected_index = 0;
                        self.scroll_offset_index = 0;
                        self.update_target_scroll();
                        cx.notify();
                    }
                    "$" => {
                        self.selected_index = self.items.len().saturating_sub(1);
                        self.scroll_offset_index = self.selected_index.saturating_sub(2);
                        self.update_target_scroll();
                        cx.notify();
                    }
                    "w" => {
                        // Jump forward 3 cards
                        for _ in 0..3 {
                            self.scroll_right(cx);
                        }
                    }
                    "b" => {
                        // Jump backward 3 cards
                        for _ in 0..3 {
                            self.scroll_left(cx);
                        }
                    }
                    // Search mode activation
                    "/" | "i" => {
                        self.mode = AppMode::Search;
                        cx.notify();
                    }
                    // Help cheatsheet
                    "?" => {
                        self.mode = AppMode::Help;
                        cx.notify();
                    }
                    // Media controls
                    "space" | "p" => {
                        self.toggle_media(self.selected_index, cx);
                    }
                    "s" => {
                        self.media_player.stop();
                        cx.notify();
                    }
                    "[" => {
                        self.media_player.seek_relative(-5.0);
                        cx.notify();
                    }
                    "]" => {
                        self.media_player.seek_relative(5.0);
                        cx.notify();
                    }
                    // Open with XDG default
                    "o" if !event.keystroke.modifiers.shift => {
                        self.open_xdg_default(self.selected_index, cx);
                    }
                    // Open With custom app menu
                    "O" | "a" | "o" if event.keystroke.modifiers.shift => {
                        self.open_with_menu(self.selected_index, cx);
                    }
                    // Delete item
                    "x" | "d" => {
                        self.delete_selected(cx);
                    }
                    // Cycle filters
                    "tab" => {
                        let delta = if event.keystroke.modifiers.shift { -1 } else { 1 };
                        self.cycle_filter(delta, cx);
                    }
                    "backtab" => {
                        self.cycle_filter(-1, cx);
                    }
                    "H" => {
                        self.cycle_filter(-1, cx);
                    }
                    "L" => {
                        self.cycle_filter(1, cx);
                    }
                    // Scroll text preview on current card
                    "J" => {
                        if let Some(item) = self.items.get(self.selected_index) {
                            let entry = self.text_scroll_lines.entry(item.hash.clone()).or_insert(0);
                            *entry = entry.saturating_add(3);
                            cx.notify();
                        }
                    }
                    "K" => {
                        if let Some(item) = self.items.get(self.selected_index) {
                            let entry = self.text_scroll_lines.entry(item.hash.clone()).or_insert(0);
                            *entry = entry.saturating_sub(3);
                            cx.notify();
                        }
                    }
                    // Alt+1..9 direct filter jump
                    k if event.keystroke.modifiers.alt
                        && k.len() == 1
                        && k.chars().next().unwrap().is_ascii_digit() =>
                    {
                        let digit = k.parse::<usize>().unwrap_or(0);
                        if digit >= 1 && digit <= FilterKind::ALL.len() {
                            self.set_filter(FilterKind::ALL[digit - 1], cx);
                        }
                    }
                    // Number quick-paste
                    k if k.len() == 1
                        && k.chars().next().unwrap().is_ascii_digit()
                        && k != "0" =>
                    {
                        let digit = k.parse::<usize>().unwrap_or(0);
                        if digit >= 1 && digit <= 9 && digit - 1 < self.items.len() {
                            self.select_and_paste(digit - 1, window, cx);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn render_top_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_focused = self.mode == AppMode::Search;
        let border_color = if is_focused {
            Theme::SEARCH_BORDER_FOCUSED
        } else {
            Theme::SEARCH_BORDER
        };

        let mut search_box = div()
            .id("search-input")
            .cursor_text()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(10.0))
            .h(px(28.0))
            .w(px(220.0))
            .rounded(px(14.0))
            .bg(Theme::SEARCH_BG)
            .border_1()
            .border_color(border_color)
            .on_click(cx.listener(|this, _, _, cx| {
                this.mode = AppMode::Search;
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(render_search_icon())
                    .child(
                        if self.search_query.is_empty() {
                            let mut text_box = div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(2.0));
                            if is_focused {
                                let elapsed = self.start_time.elapsed().as_secs_f32();
                                let cursor_alpha = 0.25 + 0.75 * (elapsed * std::f32::consts::PI).sin().abs();
                                text_box = text_box.child(
                                    div()
                                        .w(px(1.5))
                                        .h(px(13.0))
                                        .rounded_full()
                                        .bg(Theme::ACCENT_TEXT)
                                        .opacity(cursor_alpha),
                                );
                            } else {
                                text_box = text_box.child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(Theme::SEARCH_PLACEHOLDER)
                                        .child("Search history..."),
                                );
                            }
                            text_box
                        } else {
                            let mut text_box = div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(Theme::SEARCH_TEXT)
                                        .child(self.search_query.clone()),
                                );
                            if is_focused {
                                let elapsed = self.start_time.elapsed().as_secs_f32();
                                let cursor_alpha = 0.25 + 0.75 * (elapsed * std::f32::consts::PI).sin().abs();
                                text_box = text_box.child(
                                    div()
                                        .w(px(1.5))
                                        .h(px(13.0))
                                        .ml(px(1.0))
                                        .rounded_full()
                                        .bg(Theme::ACCENT_TEXT)
                                        .opacity(cursor_alpha),
                                );
                            }
                            text_box
                        },
                    ),
            );

        if !self.search_query.is_empty() {
            search_box = search_box.child(
                div()
                    .id("clear-search-btn")
                    .cursor_pointer()
                    .p(px(2.0))
                    .rounded_full()
                    .hover(|s| s.bg(gpui::rgba(0xffffff20)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.search_query.clear();
                        this.selected_index = 0;
                        this.scroll_offset_index = 0;
                        this.refresh_items();
                        cx.notify();
                    }))
                    .child(
                        svg()
                            .path("icons/close.svg")
                            .size(px(12.0))
                            .text_color(Theme::TEXT_MUTED),
                    ),
            );
        }

        let mut chips_row = div()
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(CHIP_GAP))
            .child(
                div()
                    .absolute()
                    .left(px(self.chip_pill_x))
                    .top(px(0.0))
                    .w(px(self.chip_pill_w))
                    .h(px(22.0))
                    .rounded(px(11.0))
                    .bg(Theme::CHIP_ACTIVE_BG),
            );

        for filter in FilterKind::ALL {
            let is_active = filter == self.active_filter;
            let (text_color, weight) = if is_active {
                (Theme::CHIP_ACTIVE_TEXT, FontWeight::SEMIBOLD)
            } else {
                (Theme::CHIP_INACTIVE_TEXT, FontWeight::NORMAL)
            };
            let chip_w = chip_width(filter);

            chips_row = chips_row.child(
                div()
                    .id(ElementId::Name(filter.label().into()))
                    .cursor_pointer()
                    .w(px(chip_w))
                    .h(px(22.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(11.0))
                    .hover(move |s| {
                        if !is_active {
                            s.bg(Theme::CHIP_HOVER_BG)
                        } else {
                            s
                        }
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_filter(filter, cx);
                    }))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(weight)
                            .text_color(text_color)
                            .child(filter.label()),
                    ),
            );
        }

        let help_btn = div()
            .id("help-modal-trigger")
            .cursor_pointer()
            .w(px(22.0))
            .h(px(22.0))
            .rounded_full()
            .bg(if self.mode == AppMode::Help { Theme::CHIP_ACTIVE_BG } else { gpui::rgba(0xffffff0e) })
            .hover(|s| s.bg(gpui::rgba(0xffffff20)))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(11.0))
            .font_weight(FontWeight::BOLD)
            .text_color(if self.mode == AppMode::Help { Theme::CHIP_ACTIVE_TEXT } else { Theme::TEXT_MUTED })
            .on_click(cx.listener(|this, _, _, cx| {
                this.mode = if this.mode == AppMode::Help { AppMode::Normal } else { AppMode::Help };
                cx.notify();
            }))
            .child("?");

        let right_controls = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(10.0))
            .child(chips_row)
            .child(help_btn);

        let left_section = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(10.0))
            .child(
                svg()
                    .path("icons/logo.svg")
                    .size(px(20.0))
            )
            .child(search_box);

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
            .child(left_section)
            .child(right_controls)
    }

    fn render_carousel(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.items.is_empty() {
            return div()
                .h(px(Theme::CARD_HEIGHT + 32.0))
                .w_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .child(
                    svg()
                        .path("icons/clipboard.svg")
                        .size(px(32.0))
                        .text_color(Theme::TEXT_MUTED),
                )
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_size(px(13.0))
                        .text_color(Theme::TEXT_SECONDARY)
                        .child("No clipboard history found"),
                )
                .into_any_element();
        }

        let is_media_playing = self.media_player.is_playing;
        let media_pos = self.media_player.position;
        let media_dur = self.media_player.duration;

        let mut inner_track = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(Theme::CARD_GAP))
            .ml(px(-self.scroll_x));

        for (actual_idx, item) in self.items.iter().enumerate() {
            let is_selected = actual_idx == self.selected_index;
            let quick_paste_num = if actual_idx < 9 {
                Some(actual_idx + 1)
            } else {
                None
            };

            let is_this_playing = is_media_playing
                && self.media_player.current_item_hash.as_deref() == Some(&item.hash);

            let text_scroll = self.text_scroll_lines.get(&item.hash).copied().unwrap_or(0);
            let elapsed_secs = self.start_time.elapsed().as_secs_f32();
            let card_el = CardView::new(
                item,
                is_selected,
                quick_paste_num,
                actual_idx,
                elapsed_secs,
                is_this_playing,
                self.media_player.is_paused,
                media_pos,
                media_dur,
                text_scroll,
            )
            .render()
            // LMB: Select & Paste (unless dragging scrollbar)
            .on_click(cx.listener(move |this, _, window, cx| {
                if this.was_dragging_scrollbar {
                    this.was_dragging_scrollbar = false;
                    return;
                }
                this.select_and_paste(actual_idx, window, cx);
            }))
            // LMB down on scrollbar track: initiate drag & seek
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                if let Some(item) = this.items.get(actual_idx) {
                    if item.kind == ContentKind::Text || item.kind == ContentKind::Code || item.kind == ContentKind::Html {
                        let card_x = 16.0 - this.scroll_x + (actual_idx as f32) * (Theme::CARD_WIDTH + Theme::CARD_GAP);
                        let rel_x = f32::from(event.position.x) - card_x;
                        let rel_y = f32::from(event.position.y);
                        if rel_x >= 148.0 && rel_x <= 180.0 && rel_y >= 70.0 && rel_y <= 240.0 {
                            let total_lines = crate::card::get_text_total_lines(item);
                            if total_lines > 8 {
                                let start_scroll_line = this.text_scroll_lines.get(&item.hash).copied().unwrap_or(0);
                                this.scrollbar_drag = Some(ScrollbarDragState {
                                    hash: item.hash.clone(),
                                    total_lines,
                                    drag_start_y: rel_y,
                                    start_scroll_line,
                                });
                                this.was_dragging_scrollbar = true;
                                cx.notify();
                            }
                        }
                    }
                }
            }))
            // RMB: drag text scrollbar, scrub media, or toggle playback
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                if let Some(item) = this.items.get(actual_idx) {
                    let card_x = 16.0 - this.scroll_x + (actual_idx as f32) * (Theme::CARD_WIDTH + Theme::CARD_GAP);
                    let rel_x = f32::from(event.position.x) - card_x;
                    let rel_y = f32::from(event.position.y);

                    if item.kind == ContentKind::Text || item.kind == ContentKind::Code || item.kind == ContentKind::Html {
                        if rel_x >= 148.0 && rel_x <= 180.0 && rel_y >= 70.0 && rel_y <= 240.0 {
                            let total_lines = crate::card::get_text_total_lines(item);
                            if total_lines > 8 {
                                let start_scroll_line = this.text_scroll_lines.get(&item.hash).copied().unwrap_or(0);
                                this.scrollbar_drag = Some(ScrollbarDragState {
                                    hash: item.hash.clone(),
                                    total_lines,
                                    drag_start_y: rel_y,
                                    start_scroll_line,
                                });
                                this.was_dragging_scrollbar = true;
                                cx.notify();
                            }
                        }
                    } else if item.kind == ContentKind::Audio || item.kind == ContentKind::Video {
                        let is_on_progress = rel_y >= 195.0;
                        if is_on_progress {
                            this.media_scrub_drag = Some(actual_idx);
                            let progress_rel_x = rel_x - 8.0;
                            let pct = ((progress_rel_x / 164.0) as f64 * 100.0).clamp(0.0, 100.0);
                            let is_this_item = this.media_player.current_item_hash.as_deref() == Some(&item.hash);
                            if this.media_player.is_playing && is_this_item {
                                this.media_player.seek_percent(pct);
                            } else if let Some(path) = launcher::resolve_or_dump_item(item) {
                                this.media_player.play_at(&path, item.hash.clone(), pct);
                            }
                            cx.notify();
                        } else {
                            this.toggle_media(actual_idx, cx);
                        }
                    }
                }
            }))
            // MMB (Scroll wheel click): Open default XDG / Shift+MMB: Open With
            .on_mouse_down(MouseButton::Middle, cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                if event.modifiers.shift {
                    this.open_with_menu(actual_idx, cx);
                } else {
                    this.open_xdg_default(actual_idx, cx);
                }
            }));

            inner_track = inner_track.child(card_el);
        }

        div()
            .id("cards-carousel")
            .h(px(Theme::CARD_HEIGHT + 32.0))
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .px(px(16.0))
            .overflow_hidden()
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                let delta = match event.delta {
                    ScrollDelta::Pixels(d) => {
                        let dx = f32::from(d.x);
                        let dy = f32::from(d.y);
                        if dx.abs() > dy.abs() {
                            dx
                        } else {
                            dy
                        }
                    }
                    ScrollDelta::Lines(d) => {
                        if d.x.abs() > d.y.abs() {
                            d.x * 25.0
                        } else {
                            d.y * 25.0
                        }
                    }
                };

                this.wheel_accumulator += delta;
                let threshold = 35.0;
                if this.wheel_accumulator >= threshold {
                    this.wheel_accumulator = 0.0;
                    this.scroll_right(cx);
                } else if this.wheel_accumulator <= -threshold {
                    this.wheel_accumulator = 0.0;
                    this.scroll_left(cx);
                }
            }))
            .child(inner_track)
            .into_any_element()
    }

    fn render_help_overlay(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .bg(gpui::rgba(0x000000cc))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .p(px(20.0))
            .child(
                div()
                    .w(px(520.0))
                    .p(px(18.0))
                    .rounded(px(16.0))
                    .bg(gpui::rgba(0x18181ff8))
                    .border_1()
                    .border_color(gpui::rgba(0xffffff20))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        svg()
                                            .path("icons/logo.svg")
                                            .size(px(20.0))
                                    )
                                    .child(
                                        div()
                                            .font_weight(FontWeight::BOLD)
                                            .text_size(px(14.0))
                                            .text_color(Theme::TEXT_PRIMARY)
                                            .child("kclip - Controls & Shortcuts"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(Theme::TEXT_MUTED)
                                    .child("Press ? or Esc to close"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(24.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.0))
                                    .child(self.help_row("h / l / j / k", "Navigate cards"))
                                    .child(self.help_row("Tab / S-Tab / H / L", "Cycle filter chips"))
                                    .child(self.help_row("Alt + 1..9", "Jump to filter"))
                                    .child(self.help_row("J / K", "Scroll text in card"))
                                    .child(self.help_row("/ or i", "Search mode"))
                                    .child(self.help_row("Enter", "Paste selected item")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.0))
                                    .child(self.help_row("Space / p", "Play / Pause media"))
                                    .child(self.help_row("s", "Stop media playback"))
                                    .child(self.help_row("[ / ]", "Seek -5s / +5s"))
                                    .child(self.help_row("RMB Drag", "Seek progress / Scroll text"))
                                    .child(self.help_row("o / SCM", "Open file (xdg-open)"))
                                    .child(self.help_row("O / Shift+SCM", "Open With... custom app"))
                                    .child(self.help_row("x / d", "Delete from history")),
                            ),
                    ),
            )
    }

    fn render_open_with_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list_col = div().flex().flex_col().gap(px(4.0));

        for (idx, app) in self.open_with_apps.iter().enumerate() {
            let is_sel = idx == self.open_with_selected;
            let bg = if is_sel { gpui::rgba(0x0a84ffff) } else { gpui::rgba(0xffffff10) };
            let text_color = if is_sel { gpui::rgba(0xffffffff) } else { Theme::TEXT_PRIMARY };
            let app_id = app.desktop_id.clone();

            list_col = list_col.child(
                div()
                    .id(ElementId::NamedInteger("app-choice".into(), idx as u64))
                    .cursor_pointer()
                    .px(px(12.0))
                    .py(px(6.0))
                    .rounded(px(8.0))
                    .bg(bg)
                    .hover(move |s| if !is_sel { s.bg(gpui::rgba(0xffffff20)) } else { s })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(item) = this.items.get(this.selected_index) {
                            let _ = launcher::open_item_with_app(item, &app_id);
                        }
                        this.mode = AppMode::Normal;
                        cx.quit();
                    }))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(if is_sel { FontWeight::BOLD } else { FontWeight::NORMAL })
                            .text_color(text_color)
                            .child(app.display_name.clone()),
                    ),
            );
        }

        div()
            .absolute()
            .inset_0()
            .bg(gpui::rgba(0x000000cc))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .p(px(20.0))
            .child(
                div()
                    .w(px(340.0))
                    .p(px(16.0))
                    .rounded(px(14.0))
                    .bg(gpui::rgba(0x1a1a22fa))
                    .border_1()
                    .border_color(gpui::rgba(0xffffff20))
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_size(px(13.0))
                                    .text_color(Theme::TEXT_PRIMARY)
                                    .child("Open with application..."),
                            )
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(Theme::TEXT_MUTED)
                                    .child("Esc to cancel"),
                            ),
                    )
                    .child(list_col),
            )
    }

    fn help_row(&self, key: &'static str, desc: &'static str) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .px(px(6.0))
                    .py(px(2.0))
                    .rounded(px(4.0))
                    .bg(gpui::rgba(0xffffff15))
                    .border_1()
                    .border_color(gpui::rgba(0xffffff25))
                    .text_size(px(10.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(Theme::TEXT_PRIMARY)
                    .child(key),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(Theme::TEXT_SECONDARY)
                    .child(desc),
            )
    }
}

impl Render for KclipAppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Ensure focus is retained on window for instantaneous keyboard control
        window.focus(&self.focus_handle);

        // Animate carousel scroll
        let diff = self.target_scroll_x - self.scroll_x;
        if diff.abs() > 0.5 {
            self.scroll_x += diff * 0.28;
            window.request_animation_frame();
        } else {
            self.scroll_x = self.target_scroll_x;
        }

        // Animate filter chips pill
        let (target_pill_x, target_pill_w) = get_chip_layout(self.active_filter);
        let diff_pill_x = target_pill_x - self.chip_pill_x;
        let diff_pill_w = target_pill_w - self.chip_pill_w;
        if diff_pill_x.abs() > 0.5 || diff_pill_w.abs() > 0.5 {
            self.chip_pill_x += diff_pill_x * 0.28;
            self.chip_pill_w += diff_pill_w * 0.28;
            window.request_animation_frame();
        } else {
            self.chip_pill_x = target_pill_x;
            self.chip_pill_w = target_pill_w;
        }

        if self.media_player.is_playing {
            if self.media_scrub_drag.is_none() {
                self.media_player.update_status();
            }
            window.request_animation_frame();
        }

        if self.mode == AppMode::Search || self.scrollbar_drag.is_some() || self.media_scrub_drag.is_some() {
            window.request_animation_frame();
        }

        let mut root = div()
            .key_context("KclipApp")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if let Some(drag) = &this.scrollbar_drag {
                    this.was_dragging_scrollbar = true;
                    let current_y = f32::from(event.position.y);
                    let dy = current_y - drag.drag_start_y;
                    let max_scroll = drag.total_lines.saturating_sub(8) as f32;
                    if max_scroll > 0.0 {
                        let delta_lines = dy * (max_scroll / 130.0);
                        let target = (drag.start_scroll_line as f32 + delta_lines)
                            .round()
                            .clamp(0.0, max_scroll) as usize;
                        this.text_scroll_lines.insert(drag.hash.clone(), target);
                        cx.notify();
                    }
                } else if let Some(media_idx) = this.media_scrub_drag {
                    let card_x = 16.0 - this.scroll_x + (media_idx as f32) * (Theme::CARD_WIDTH + Theme::CARD_GAP);
                    let rel_x = f32::from(event.position.x) - card_x - 8.0;
                    let pct = ((rel_x / 164.0) as f64 * 100.0).clamp(0.0, 100.0);
                    this.media_player.seek_percent(pct);
                    cx.notify();
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                if this.scrollbar_drag.is_some() || this.media_scrub_drag.is_some() {
                    this.scrollbar_drag = None;
                    this.media_scrub_drag = None;
                    cx.notify();
                }
            }))
            .on_mouse_up(MouseButton::Right, cx.listener(|this, event: &MouseUpEvent, _, cx| {
                if let Some(media_idx) = this.media_scrub_drag.take() {
                    let card_x = 16.0 - this.scroll_x + (media_idx as f32) * (Theme::CARD_WIDTH + Theme::CARD_GAP);
                    let rel_x = f32::from(event.position.x) - card_x - 8.0;
                    let pct = ((rel_x / 164.0) as f64 * 100.0).clamp(0.0, 100.0);
                    this.media_player.seek_percent(pct);
                    cx.notify();
                }
                if this.scrollbar_drag.is_some() {
                    this.scrollbar_drag = None;
                    cx.notify();
                }
            }))
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(Theme::WINDOW_BG)
            .child(self.render_top_bar(cx))
            .child(self.render_carousel(cx));

        if self.mode == AppMode::Help {
            root = root.child(self.render_help_overlay(cx));
        } else if self.mode == AppMode::OpenWith {
            root = root.child(self.render_open_with_overlay(cx));
        }

        root
    }
}

pub fn launch_ui(config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = config
        .storage
        .db_path
        .as_ref()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .or_else(AppConfig::default_data_dir)
        .unwrap_or_else(|| PathBuf::from("/tmp/kclip-data"));

    let db_path = config
        .storage
        .db_path
        .clone()
        .unwrap_or_else(|| data_dir.join("kclip.db"));
    let blobs_dir = data_dir.join("blobs");
    let socket_path = resolve_socket_path(config.app.socket_path.as_ref());

    let blob_store = BlobStore::new(&blobs_dir)?;
    let db = Arc::new(Mutex::new(Database::open(&db_path, blob_store)?));

    let app = gpui::Application::new().with_assets(KclipAssetSource);

    app.run(move |cx: &mut App| {
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::new(px(0.0), px(0.0)),
                size: Size {
                    width: px(840.0),
                    height: px(Theme::TOP_BAR_HEIGHT + Theme::CARD_HEIGHT + 32.0),
                },
            })),
            titlebar: None,
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            is_resizable: false,
            is_minimizable: false,
            display_id: None,
            window_background: WindowBackgroundAppearance::Transparent,
            app_id: Some("kclip".to_string()),
            window_min_size: Some(Size {
                width: px(600.0),
                height: px(280.0),
            }),
            ..Default::default()
        };

        let db_clone = db.clone();
        let socket_clone = socket_path.clone();

        let _window = cx.open_window(window_options, move |_window, cx| {
            cx.new(|cx| KclipAppView::new(db_clone, socket_clone, cx))
        });
    });

    Ok(())
}
