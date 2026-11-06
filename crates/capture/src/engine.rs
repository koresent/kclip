use crossbeam_channel::{Receiver, Sender, bounded};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use std::collections::HashMap;
use std::io::Write;
use std::os::fd::{AsFd, FromRawFd, IntoRawFd};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tracing::{debug, error, info, warn};
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    backend::ObjectId,
    globals::{GlobalListContents, registry_queue_init},
};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::{self, ZwlrDataControlDeviceV1},
    zwlr_data_control_manager_v1::{self, ZwlrDataControlManagerV1},
    zwlr_data_control_offer_v1::{self, ZwlrDataControlOfferV1},
    zwlr_data_control_source_v1::{self, ZwlrDataControlSourceV1},
};

use crate::error::CaptureError;
use crate::pipe::{create_pipe, read_to_end_with_timeout};
use crate::pipeline::{ClipboardEventPipeline, PipelineConfig};
use crate::types::{RawClipboardSnapshot, SelectionKind};
use content::MimeType;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub capture_primary: bool,
    pub max_item_size_mb: usize,
    pub read_timeout: Duration,
    pub pipeline: PipelineConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            capture_primary: true,
            max_item_size_mb: 50,
            read_timeout: Duration::from_millis(1500),
            pipeline: PipelineConfig::default(),
        }
    }
}

pub struct WaylandClipboardEngine {
    command_tx: Sender<EngineCommand>,
    snapshot_rx: Receiver<RawClipboardSnapshot>,
    stopped: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

enum EngineCommand {
    RestoreSelection {
        kind: SelectionKind,
        representations: HashMap<String, Vec<u8>>,
    },
    Stop,
}

#[derive(Clone)]
struct InternalOfferEvent {
    #[allow(dead_code)]
    sequence: u64,
    kind: SelectionKind,
    offer: ZwlrDataControlOfferV1,
    mime_types: Vec<String>,
}

struct EngineState {
    sequence: Arc<AtomicU64>,
    capture_primary: bool,
    known_offers: HashMap<ObjectId, Vec<String>>,
    current_clipboard_offer: Option<ZwlrDataControlOfferV1>,
    current_primary_offer: Option<ZwlrDataControlOfferV1>,
    active_sources: HashMap<ObjectId, HashMap<String, Vec<u8>>>,
    device: Option<ZwlrDataControlDeviceV1>,
    manager: Option<ZwlrDataControlManagerV1>,
    pipeline: Option<ClipboardEventPipeline<InternalOfferEvent>>,
    conn: Connection,
}

impl WaylandClipboardEngine {
    pub fn spawn(config: EngineConfig) -> Result<Self, CaptureError> {
        let conn = Connection::connect_to_env()?;
        let (command_tx, command_rx) = bounded::<EngineCommand>(16);
        let (out_snapshot_tx, out_snapshot_rx) =
            bounded::<RawClipboardSnapshot>(config.pipeline.snapshot_queue_capacity);
        let stopped = Arc::new(AtomicBool::new(false));
        let stopped_clone = stopped.clone();

        let thread_handle = thread::spawn(move || {
            let (globals, mut event_queue) = match registry_queue_init::<EngineState>(&conn) {
                Ok(res) => res,
                Err(e) => {
                    error!("Failed to initialize Wayland registry queue: {}", e);
                    return;
                }
            };
            let qh = event_queue.handle();

            let seat = match globals.bind::<WlSeat, _, _>(&qh, 1..=9, ()) {
                Ok(s) => s,
                Err(e) => {
                    error!("Compositor offers no wl_seat: {}", e);
                    return;
                }
            };

            let manager = match globals.bind::<ZwlrDataControlManagerV1, _, _>(&qh, 1..=2, ()) {
                Ok(m) => m,
                Err(e) => {
                    error!(
                        "Compositor does not support zwlr_data_control_manager_v1: {}",
                        e
                    );
                    return;
                }
            };

            let device = manager.get_data_device(&seat, &qh, ());
            let sequence = Arc::new(AtomicU64::new(0));

            let seq_clone = sequence.clone();
            let timeout = config.read_timeout;
            let max_bytes = config.max_item_size_mb * 1024 * 1024;
            let conn_for_pipe = conn.clone();

            let pipeline = ClipboardEventPipeline::spawn(
                config.pipeline,
                seq_clone,
                move |event: &InternalOfferEvent, seq| {
                    Self::read_snapshot_internal(&conn_for_pipe, event, seq, timeout, max_bytes)
                },
            );

            let mut state = EngineState {
                sequence,
                capture_primary: config.capture_primary,
                known_offers: HashMap::new(),
                current_clipboard_offer: None,
                current_primary_offer: None,
                active_sources: HashMap::new(),
                device: Some(device),
                manager: Some(manager),
                pipeline: Some(pipeline),
                conn: conn.clone(),
            };

            info!("Wayland clipboard engine initialized and listening");

            while !stopped_clone.load(Ordering::Relaxed) {
                // Check if pipeline emitted debounced snapshots
                if let Some(ref p) = state.pipeline {
                    while let Ok(snap) = p.snapshot_receiver().try_recv() {
                        let _ = out_snapshot_tx.try_send(snap);
                    }
                }

                // Check for incoming commands
                while let Ok(cmd) = command_rx.try_recv() {
                    match cmd {
                        EngineCommand::RestoreSelection {
                            kind,
                            representations,
                        } => {
                            state.restore_selection(&qh, kind, representations);
                        }
                        EngineCommand::Stop => {
                            stopped_clone.store(true, Ordering::Relaxed);
                            break;
                        }
                    }
                }

                if stopped_clone.load(Ordering::Relaxed) {
                    break;
                }

                // Non-blocking flush and timed poll to remain responsive to stop/commands
                let _ = conn.flush();
                if let Some(guard) = conn.prepare_read() {
                    let mut poll_fds = [PollFd::new(
                        &conn,
                        PollFlags::IN | PollFlags::ERR | PollFlags::HUP,
                    )];
                    let ts = Timespec {
                        tv_sec: 0,
                        tv_nsec: 50_000_000, // 50ms poll
                    };
                    match poll(&mut poll_fds, Some(&ts)) {
                        Ok(ready) if ready > 0 => {
                            let _ = guard.read();
                        }
                        _ => {
                            // Times out or cancelled: dropping guard cancels read registration
                        }
                    }
                }

                if let Err(e) = event_queue.dispatch_pending(&mut state) {
                    warn!("Wayland event dispatch error: {}", e);
                    break;
                }
            }

            info!("Wayland clipboard engine stopped");
        });

        Ok(Self {
            command_tx,
            snapshot_rx: out_snapshot_rx,
            stopped,
            thread_handle: Some(thread_handle),
        })
    }

    pub fn snapshot_receiver(&self) -> &Receiver<RawClipboardSnapshot> {
        &self.snapshot_rx
    }

    pub fn restore_selection(
        &self,
        kind: SelectionKind,
        representations: HashMap<String, Vec<u8>>,
    ) {
        let _ = self.command_tx.send(EngineCommand::RestoreSelection {
            kind,
            representations,
        });
    }

    pub fn stop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        let _ = self.command_tx.send(EngineCommand::Stop);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    fn read_snapshot_internal(
        conn: &Connection,
        event: &InternalOfferEvent,
        sequence: u64,
        timeout: Duration,
        max_bytes: usize,
    ) -> Result<Option<RawClipboardSnapshot>, CaptureError> {
        let mut representations = HashMap::new();

        // Read all advertised representations
        for mime_str in &event.mime_types {
            let (read_fd, write_fd) = match create_pipe() {
                Ok(p) => p,
                Err(e) => {
                    warn!("Failed to create pipe for {}: {}", mime_str, e);
                    continue;
                }
            };

            event.offer.receive(mime_str.clone(), write_fd.as_fd());
            drop(write_fd);
            let _ = conn.flush();

            match read_to_end_with_timeout(read_fd, timeout, max_bytes) {
                Ok(bytes) => {
                    representations.insert(MimeType::new(mime_str), bytes);
                }
                Err(e) => {
                    debug!("Failed reading MIME {}: {}", mime_str, e);
                }
            }
        }

        if representations.is_empty() {
            return Ok(None);
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        Ok(Some(RawClipboardSnapshot {
            sequence,
            kind: event.kind,
            representations,
            timestamp,
        }))
    }
}

impl EngineState {
    fn restore_selection(
        &mut self,
        qh: &QueueHandle<Self>,
        kind: SelectionKind,
        representations: HashMap<String, Vec<u8>>,
    ) {
        let (manager, device) = match (&self.manager, &self.device) {
            (Some(m), Some(d)) => (m, d),
            _ => return,
        };

        let source = manager.create_data_source(qh, ());
        for mime in representations.keys() {
            source.offer(mime.clone());
        }

        let source_id = source.id();
        self.active_sources.insert(source_id, representations);

        match kind {
            SelectionKind::Clipboard => {
                device.set_selection(Some(&source));
            }
            SelectionKind::Primary => {
                device.set_primary_selection(Some(&source));
            }
        }
        let _ = self.conn.flush();
    }
}

impl Drop for WaylandClipboardEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for EngineState {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSeat, ()> for EngineState {
    fn event(
        _state: &mut Self,
        _proxy: &WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrDataControlManagerV1, ()> for EngineState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrDataControlManagerV1,
        _event: zwlr_data_control_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrDataControlDeviceV1, ()> for EngineState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrDataControlDeviceV1,
        event: zwlr_data_control_device_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_data_control_device_v1::Event::DataOffer { id } => {
                state.known_offers.insert(id.id(), Vec::new());
            }
            zwlr_data_control_device_v1::Event::Selection { id } => {
                state.current_clipboard_offer = id.clone();
                if let Some(offer) = id {
                    let seq = state.sequence.fetch_add(1, Ordering::Relaxed) + 1;
                    let mimes = state
                        .known_offers
                        .get(&offer.id())
                        .cloned()
                        .unwrap_or_default();
                    if let Some(ref p) = state.pipeline {
                        p.push_event(InternalOfferEvent {
                            sequence: seq,
                            kind: SelectionKind::Clipboard,
                            offer,
                            mime_types: mimes,
                        });
                    }
                }
            }
            zwlr_data_control_device_v1::Event::PrimarySelection { id } => {
                if !state.capture_primary {
                    return;
                }
                state.current_primary_offer = id.clone();
                if let Some(offer) = id {
                    let seq = state.sequence.fetch_add(1, Ordering::Relaxed) + 1;
                    let mimes = state
                        .known_offers
                        .get(&offer.id())
                        .cloned()
                        .unwrap_or_default();
                    if let Some(ref p) = state.pipeline {
                        p.push_event(InternalOfferEvent {
                            sequence: seq,
                            kind: SelectionKind::Primary,
                            offer,
                            mime_types: mimes,
                        });
                    }
                }
            }
            zwlr_data_control_device_v1::Event::Finished => {
                debug!("Data control device finished");
            }
            _ => {}
        }
    }

    wayland_client::event_created_child!(EngineState, ZwlrDataControlDeviceV1, [
        0 => (ZwlrDataControlOfferV1, ()),
    ]);
}

impl Dispatch<ZwlrDataControlOfferV1, ()> for EngineState {
    fn event(
        state: &mut Self,
        proxy: &ZwlrDataControlOfferV1,
        event: zwlr_data_control_offer_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let zwlr_data_control_offer_v1::Event::Offer { mime_type } = event
            && let Some(mimes) = state.known_offers.get_mut(&proxy.id())
        {
            mimes.push(mime_type);
        }
    }
}

impl Dispatch<ZwlrDataControlSourceV1, ()> for EngineState {
    fn event(
        state: &mut Self,
        proxy: &ZwlrDataControlSourceV1,
        event: zwlr_data_control_source_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_data_control_source_v1::Event::Send { mime_type, fd } => {
                let mut file = unsafe { std::fs::File::from_raw_fd(fd.into_raw_fd()) };
                if let Some(contents) = state.active_sources.get(&proxy.id())
                    && let Some(bytes) = contents.get(&mime_type)
                {
                    let _ = file.write_all(bytes);
                }
            }
            zwlr_data_control_source_v1::Event::Cancelled => {
                state.active_sources.remove(&proxy.id());
                proxy.destroy();
            }
            _ => {}
        }
    }
}
