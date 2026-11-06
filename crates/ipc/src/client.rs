use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;
use crate::error::IpcError;
use crate::protocol::{IpcCommand, IpcResponse};

pub struct IpcClient;

impl IpcClient {
    pub fn send(
        socket_path: impl AsRef<Path>,
        command: &IpcCommand,
        timeout: Duration,
    ) -> Result<IpcResponse, IpcError> {
        let path = socket_path.as_ref();
        let stream = UnixStream::connect(path).map_err(|e| IpcError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        stream.set_read_timeout(Some(timeout)).map_err(|e| IpcError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        stream.set_write_timeout(Some(timeout)).map_err(|e| IpcError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        let mut writer = stream.try_clone().map_err(|e| IpcError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        let payload = serde_json::to_string(command)?;
        writeln!(writer, "{}", payload).map_err(|e| IpcError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        writer.flush().map_err(|e| IpcError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| IpcError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        if line.is_empty() {
            return Err(IpcError::DaemonError("empty response from daemon".to_string()));
        }

        let response: IpcResponse = serde_json::from_str(&line)?;
        Ok(response)
    }
}
