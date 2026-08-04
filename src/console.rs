use nix::pty::openpty;
use nix::unistd::{close, read};
use std::collections::VecDeque;
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

nix::ioctl_write_int_bad!(tioccons, 0x541D);

const MAX_CONSOLE_LINES: usize = 50;

/// Thread-safe ring buffer of captured console messages.
pub type ConsoleBuffer = Arc<Mutex<VecDeque<String>>>;

/// Intercepts kernel/systemd console output via TIOCCONS and captures it
/// into a shared ring buffer. The TUI can read from this buffer to display
/// console messages, and they also stay out of the terminal.
pub struct ConsoleInterceptor {
    _reader_handle: JoinHandle<()>,
    master_fd: RawFd,
    slave_fd: RawFd,
}

impl ConsoleInterceptor {
    /// Opens a pty, redirects /dev/console output to it via TIOCCONS,
    /// and spawns a background thread to read captured messages.
    pub fn intercept(buffer: ConsoleBuffer) -> io::Result<Self> {
        let pty = openpty(None, None)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("openpty: {}", e)))?;

        let slave_fd = pty.slave.as_raw_fd();

        // Redirect /dev/console output to our pty slave
        unsafe {
            tioccons(slave_fd, 0).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("TIOCCONS: {} (are you root?)", e),
                )
            })?;
        }

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
                                let mut buf = reader_buffer.lock().unwrap();
                                if buf.len() >= MAX_CONSOLE_LINES {
                                    buf.pop_front();
                                }
                                buf.push_back(line);
                                log::debug!(target: "console", "{}", buf.back().unwrap());
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
        })
    }
}

impl Drop for ConsoleInterceptor {
    fn drop(&mut self) {
        // Closing the slave fd releases TIOCCONS, restoring normal console output
        let _ = close(self.slave_fd);
        let _ = close(self.master_fd);
    }
}
