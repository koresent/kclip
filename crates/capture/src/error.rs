use thiserror::Error;

#[derive(Error, Debug)]
pub enum CaptureError {
    #[error("failed to connect to Wayland display: {0}")]
    WaylandConnect(#[from] wayland_client::ConnectError),

    #[error("failed to initialize Wayland globals or queue: {0}")]
    GlobalInit(#[from] wayland_client::globals::GlobalError),

    #[error("Wayland dispatch error: {0}")]
    Dispatch(#[from] wayland_client::DispatchError),

    #[error("Wayland compositor does not support data-control protocol")]
    ProtocolUnavailable,

    #[error("compositor offers no wl_seat global")]
    NoSeat,

    #[error("I/O error during clipboard pipe operation: {0}")]
    Io(#[from] std::io::Error),

    #[error("system call error: {0}")]
    System(#[from] rustix::io::Errno),

    #[error("pipe read timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("payload exceeded maximum size limit of {0} bytes")]
    PayloadTooLarge(usize),

    #[error("selection became stale during read")]
    StaleSelection,
}
