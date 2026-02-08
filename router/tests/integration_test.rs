//! Integration tests for gbe-router

use anyhow::Result;
use gbe_protocol::ControlMessage;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

struct RouterProcess {
    child: Child,
    socket_path: String,
}

impl RouterProcess {
    fn start() -> Result<Self> {
        let socket_path = "/tmp/gbe-router-test.sock";

        // Clean up old socket
        let _ = std::fs::remove_file(socket_path);

        // Start router in background
        let child = Command::new("cargo")
            .args(&[
                "run",
                "--package",
                "gbe-router",
                "--",
                "--socket",
                socket_path,
            ])
            .spawn()?;

        // Wait for router to start
        thread::sleep(Duration::from_millis(100));

        Ok(Self {
            child,
            socket_path: socket_path.to_string(),
        })
    }

    fn connect(&self) -> Result<TestConnection> {
        let stream = UnixStream::connect(&self.socket_path)?;
        Ok(TestConnection::new(stream))
    }
}

impl Drop for RouterProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

struct TestConnection {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl TestConnection {
    fn new(stream: UnixStream) -> Self {
        let writer = stream.try_clone().unwrap();
        let reader = BufReader::new(stream);
        Self { reader, writer }
    }

    fn send(&mut self, msg: &ControlMessage) -> Result<()> {
        let json = serde_json::to_string(msg)?;
        writeln!(self.writer, "{}", json)?;
        self.writer.flush()?;
        Ok(())
    }

    fn recv(&mut self) -> Result<ControlMessage> {
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        Ok(serde_json::from_str(line.trim())?)
    }
}

#[test]
#[ignore] // Requires router binary
fn test_connect_and_disconnect() -> Result<()> {
    let router = RouterProcess::start()?;
    let mut conn = router.connect()?;

    // Send Connect
    conn.send(&ControlMessage::Connect {
        capabilities: vec!["pty".to_string()],
    })?;

    // Receive ConnectAck
    match conn.recv()? {
        ControlMessage::ConnectAck {
            tool_id,
            data_listen_address,
        } => {
            assert!(tool_id.contains('-'));
            assert!(data_listen_address.starts_with("unix:///tmp/gbe-"));
        }
        msg => panic!("Expected ConnectAck, got {:?}", msg),
    }

    // Send Disconnect
    conn.send(&ControlMessage::Disconnect)?;

    Ok(())
}

#[test]
#[ignore] // Requires router binary
fn test_subscribe_to_tool() -> Result<()> {
    let router = RouterProcess::start()?;

    // Tool A connects
    let mut tool_a = router.connect()?;
    tool_a.send(&ControlMessage::Connect {
        capabilities: vec![],
    })?;

    let tool_a_id = match tool_a.recv()? {
        ControlMessage::ConnectAck { tool_id, .. } => tool_id,
        msg => panic!("Expected ConnectAck, got {:?}", msg),
    };

    // Tool B connects
    let mut tool_b = router.connect()?;
    tool_b.send(&ControlMessage::Connect {
        capabilities: vec!["raw".to_string()],
    })?;

    let _ = tool_b.recv()?; // Consume ConnectAck

    // Tool B subscribes to Tool A
    tool_b.send(&ControlMessage::Subscribe {
        target: tool_a_id.clone(),
    })?;

    match tool_b.recv()? {
        ControlMessage::SubscribeAck {
            data_connect_address,
            capabilities,
        } => {
            assert!(data_connect_address.contains(&tool_a_id));
            assert_eq!(capabilities.len(), 0);
        }
        msg => panic!("Expected SubscribeAck, got {:?}", msg),
    }

    Ok(())
}

#[test]
#[ignore] // Requires router binary
fn test_subscribe_to_unknown_tool() -> Result<()> {
    let router = RouterProcess::start()?;

    let mut conn = router.connect()?;

    // Connect
    conn.send(&ControlMessage::Connect {
        capabilities: vec![],
    })?;
    let _ = conn.recv()?; // Consume ConnectAck

    // Subscribe to non-existent tool
    conn.send(&ControlMessage::Subscribe {
        target: "99999-999".to_string(),
    })?;

    match conn.recv()? {
        ControlMessage::Error { code, message } => {
            assert_eq!(code, "NOT_FOUND");
            assert!(message.contains("not found"));
        }
        msg => panic!("Expected Error, got {:?}", msg),
    }

    Ok(())
}

#[test]
#[ignore] // Requires router binary
fn test_query_capabilities() -> Result<()> {
    let router = RouterProcess::start()?;

    // Tool A connects with capabilities
    let mut tool_a = router.connect()?;
    tool_a.send(&ControlMessage::Connect {
        capabilities: vec!["pty".to_string(), "color".to_string()],
    })?;

    let tool_a_id = match tool_a.recv()? {
        ControlMessage::ConnectAck { tool_id, .. } => tool_id,
        msg => panic!("Expected ConnectAck, got {:?}", msg),
    };

    // Tool B queries A's capabilities
    let mut tool_b = router.connect()?;
    tool_b.send(&ControlMessage::Connect {
        capabilities: vec![],
    })?;
    let _ = tool_b.recv()?; // Consume ConnectAck

    tool_b.send(&ControlMessage::QueryCapabilities { target: tool_a_id })?;

    match tool_b.recv()? {
        ControlMessage::CapabilitiesResponse { capabilities } => {
            assert_eq!(capabilities.len(), 2);
            assert!(capabilities.contains(&"pty".to_string()));
            assert!(capabilities.contains(&"color".to_string()));
        }
        msg => panic!("Expected CapabilitiesResponse, got {:?}", msg),
    }

    Ok(())
}
