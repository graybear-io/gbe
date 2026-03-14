//! Connection handling for router control channel

use anyhow::{Context, Result};
use gbe_protocol::ControlMessage;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

/// Wrapper for a control channel connection
pub struct Connection {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl Connection {
    /// Create a new connection from a `UnixStream`
    pub fn new(stream: UnixStream) -> Self {
        let writer = stream.try_clone().expect("Failed to clone stream");
        let reader = BufReader::new(stream);

        Self { reader, writer }
    }

    /// Receive a control message (JSON, newline-delimited)
    pub fn recv_message(&mut self) -> Result<Option<ControlMessage>> {
        let mut line = String::new();
        let n = self
            .reader
            .read_line(&mut line)
            .context("Failed to read from socket")?;

        if n == 0 {
            // EOF
            return Ok(None);
        }

        let msg = serde_json::from_str(line.trim()).context("Failed to parse control message")?;

        Ok(Some(msg))
    }

    /// Send a control message (JSON, newline-delimited)
    pub fn send_message(&mut self, msg: &ControlMessage) -> Result<()> {
        let json = serde_json::to_string(msg).context("Failed to serialize control message")?;

        writeln!(self.writer, "{json}").context("Failed to write to socket")?;

        self.writer.flush().context("Failed to flush socket")?;

        Ok(())
    }
}
