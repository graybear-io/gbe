//! Router connection handling for adapter

use anyhow::{Context, Result};
use gbe_protocol::ControlMessage;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

/// Connection to router control channel
pub struct RouterConnection {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl RouterConnection {
    /// Connect to router
    pub fn connect(router_socket: &str) -> Result<Self> {
        let stream = UnixStream::connect(router_socket)
            .context("Failed to connect to router")?;

        let writer = stream.try_clone()
            .context("Failed to clone stream")?;
        let reader = BufReader::new(stream);

        Ok(Self { reader, writer })
    }

    /// Send a control message to router
    pub fn send(&mut self, msg: &ControlMessage) -> Result<()> {
        let json = serde_json::to_string(msg)
            .context("Failed to serialize message")?;

        writeln!(self.writer, "{}", json)
            .context("Failed to write to router")?;

        self.writer.flush()
            .context("Failed to flush")?;

        Ok(())
    }

    /// Receive a control message from router
    pub fn recv(&mut self) -> Result<ControlMessage> {
        let mut line = String::new();
        self.reader.read_line(&mut line)
            .context("Failed to read from router")?;

        serde_json::from_str(line.trim())
            .context("Failed to parse message")
    }
}
