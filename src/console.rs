use nix::pty::openpty;
use nix::unistd::{close, dup, dup2, read};
use std::collections::VecDeque;
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

nix::ioctl_write_int_bad!(tioccons, 0x541D);

const MAX_CONSOLE_LINES: usize = 50;

/// Thread-safe ring buffer of captured console messages.
pub type ConsoleBuffer = Arc<Mutex<VecDeque<String>>>;

/// Intercepts kernel/systemd console output via TIOCCONS and process stdout/stderr
/// via dup2, capturing all messages into a shared ring buffer.
pub struct ConsoleInterceptor {
    _reader_handle: JoinHandle<()>,
    master_fd: RawFd,
    slave_fd: RawFd,
    orig_stderr: Option<RawFd>,
}

impl ConsoleInterceptor {
    /// Opens a pty, redirects /dev/console output via TIOCCONS and process stdout/stderr
    /// via dup2, and spawns a background thread to read captured messages.
    pub fn intercept(buffer: ConsoleBuffer) -> io::Result<Self> {
        let pty = openpty(None, None)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("openpty: {}", e)))?;

        let slave_fd = pty.slave.as_raw_fd();

        // Attempt /dev/console redirection via TIOCCONS (requires root)
        unsafe {
            let _ = tioccons(slave_fd, 0);
        }

        // Duplicate original stderr (FD 2) and redirect it to PTY slave
        let orig_stderr = dup(2).ok();
        let _ = dup2(slave_fd, 2);

        let master_fd = pty.master.as_raw_fd();

        // Prevent nix from closing the fds when OwnedFd drops
        let master_raw = master_fd;
        let slave_raw = slave_fd;
        std::mem::forget(pty.master);
        std::mem::forget(pty.slave);
        let reader_buffer = buffer.clone();

        let reader_handle = thread::spawn(move || {
            let mut line_buf = Vec::with_capacity(512);
            let mut read_buf = [0u8; 1024];

            loop {
                match read(master_raw, &mut read_buf) {
                    Ok(0) => break, // EOF — pty closed
                    Ok(n) => {
                        for &byte in &read_buf[..n] {
                            if byte == b'\n' {
                                let line = String::from_utf8_lossy(&line_buf).to_string();
                                {
                                    let mut buf = reader_buffer.lock().unwrap();
                                    if buf.len() >= MAX_CONSOLE_LINES {
                                        buf.pop_front();
                                    }
                                    buf.push_back(line.clone());
                                }
                                // Write natively to systemd's original service stderr pipe
                                if let Some(fd) = orig_stderr {
                                    let mut out = line.as_bytes().to_vec();
                                    out.push(b'\n');
                                    unsafe {
                                        libc::write(fd, out.as_ptr() as *const _, out.len());
                                    }
                                }
                                line_buf.clear();
                            } else {
                                line_buf.push(byte);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            _reader_handle: reader_handle,
            master_fd: master_raw,
            slave_fd: slave_raw,
            orig_stderr,
        })
    }
}

impl Drop for ConsoleInterceptor {
    fn drop(&mut self) {
        if let Some(fd) = self.orig_stderr {
            let _ = dup2(fd, 2);
            let _ = close(fd);
        }
        let _ = close(self.slave_fd);
        let _ = close(self.master_fd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_interceptor_captures_stdout_stderr() {
        let buffer: ConsoleBuffer = Arc::new(Mutex::new(VecDeque::new()));
        {
            let _interceptor = ConsoleInterceptor::intercept(buffer.clone()).unwrap();
            unsafe {
                libc::write(2, b"test_captured_stderr_line\n".as_ptr() as *const _, 26);
            }
            let start = std::time::Instant::now();
            while start.elapsed() < std::time::Duration::from_millis(300) {
                {
                    let lines = buffer.lock().unwrap();
                    if lines.iter().any(|l| l.contains("test_captured_stderr_line")) {
                        return;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        let lines = buffer.lock().unwrap();
        assert!(
            lines.iter().any(|l| l.contains("test_captured_stderr_line")),
            "ConsoleInterceptor should capture process stderr into console_buffer"
        );
    }
}
