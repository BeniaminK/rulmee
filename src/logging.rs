use std::fs::File;
use std::io::{self, Write};
use crate::console::ConsoleBuffer;

const DEFAULT_LOG_FILE: &str = "/tmp/lidm.log";

struct MultiWriter {
    file: File,
    buffer: Option<ConsoleBuffer>,
}

impl Write for MultiWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write_all(buf)?;
        
        if let Some(ref buffer) = self.buffer {
            let text = String::from_utf8_lossy(buf);
            let mut lines = buffer.lock().unwrap();
            for line in text.lines() {
                if lines.len() >= 50 {
                    lines.pop_front();
                }
                lines.push_back(line.to_string());
            }
        }
        
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

pub fn initialize_logging(log_file: Option<&str>, console_buffer: Option<ConsoleBuffer>) -> std::io::Result<()> {
    let path = log_file.unwrap_or(DEFAULT_LOG_FILE);
    let file = File::create(path)?;

    let writer = MultiWriter {
        file,
        buffer: console_buffer,
    };

    let log_filter = std::env::var("LIDM_LOGLEVEL").unwrap_or_else(|_| "debug".to_string());

    env_logger::Builder::new()
        .parse_filters(&log_filter)
        .target(env_logger::Target::Pipe(Box::new(writer)))
        .init();

    Ok(())
}
