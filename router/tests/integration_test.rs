//! Integration tests for gbe-router

mod test_harness;

use anyhow::Result;
use gbe_protocol::ControlMessage;
use test_harness::TestEnv;

#[test]
#[ignore] // Requires router binary
fn test_connect_and_disconnect() -> Result<()> {
    let mut env = TestEnv::new()?;
    env.start_router()?;
    let mut client = env.connect_client()?;

    // Send Connect
    client.send(&ControlMessage::Connect {
        capabilities: vec!["pty".to_string()],
    })?;

    // Receive ConnectAck
    match client.recv()? {
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
    client.send(&ControlMessage::Disconnect)?;

    Ok(())
}

#[test]
#[ignore] // Requires router binary
fn test_subscribe_to_tool() -> Result<()> {
    let mut env = TestEnv::new()?;
    env.start_router()?;

    // Tool A connects
    let mut tool_a = env.connect_client()?;
    tool_a.send(&ControlMessage::Connect {
        capabilities: vec![],
    })?;

    let tool_a_id = match tool_a.recv()? {
        ControlMessage::ConnectAck { tool_id, .. } => tool_id,
        msg => panic!("Expected ConnectAck, got {:?}", msg),
    };

    // Tool B connects
    let mut tool_b = env.connect_client()?;
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
    let mut env = TestEnv::new()?;
    env.start_router()?;

    let mut client = env.connect_client()?;

    // Connect
    client.send(&ControlMessage::Connect {
        capabilities: vec![],
    })?;
    let _ = client.recv()?; // Consume ConnectAck

    // Subscribe to non-existent tool
    client.send(&ControlMessage::Subscribe {
        target: "99999-999".to_string(),
    })?;

    match client.recv()? {
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
    let mut env = TestEnv::new()?;
    env.start_router()?;

    // Tool A connects with capabilities
    let mut tool_a = env.connect_client()?;
    tool_a.send(&ControlMessage::Connect {
        capabilities: vec!["pty".to_string(), "color".to_string()],
    })?;

    let tool_a_id = match tool_a.recv()? {
        ControlMessage::ConnectAck { tool_id, .. } => tool_id,
        msg => panic!("Expected ConnectAck, got {:?}", msg),
    };

    // Tool B queries A's capabilities
    let mut tool_b = env.connect_client()?;
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
