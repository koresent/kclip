pub mod engine;
pub mod error;
pub mod pipe;
pub mod pipeline;
pub mod types;

pub use engine::{EngineConfig, WaylandClipboardEngine};
pub use error::CaptureError;
pub use pipeline::{ClipboardEventPipeline, PipelineConfig};
pub use types::{RawClipboardSnapshot, SelectionKind};
