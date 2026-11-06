# System Architecture: kclip

This document defines the system architecture, subsystem boundaries, data flow, and concurrency model for **kclip**.

---

## 1. Architectural Principles

1. **Strict Subsystem Isolation**:
   - UI (`kclip-ui`) never accesses SQLite directly or handles raw Wayland sockets.
   - Clipboard capture (`kclip-capture`) has zero dependencies on GPUI.
   - Content processors (`kclip-processors`) are pure transformation pipelines without side-effects.
2. **Single-Source Clipboard Engine**:
   - Only `kclip-capture` interacts with `ext-data-control-v1` / `zwlr-data-control-v1`.
3. **Event Debouncing & Pipeline Integrity**:
   - Fast, non-blocking event reception decoupled from slow snapshot data reading and serialization.
4. **Instant UI Responsiveness**:
   - Zero synchronous I/O or database queries on the GPUI rendering thread.
   - Asynchronous pagination, thumbnail caching, and background search indexing.

---

## 2. Workspace & Crate Structure

```text
kclip/
├── Cargo.toml                  # Workspace definition with pinned dependencies
├── README.md
├── config/
│   └── default.yaml            # Canonical default configuration
├── docs/                       # Architectural & design documentation
└── crates/
    ├── app-core/               # Shared domain types, events, and identifiers
    ├── capture/                # Low-level Wayland data-control clipboard monitor
    ├── content/                # Content models, representations, MIME types
    ├── detection/              # Fast heuristic classification (regex, magic bytes, syntax)
    ├── processors/             # Content transformers (image resize, syntax highlight, HTML sanitize)
    ├── storage/                # SQLite connection pool, migrations, queries, blob store
    ├── search/                 # FTS5 full-text indexing, fuzzy search, query parser
    ├── config/                 # YAML configuration parsing, schema validation, defaults
    ├── ipc/                    # UNIX domain socket server/client for daemon control
    └── ui/                     # GPUI application, visual components, keyboard navigation
```

### Dependency Graph

```mermaid
graph TD
    UI[crates/ui (GPUI)] --> Core[crates/app-core]
    UI --> Content[crates/content]
    UI --> Storage[crates/storage]
    UI --> Search[crates/search]
    UI --> Config[crates/config]

    Daemon[crates/app-core (Daemon Main)] --> Capture[crates/capture]
    Daemon --> Detection[crates/detection]
    Daemon --> Processors[crates/processors]
    Daemon --> Storage
    Daemon --> IPC[crates/ipc]
    Daemon --> UI

    Capture --> Content
    Detection --> Content
    Processors --> Content
    Storage --> Content
    Search --> Content
```

---

## 3. Subsystem Responsibilities

### 3.1 `kclip-content`
Defines core data structures:
- `MimeType`: Strongly typed MIME representations (`text/plain`, `text/html`, `image/png`, `text/uri-list`, etc.).
- `ClipboardItem`: Represents a single user copy action, bearing a UUID, timestamp, source application `app_id`, content classification, and collection of representations.
- `ClipboardRepresentation`: Raw byte payload or disk blob pointer for a specific MIME type.
- `ContentKind`: Classification enum (`Text`, `Code`, `Url`, `Image`, `Color`, `Files`, `Html`, `Binary`).

### 3.2 `kclip-capture`
Wayland data-control clipboard monitoring:
- Connects to Wayland display socket (`$WAYLAND_DISPLAY`).
- Binds `ext_data_control_manager_v1` or `zwlr_data_control_manager_v1`.
- Implements `ClipboardEventPipeline`:
  - Debounces rapid/burst writes (100 ms quiet window, 500 ms burst window).
  - Reads advertised MIME types via UNIX pipes asynchronously.
  - Verifies selection serial before and after read to discard corrupted snapshots.
  - Lazily provides clipboard data when kclip itself restores an item to the clipboard.

### 3.3 `kclip-detection` & `kclip-processors`
- **Detection**: Fast, non-blocking heuristics:
  - Color detection: Hex `#RGB`, `#RRGGBB`, `rgb()`, `hsl()`.
  - URL detection: Valid HTTP/HTTPS URLs with domain extraction.
  - Code detection: Syntax patterns (braces, keywords, shebangs).
  - File lists: Parsing `text/uri-list` and validating path existence.
- **Processors**:
  - Image thumbnailing: Downsamples large images to 200×200 pt thumbnails using `image` crate (Lanczos3/Triangle filter).
  - Syntax highlighting: Tokenizes source code using `syntect` into highlighted spans for GPUI text rendering.
  - URL cleaning: Removes UTM and tracking query parameters (`utm_*`, `fbclid`, etc.).

### 3.4 `kclip-storage`
- Embedded SQLite using `rusqlite` (with `bundled` + `fts5` features) or `sqlx`.
- WAL mode (`PRAGMA journal_mode = WAL`) and `PRAGMA synchronous = NORMAL` for high concurrency.
- Blob storage: Keeps small payloads (< 64 KB) inline in SQLite; offloads large files and images to `~/.local/share/kclip/blobs/<sha256>`.
- Deduplication: Bumps existing entries when `(hash, content_kind)` matches.

### 3.5 `kclip-search`
- Backed by SQLite FTS5 with custom unicode61 tokenizer.
- Search prefixes:
  - `kind:image`, `kind:code`, `kind:url`, `app:ghostty`, `date:today`.
- Triggers maintain consistency between `items` table and `items_fts` index.

### 3.6 `kclip-ipc` & `kclip-ui`
- `kclip-ipc`: Standard UNIX domain socket at `$XDG_RUNTIME_DIR/kclip.sock`.
  - Commands: `Toggle`, `Show`, `Hide`, `Paste <id>`, `Clear`, `Quit`.
- `kclip-ui`: GPUI frontend providing the floating card carousel:
  - Floating horizontal window (`xdg_toplevel` with Umbriel window rules).
  - 200×200 pt cards with two-tone header/body layout.
  - Full keyboard control (`Left`/`Right`, `Enter`, `Shift+Enter`, `Super+1..9`, `Space`, `Esc`).
