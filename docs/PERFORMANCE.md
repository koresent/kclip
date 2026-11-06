# Performance & Scalability Targets: kclip

This document outlines the performance requirements, memory budgets, latency constraints, and optimization strategies for **kclip**.

---

## 1. Target Metrics & Budgets

| Metric | Target Budget | Rationale |
|:---|:---|:---|
| **Memory Footprint (Idle)** | < 35 MB RSS | Native Rust without JVM/Electron runtime |
| **Memory Footprint (Active UI)**| < 75 MB RSS | Including GPUI GPU textures & cached thumbnails |
| **Window Toggle Latency** | < 15 ms | Pre-warmed window toggled via IPC command |
| **Clipboard Capture Latency** | < 5 ms (unblocked) | Asynchronous read pipe; writer never stalls |
| **Database Query Latency** | < 2 ms (indexed) | Indexed queries with SQLite WAL mode |
| **Full-Text Search Latency** | < 10 ms (FTS5) | Sub-10ms response for 10,000 items |
| **Scroll Rendering Frame Rate** | 60–165 FPS (vsync) | Fluid 165Hz scrolling matching target display |

---

## 2. Asynchronous Architecture & Concurrency

### 2.1 Non-Blocking UI Thread
- The GPUI main loop runs on the main thread and MUST NOT perform disk I/O, network requests, or synchronous database transactions.
- Database access occurs via an asynchronous worker channel or thread pool.
- Items are loaded into UI memory lazily in pages of 30 items.

### 2.2 Clipboard Pipe Offloading
- Wayland `receive(mime_type, fd)` yields a file descriptor.
- Reading this descriptor is performed on an asynchronous background task.
- If a target application provides data slowly (e.g. huge image or network mount), only the background task waits; neither the Wayland event loop nor the UI is impacted.

---

## 3. Thumbnail & Asset Caching

### 3.1 Thumbnail Generation Gate
- Image decoding and downsampling (to 200×200 pt) is CPU-intensive.
- Downsampling uses a bounded worker concurrency gate (max 2 concurrent decodes) to prevent CPU spikes when multiple images are captured.
- Generated thumbnails are saved to disk once (`~/.local/share/kclip/thumbnails/<sha256>.png`) and cached in an in-memory LRU cache (`max_entries = 128`, memory cap 32 MB).

### 3.2 Content Deferral
- Plain text previews are truncated to 500 characters for initial card rendering.
- Full text payloads are only retrieved when the user triggers the Space preview or pastes the item.

---

## 4. Benchmarking & Verification Plan

1. **Capture Latency Benchmark**:
   - Measure elapsed time from `selection` event to completion of deduplication and insertion.
2. **Burst Resistance Benchmark**:
   - Simulate 50 back-to-back clipboard writes spaced 20 ms apart (mimicking Snipping Tool/Flameshot).
   - Validate that exactly 1 canonical snapshot is captured and 0 partial/corrupted records exist.
3. **FTS Search Benchmark**:
   - Populate database with 10,000 realistic code snippets, URLs, and articles.
   - Benchmark query execution time for single-word, multi-word, and prefix searches (`app:ghostty query*`).
4. **Memory Stability Test**:
   - Run kclip for 48 hours under continuous desktop usage; monitor RSS to verify zero leaks.
