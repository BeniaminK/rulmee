use std::fs::OpenOptions;
use std::io::{self, Write};
use std::sync::Arc;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use crate::config::LoggingConfig;
use crate::console::ConsoleBuffer;

const DEFAULT_LOG_FILE: &str = "/tmp/lidm.log";

struct ConsoleBufferWriter {
    buffer: ConsoleBuffer,
}

impl Write for ConsoleBufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&*self).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        (&*self).flush()
    }
}

impl Write for &ConsoleBufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        let mut lines = self.buffer.lock().unwrap();
        for line in text.lines() {
            if lines.len() >= 50 {
                lines.pop_front();
            }
            lines.push_back(line.to_string());
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct SystemdPipeWriter {
    fd: Option<std::os::unix::io::RawFd>,
}

impl Write for SystemdPipeWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&*self).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        (&*self).flush()
    }
}

impl Write for &SystemdPipeWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(fd) = self.fd {
            unsafe {
                libc::write(fd, buf.as_ptr() as *const _, buf.len());
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn resolve_log_path(cli_log_file: Option<&str>) -> String {
    if let Some(cli_path) = cli_log_file {
        if !cli_path.trim().is_empty() {
            return cli_path.to_string();
        }
    }
    DEFAULT_LOG_FILE.to_string()
}

pub fn initialize_logging(
    log_cfg: &LoggingConfig,
    console_buffer: Option<ConsoleBuffer>,
) -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    // Ignore error if log tracer was already initialized in process
    let _ = tracing_log::LogTracer::init();

    let path = resolve_log_path(Some(&log_cfg.file));
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;

    let (non_blocking_file, guard) = tracing_appender::non_blocking(file);

    let env_filter = EnvFilter::try_new(&log_cfg.level)
        .unwrap_or_else(|_| EnvFilter::new("debug"));

    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false)
        .compact();

    let console_layer = console_buffer.map(|buffer| {
        let console_writer = ConsoleBufferWriter { buffer };
        fmt::layer()
            .with_writer(Arc::new(console_writer))
            .with_ansi(false)
            .compact()
    });

    let systemd_layer = if log_cfg.stdout {
        let systemd_fd = nix::unistd::dup(1).ok();
        Some(
            fmt::layer()
                .with_writer(Arc::new(SystemdPipeWriter { fd: systemd_fd }))
                .with_ansi(false)
                .compact(),
        )
    } else {
        None
    };

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(console_layer)
        .with(systemd_layer);

    let _ = subscriber.try_init();

    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn test_console_buffer_writer_ring_buffer() {
        use std::collections::VecDeque;

        let buffer: ConsoleBuffer = Arc::new(Mutex::new(VecDeque::new()));
        let mut writer = ConsoleBufferWriter {
            buffer: buffer.clone(),
        };

        for i in 0..60 {
            let log_line = format!("line {}\n", i);
            writer.write_all(log_line.as_bytes()).unwrap();
        }

        let lines = buffer.lock().unwrap();
        assert_eq!(lines.len(), 50);
        assert_eq!(lines.front().unwrap(), "line 10");
        assert_eq!(lines.back().unwrap(), "line 59");
    }

    #[test]
    fn test_resolve_log_path_cli_arg() {
        let path = resolve_log_path(Some("/tmp/lidm_cli_test.log"));
        assert_eq!(path, "/tmp/lidm_cli_test.log");
    }

    #[test]
    fn test_resolve_log_path_default_fallback() {
        let path = resolve_log_path(None);
        assert_eq!(path, DEFAULT_LOG_FILE);

        let path_empty = resolve_log_path(Some("  "));
        assert_eq!(path_empty, DEFAULT_LOG_FILE);
    }

    #[test]
    fn test_double_initialize_logging_idempotent() {
        let cfg = LoggingConfig {
            file: "/tmp/lidm_test_double.log".to_string(),
            level: "debug".to_string(),
            stdout: false,
        };
        let _guard1 = initialize_logging(&cfg, None);
        assert!(_guard1.is_ok());
        let _guard2 = initialize_logging(&cfg, None);
        assert!(_guard2.is_ok());
    }

    #[test]
    fn test_stdout_colored_logging() {
        let cfg = LoggingConfig {
            file: "/tmp/lidm_stdout_test.log".to_string(),
            level: "trace".to_string(),
            stdout: true,
        };
        let _log_guard = initialize_logging(&cfg, None).unwrap();

        tracing::trace!("This is TRACE (file / stdout)");
        tracing::debug!("This is DEBUG (file / stdout)");
        tracing::info!("This is INFO (file / stdout)");
        tracing::warn!("This is WARN (file / stdout)");
        tracing::error!("This is ERROR (file / stdout)");
    }
}



