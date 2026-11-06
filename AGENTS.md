# AGENTS.md

Practical reference for AI coding agents working on `kclip`.

## 1. Documentation Index

Detailed architectural specs and design documents live in `docs/`:

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): Multi-crate structure, dependency graph, subsystem breakdown.
- [docs/UI_DESIGN.md](docs/UI_DESIGN.md): GPUI components, card sizing, layout dimensions, theme tokens.
- [docs/DATA_MODEL.md](docs/DATA_MODEL.md): SQLite schema, FTS5 triggers, blob storage layout, MIME classification.
- [docs/DECISIONS.md](docs/DECISIONS.md): Architectural decisions (capture debouncing, headless `mpv` IPC, floating windowing).
- [docs/CONFIGURATION.md](docs/CONFIGURATION.md): YAML configuration format, schema validation, defaults.
- [docs/PERFORMANCE.md](docs/PERFORMANCE.md): Latency budgets, memory footprint, capture throughput benchmarks.

## 2. Repository Layout

```
crates/
├── app-core/       # Daemon orchestration, media scrub IPC, background workflows
├── capture/        # Wayland data-control event loop (ext-data-control-v1 / zwlr)
├── cli/            # Main binary entry point (`kclip`)
├── config/         # YAML config parsing & schema
├── content/        # Data models (ClipboardItem, ContentKind, MIME)
├── detection/      # Fast content classifiers (URL, code, color, file list)
├── ipc/            # UNIX domain socket server/client protocol
├── processors/     # Thumbnail generation, syntax highlighting, URL cleaner
├── storage/        # SQLite connection pool, FTS5 search, disk blob store
└── ui/             # GPUI desktop interface and card renderers
```

## 3. Development Commands

```bash
# Run tests
cargo test

# Build release binary
cargo build --release

# Local service restart
systemctl --user stop kclip.service
cp target/release/kclip ~/.local/bin/kclip
systemctl --user start kclip.service

# UI control
kclip toggle
kclip show
kclip hide
```

## 4. Key Implementation Rules

- **No Panics in Daemon Paths**: Never call `.unwrap()` or `.expect()` on clipboard pipe I/O or IPC socket streams; propagate errors with `?`.
- **Pure GPUI Thread**: Never execute blocking I/O, database writes, or system commands (`mpv`, `ffmpeg`) inside GPUI `render()` methods. Dispatch via `cx.spawn()` or background threads.
- **Text Bounds in UI**: GPUI does not automatically clip long text inside cards without explicit truncation or container bounds. Always bound text strings before rendering.
- **Transactions**: Wrap multi-step database mutations (item insertion, representations, FTS updates) in explicit SQLite transactions.
