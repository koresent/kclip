use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tracing::{debug, info, warn};
use crate::error::IpcError;
use crate::protocol::{IpcCommand, IpcResponse};

pub struct IpcServer {
    socket_path: PathBuf,
    stopped: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl IpcServer {
    pub fn start<F>(socket_path: impl AsRef<Path>, handler: F) -> Result<Self, IpcError>
    where
        F: Fn(IpcCommand) -> IpcResponse + Send + Sync + 'static,
    {
        let path = socket_path.as_ref().to_path_buf();

        // Remove stale socket if present
        if path.exists() {
            let _ = fs::remove_file(&path);
        }

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let listener = UnixListener::bind(&path).map_err(|e| IpcError::Io {
            path: path.clone(),
            source: e,
        })?;

        listener
            .set_nonblocking(true)
            .map_err(|e| IpcError::Io {
                path: path.clone(),
                source: e,
            })?;

        let stopped = Arc::new(AtomicBool::new(false));
        let stopped_clone = stopped.clone();
        let path_clone = path.clone();
        let handler = Arc::new(handler);

        let thread_handle = thread::spawn(move || {
            info!("IPC server listening at {}", path_clone.display());

            while !stopped_clone.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let handler_clone = handler.clone();
                        thread::spawn(move || {
                            Self::handle_connection(stream, handler_clone);
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(30));
                    }
                    Err(e) => {
                        warn!("IPC listener accept error: {}", e);
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            }

            let _ = fs::remove_file(&path_clone);
            info!("IPC server stopped");
        });

        Ok(Self {
            socket_path: path,
            stopped,
            thread_handle: Some(thread_handle),
        })
    }

    fn handle_connection<F>(mut stream: UnixStream, handler: Arc<F>)
    where
        F: Fn(IpcCommand) -> IpcResponse,
    {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(1000)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(1000)));

        let mut reader = BufReader::new(match stream.try_clone() {
            Ok(s) => s,
            Err(_) => return,
        });

        let mut line = String::new();
        if reader.read_line(&mut line).is_ok() && !line.trim().is_empty() {
            let response = match serde_json::from_str::<IpcCommand>(&line) {
                Ok(cmd) => {
                    debug!("Received IPC command: {:?}", cmd);
                    handler(cmd)
                }
                Err(e) => IpcResponse::Error {
                    message: format!("Invalid command format: {}", e),
                },
            };

            if let Ok(resp_json) = serde_json::to_string(&response) {
                let _ = writeln!(stream, "{}", resp_json);
                let _ = stream.flush();
            }
        }
    }

    pub fn stop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        let _ = fs::remove_file(&self.socket_path);
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::IpcClient;
    use tempfile::tempdir;

    #[test]
    fn test_ipc_roundtrip() {
        let dir = tempdir().unwrap();
        let sock_path = dir.path().join("test.sock");

        let mut server = IpcServer::start(&sock_path, |cmd| match cmd {
            IpcCommand::Status => IpcResponse::Status {
                running: true,
                item_count: 42,
            },
            IpcCommand::Toggle => IpcResponse::Ok {
                message: "toggled".to_string(),
            },
            _ => IpcResponse::Ok {
                message: "ok".to_string(),
            },
        })
        .unwrap();

        let resp = IpcClient::send(&sock_path, &IpcCommand::Status, Duration::from_millis(500)).unwrap();
        assert_eq!(
            resp,
            IpcResponse::Status {
                running: true,
                item_count: 42
            }
        );

        let toggle_resp = IpcClient::send(&sock_path, &IpcCommand::Toggle, Duration::from_millis(500)).unwrap();
        assert_eq!(
            toggle_resp,
            IpcResponse::Ok {
                message: "toggled".to_string()
            }
        );

        server.stop();
    }
}
