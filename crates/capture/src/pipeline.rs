use crate::error::CaptureError;
use crate::types::RawClipboardSnapshot;
use crossbeam_channel::{Receiver, Sender, bounded};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub quiet_window: Duration,
    pub burst_quiet_window: Duration,
    pub max_burst_restarts: usize,
    pub snapshot_queue_capacity: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            quiet_window: Duration::from_millis(100),
            burst_quiet_window: Duration::from_millis(500),
            max_burst_restarts: 4,
            snapshot_queue_capacity: 8,
        }
    }
}

pub struct ClipboardEventPipeline<E> {
    event_tx: Sender<E>,
    snapshot_rx: Receiver<RawClipboardSnapshot>,
    stopped: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,
}

impl<E: Send + 'static> ClipboardEventPipeline<E> {
    pub fn spawn<F>(
        config: PipelineConfig,
        sequence_provider: Arc<AtomicU64>,
        mut snapshot_reader: F,
    ) -> Self
    where
        F: FnMut(&E, u64) -> Result<Option<RawClipboardSnapshot>, CaptureError> + Send + 'static,
    {
        let (event_tx, event_rx) = bounded::<E>(16);
        let (snapshot_tx, snapshot_rx) =
            bounded::<RawClipboardSnapshot>(config.snapshot_queue_capacity);
        let snapshot_rx_worker = snapshot_rx.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let stopped_clone = stopped.clone();

        let worker_handle = thread::spawn(move || {
            let mut latest_event: Option<E> = None;
            let mut pending_sequence: u64 = 0;

            while !stopped_clone.load(Ordering::Relaxed) {
                // Phase 1: Wait for an incoming event if we have none pending
                if latest_event.is_none() {
                    match event_rx.recv_timeout(Duration::from_millis(50)) {
                        Ok(ev) => {
                            pending_sequence = sequence_provider.load(Ordering::Relaxed);
                            latest_event = Some(ev);
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                            continue;
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                    }
                }

                // Phase 2: Debounce quiet window
                let mut restarts = 0;
                let mut current_window = config.quiet_window;

                loop {
                    let deadline = Instant::now() + current_window;
                    let timeout = deadline.saturating_duration_since(Instant::now());

                    match event_rx.recv_timeout(timeout) {
                        Ok(newer_event) => {
                            let new_seq = sequence_provider.load(Ordering::Relaxed);
                            debug!(
                                "Clipboard event updated during debounce (seq {} -> {})",
                                pending_sequence, new_seq
                            );
                            latest_event = Some(newer_event);
                            pending_sequence = new_seq;

                            // Escalate to burst quiet window on rapid successive writes
                            if restarts < config.max_burst_restarts {
                                restarts += 1;
                                current_window = config.burst_quiet_window;
                            }
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                            // Quiet window expired without newer writes; proceed to snapshot
                            break;
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                            break;
                        }
                    }
                }

                if stopped_clone.load(Ordering::Relaxed) {
                    break;
                }

                // Phase 3: Capture snapshot and verify sequence consistency
                if let Some(event) = latest_event.take() {
                    let pre_read_seq = pending_sequence;
                    match snapshot_reader(&event, pre_read_seq) {
                        Ok(Some(snapshot)) => {
                            let post_read_seq = sequence_provider.load(Ordering::Relaxed);
                            if post_read_seq != pre_read_seq {
                                warn!(
                                    "Discarding mixed/stale snapshot: sequence changed during read ({} -> {})",
                                    pre_read_seq, post_read_seq
                                );
                            } else {
                                if snapshot_tx.is_full() {
                                    warn!(
                                        "Snapshot queue full; dropping oldest unconsumed snapshot"
                                    );
                                    let _ = snapshot_rx_worker.try_recv();
                                }
                                let _ = snapshot_tx.try_send(snapshot);
                            }
                        }
                        Ok(None) => {}
                        Err(e) => {
                            debug!("Snapshot read aborted: {}", e);
                        }
                    }
                }
            }
        });

        Self {
            event_tx,
            snapshot_rx,
            stopped,
            worker_handle: Some(worker_handle),
        }
    }

    pub fn push_event(&self, event: E) {
        let _ = self.event_tx.try_send(event);
    }
}

impl<E> ClipboardEventPipeline<E> {
    pub fn snapshot_receiver(&self) -> &Receiver<RawClipboardSnapshot> {
        &self.snapshot_rx
    }

    pub fn stop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

impl<E> Drop for ClipboardEventPipeline<E> {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SelectionKind;
    use content::MimeType;
    use std::collections::HashMap;

    #[test]
    fn pipeline_debounces_burst_writes() {
        let seq = Arc::new(AtomicU64::new(0));
        let seq_clone = seq.clone();

        let config = PipelineConfig {
            quiet_window: Duration::from_millis(50),
            burst_quiet_window: Duration::from_millis(80),
            max_burst_restarts: 2,
            snapshot_queue_capacity: 4,
        };

        let pipeline = ClipboardEventPipeline::spawn(config, seq_clone, move |val: &String, s| {
            let mut reps = HashMap::new();
            reps.insert(MimeType::TEXT_PLAIN_UTF8.into(), val.as_bytes().to_vec());
            Ok(Some(RawClipboardSnapshot {
                sequence: s,
                kind: SelectionKind::Clipboard,
                representations: reps,
                timestamp: 1000,
            }))
        });

        // Fire 4 rapid events spaced 10ms apart
        for i in 1..=4 {
            seq.store(i, Ordering::Relaxed);
            pipeline.push_event(format!("val_{}", i));
            thread::sleep(Duration::from_millis(10));
        }

        // Wait for debounce quiet window to pass
        let snapshot = pipeline
            .snapshot_receiver()
            .recv_timeout(Duration::from_millis(300))
            .expect("should receive single debounced snapshot");

        // Exactly the latest value must be received
        assert_eq!(snapshot.sequence, 4);
        let text = snapshot
            .representations
            .get(&MimeType::new(MimeType::TEXT_PLAIN_UTF8))
            .unwrap();
        assert_eq!(text, b"val_4");

        // No subsequent snapshot should be emitted
        assert!(
            pipeline
                .snapshot_receiver()
                .recv_timeout(Duration::from_millis(100))
                .is_err()
        );
    }
}
