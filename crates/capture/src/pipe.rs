use crate::error::CaptureError;
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::pipe::{PipeFlags, pipe_with};
use std::io::Read;
use std::os::fd::{FromRawFd, OwnedFd};
use std::time::Duration;

pub fn create_pipe() -> Result<(OwnedFd, OwnedFd), CaptureError> {
    let (read_fd, write_fd) = pipe_with(PipeFlags::CLOEXEC)?;
    Ok((read_fd, write_fd))
}

pub fn read_to_end_with_timeout(
    read_fd: OwnedFd,
    timeout: Duration,
    max_bytes: usize,
) -> Result<Vec<u8>, CaptureError> {
    let mut file =
        unsafe { std::fs::File::from_raw_fd(std::os::fd::IntoRawFd::into_raw_fd(read_fd)) };
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];

    let start = std::time::Instant::now();

    loop {
        let elapsed = start.elapsed();
        if elapsed >= timeout {
            return Err(CaptureError::Timeout(timeout));
        }

        let remaining = timeout - elapsed;
        let ts = Timespec {
            tv_sec: remaining.as_secs() as _,
            tv_nsec: remaining.subsec_nanos() as _,
        };

        let mut poll_fds = [PollFd::new(
            &file,
            PollFlags::IN | PollFlags::HUP | PollFlags::ERR,
        )];
        let ready = poll(&mut poll_fds, Some(&ts))?;

        if ready == 0 {
            return Err(CaptureError::Timeout(timeout));
        }

        let revents = poll_fds[0].revents();
        if revents.contains(PollFlags::ERR) {
            return Err(CaptureError::Io(std::io::Error::other("pipe poll error")));
        }

        if revents.contains(PollFlags::IN) {
            let n = match file.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => n,
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(CaptureError::Io(e)),
            };

            if buffer.len() + n > max_bytes {
                return Err(CaptureError::PayloadTooLarge(max_bytes));
            }

            buffer.extend_from_slice(&chunk[..n]);
        } else if revents.contains(PollFlags::HUP) {
            match file.read_to_end(&mut buffer) {
                Ok(n) if buffer.len() + n > max_bytes => {
                    return Err(CaptureError::PayloadTooLarge(max_bytes));
                }
                Ok(_) => break,
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(CaptureError::Io(e)),
            }
        }
    }

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::fd::FromRawFd;

    #[test]
    fn pipe_write_and_read() {
        let (read_fd, write_fd) = create_pipe().unwrap();
        let mut writer =
            unsafe { std::fs::File::from_raw_fd(std::os::fd::IntoRawFd::into_raw_fd(write_fd)) };

        std::thread::spawn(move || {
            writer.write_all(b"clipboard test bytes").unwrap();
        });

        let data = read_to_end_with_timeout(read_fd, Duration::from_millis(500), 1024).unwrap();
        assert_eq!(data, b"clipboard test bytes");
    }

    #[test]
    fn pipe_timeout_on_idle() {
        let (read_fd, _write_fd) = create_pipe().unwrap();
        let res = read_to_end_with_timeout(read_fd, Duration::from_millis(50), 1024);
        assert!(matches!(res, Err(CaptureError::Timeout(_))));
    }
}
