# UI Design & GPUI Specification: kclip

This document outlines the user interface architecture, GPUI element hierarchy, typography, colors, layout components, and interaction mechanics designed for **kclip**.

---

## 1. GPUI Windowing & Shell Integration

### 1.1 Window Creation
kclip uses GPUI's `AppContext::open_window` to create an `xdg_toplevel` surface with client-side transparency:
```rust
let window_options = WindowOptions {
    window_bounds: Some(WindowBounds::Fixed(Bounds {
        origin: Point::new(px(0.0), px(0.0)),
        size: Size::new(px(780.0), px(320.0)),
    })),
    titlebar: None, // No server-side titlebar or client-side window buttons
    window_background: WindowBackgroundAppearance::Transparent,
    focus: true,
    show: true,
    kind: WindowKind::PopUp,
    is_movable: false,
    display_id: None,
};
```

### 1.2 Umbriel Window Rule Alignment
In `~/.config/umbriel/config.toml`, Umbriel anchors and floats the window:
```toml
[[window_rule]]
match.app_id = "^kclip$"
default_floating = true
default_floating_size_px = { width = 780, height = 320 }
default_position = { anchor = "center" }
blur = true
opacity = 0.95
corner_radius = 28
border_width = 1
border_color_focused = "#504945"
border_color_unfocused = "#3c3836"
```

---

## 2. Visual System & Color Palette

kclip uses high-contrast surfaces, muted borders, and neutral backgrounds for dark mode:

| Element | Color Value | Description |
|:---|:---|:---|
| **Window Background** | `rgba(28, 28, 30, 0.85)` | Deep translucent charcoal canvas |
| **Window Border** | `rgba(255, 255, 255, 0.10)` | 1 px outer perimeter |
| **Top Bar Divider** | `rgba(255, 255, 255, 0.06)` | 1 px horizontal rule below search |
| **Card Header Surface**| `rgba(56, 56, 61, 0.95)` | Subtle raised grey top section of card |
| **Card Content Surface**| `rgba(43, 43, 46, 0.98)` | Dark surface for card content preview |
| **Active Card Border** | `rgba(255, 255, 255, 0.50)` | 2 px focus ring surrounding selected card |
| **Hover Card Overlay** | `rgba(255, 255, 255, 0.04)` | Subtle brightness increase on pointer hover |
| **Quick Paste Badge** | `rgba(0, 0, 0, 0.88)` / `#FFFFFF` | High-contrast pill in card corner |
| **Primary Text** | `#EDEDED` (`rgba(237, 237, 237, 1.0)`) | Clean white for headings & text |
| **Secondary Text** | `#8E8E93` (`rgba(142, 142, 147, 1.0)`) | Muted text for timestamps & sizes |
| **Accent / Tag Active**| `#B8BB26` or `#E5A040` | Subtle warm accent matching desktop theme |

---

## 3. Component Hierarchy

```text
kclip-main-window (780 x 320 px, rounded 28px)
├── TopBar (height: 44px, padding: 8px 16px)
│   ├── SearchInput (icon, placeholder, text field, clear button)
│   └── FilterChips (All, Text, Links, Images, Files, Code, Colors)
└── HistoryCarousel (horizontal scroll view, height: 260px, padding: 12px 16px)
    ├── LazyHStack (spacing: 16px)
    │   ├── CardSlot (width: 200px, height: 200px)
    │   │   └── ClipCard [Selected: true/false]
    │   │       ├── CardHeader (height: 40px, top rounded 16px)
    │   │       │   ├── Left: SourceAppIcon (20px) + AppName (11pt) + RelativeTime (10pt)
    │   │       │   └── Right: TypeIcon (12pt) + PinnedIcon
    │   │       ├── CardContent (height: 160px, bottom rounded 16px)
    │   │       │   └── [TextCard | CodeCard | ImageCard | UrlCard | ColorCard | FileCard]
    │   │       ├── FocusBorder (2px stroke on active card)
    │   │       └── QuickPasteBadge (bottom-right pill: "#1")
    │   └── ... (additional cards)
    └── EmptyStateView (displayed if history or search result is empty)
```

---

## 4. Content Card Specializations

### 4.1 Plain Text Card
- Typography: 12 pt system font, line height 1.4.
- Max lines: 6 lines, tail truncated.
- Internal padding: 12 px.

### 4.2 Source Code Card
- Typography: 11 pt monospaced font (`JetBrains Mono` or system mono).
- Background: `#1E1E22`.
- Syntax highlighting: Tokenized spans via `syntect` into colored text elements.
- Bottom corner: Small language badge (e.g. `Rust`, `TypeScript`, `Python`, `JSON`).

### 4.3 Image Card
- Thumbnail: Centered, aspect-ratio preserved (`fit: contain`), rounded corners (4 px).
- Overlay badge: Image resolution (`1920 × 1080`) and format (`PNG`, `JPEG`).

### 4.4 URL / Link Card
- Top: Extracted domain name (`github.com`) in bold.
- Middle: Favicon / web icon and page title.
- Bottom: Sanitized URL string in secondary color.

### 4.5 Color Card
- Top 60%: Large solid color block displaying the exact color.
- Bottom 40%: HEX string (`#B8BB26`) in bold, RGB values `rgb(184, 187, 38)` in secondary text.

### 4.6 File List Card
- Top: File icon + primary file name.
- Middle: If multiple files, count pill (e.g. `+ 4 more files`).
- Bottom: Total size (e.g. `14.2 MB`).

---

## 5. Keyboard Navigation & Interaction Details

1. **Auto-focus**: Typing any printable character immediately directs input to the `SearchInput` without requiring explicit mouse clicks.
2. **Arrow Keys & Vim Navigation**:
   - `Left` / `h`: Move selection to the previous card. Automatically scrolls viewport to keep active card visible.
   - `Right` / `l`: Move selection to the next card.
3. **Execution**:
   - `Enter`: Triggers clipboard restoration of selected item, sends `window-focus-warp` or paste key event to target application, and closes kclip.
   - `Shift + Enter`: Restores selected item strictly as plain text (stripping HTML/rich formatting).
   - `Super + 1..9`: Direct quick paste of corresponding card number.
4. **Rich Preview**:
   - `Space`: Toggles attached preview popover showing full content.
5. **Dismissal**:
   - `Esc`: Closes window immediately.
   - Window loses focus (`focus-out`): Closes window.
