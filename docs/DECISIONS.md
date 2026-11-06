# Architecture Decision Records (ADRs)

This document records the architectural and technology decisions adopted for **kclip**.

---

## ADR-0001: Exclusively Target Umbriel Wayland Compositor

### Context
kclip is a personal productivity tool running on Arch Linux under the Umbriel Wayland compositor. Supporting multiple display servers (X11, GNOME Mutter, KDE KWin, Sway) introduces considerable conditional complexity, disparate protocol bugs, and maintenance overhead.

### Decision
Target only the current Umbriel Wayland compositor environment. Do not include fallbacks, X11 bridges, or abstractions for non-target compositors.

### Consequences
- Radically simplified codebase.
- Direct reliance on protocols verified in Umbriel (`ext-data-control-v1`, `zwlr-data-control-v1`, `xdg_shell`).
- Fast development cycle.

---

## ADR-0002: Wayland Clipboard via Data-Control Protocols

### Context
Standard Wayland clipboard access (`wl_data_device`) restricts reading and writing to focused windows. A background clipboard manager cannot monitor copies made in other applications using standard `wl_data_device`.

### Decision
Implement clipboard monitoring and injection via `ext_data_control_manager_v1` (with fallback to `zwlr_data_control_manager_v1`).

### Consequences
- Full background monitoring without requiring window focus.
- Clean separation from GPUI's internal window clipboard.
- Immediate event detection.

---

## ADR-0003: XDG Toplevel Floating Window with Umbriel Window Rules

### Context
kclip requires a centered, floating, translucent window with rounded corners and backdrop blur. While `zwlr_layer_shell_v1` is supported by Umbriel, GPUI 0.2.2 natively creates `xdg_toplevel` surfaces. Forking GPUI to support layer-shell introduces severe maintenance risk.

### Decision
Use standard GPUI `xdg_toplevel` windows configured with client-side transparency, paired with declarative Umbriel `[[window_rule]]` matching `app_id = "kclip"` for floating mode, centering, and compositor backdrop blur.

### Consequences
- Clean, standard GPUI application code with zero framework forks.
- Leverages Umbriel's built-in dual-kawase blur shader.
- Window placement and sizing is declaratively managed by the compositor.

---

## ADR-0004: Daemon + IPC Architecture for Global Shortcuts

### Context
Umbriel does not implement `org.freedesktop.impl.portal.GlobalShortcuts`. Umbriel global shortcuts trigger commands configured in `config.toml` (e.g. `"Mod+Shift+V" = { action = "spawn kclip toggle" }`). If `kclip` had to start from scratch on every shortcut press, startup latency would be unacceptable.

### Decision
Structure kclip as a persistent background daemon that pre-warms the GPUI window, listening on a local UNIX domain socket (`$XDG_RUNTIME_DIR/kclip.sock`). Running `kclip toggle` sends a fast IPC signal to show or hide the pre-warmed window.

### Consequences
- Toggle response time is instantaneous (< 15 ms).
- Seamless integration with Umbriel keybindings.
- Clipboard monitoring continues uninterrupted in the daemon process.

---

## ADR-0005: Event Debouncing Pipeline (Quiet Window Pattern)

### Context
Screenshot utilities, browsers, and text editors often perform 2–4 writes to the clipboard within a 30–90 ms burst. Capturing eagerly leads to mixed states, race conditions, and wasteful processing.

### Decision
Adopt the `ClipboardEventPipeline` pattern:
- Debounce events with a 100 ms quiet window.
- Expand to a 500 ms burst quiet window upon detecting rapid successive writes.
- Verify selection serial before and after reading to discard corrupted snapshots.

### Consequences
- Zero partial or corrupted records.
- Greatly reduced CPU and database write overhead during heavy copy bursts.

---

## ADR-0006: Hybrid Storage (SQLite + Content-Addressable Files)

### Context
Clipboard history contains millions of small text strings alongside large high-resolution images. Storing multi-megabyte images directly inside SQLite causes database bloat, slow VACUUM, and high memory usage.

### Decision
Store metadata, classifications, and text under 64 KB inline in SQLite. Store payloads exceeding 64 KB and image files in a filesystem blob directory (`~/.local/share/kclip/blobs/<sha256>`).

### Consequences
- SQLite database remains lean and fast (< 50 MB).
- Native zero-copy file passing and fast thumbnail caching.
- Deduplication is naturally hash-based.

---

## ADR-0007: YAML-Only Configuration

### Context
kclip is tailored for personal power-user workflows where graphical settings dialogues add bloat without value.

### Decision
Use a single, strictly validated YAML configuration file (`~/.config/kclip/config.yaml`) with hot-reload support. No graphical settings UI will be developed.

### Consequences
- Zero UI code needed for settings panels.
- Simple, transparent configuration backup and dotfile management.
