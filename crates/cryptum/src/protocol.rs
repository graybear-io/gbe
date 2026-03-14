/// ttyd wire protocol types and framing.
///
/// Protocol summary:
/// - Client sends JSON text message on connect: {"AuthToken":"","columns":N,"rows":N}
/// - After handshake, binary frames with ASCII prefix byte:
///   - b'0' = terminal I/O (input from client, output from server)
///   - b'1' = resize (client sends JSON), title (server sends string)
///   - b'2' = preferences (server sends JSON) / pause (client)
///   - b'3' = resume (client)
///
/// ASCII prefix bytes — these are char codes, not raw byte values.
pub const CLIENT_INPUT: u8 = b'0';
pub const CLIENT_RESIZE: u8 = b'1';
pub const SERVER_OUTPUT: u8 = b'0';
#[allow(dead_code)]
pub const SERVER_TITLE: u8 = b'1';
#[allow(dead_code)]
pub const SERVER_PREFS: u8 = b'2';

/// Build the JSON init handshake that ttyd expects on connect.
pub fn init_message(cols: u16, rows: u16) -> String {
    format!(r#"{{"AuthToken":"","columns":{cols},"rows":{rows}}}"#)
}

/// Wrap raw input bytes with the CLIENT_INPUT prefix.
pub fn frame_input(data: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(1 + data.len());
    msg.push(CLIENT_INPUT);
    msg.extend_from_slice(data);
    msg
}

/// Build a resize frame from terminal dimensions.
pub fn frame_resize(cols: u16, rows: u16) -> Vec<u8> {
    let json = format!(r#"{{"columns":{cols},"rows":{rows}}}"#);
    let mut msg = Vec::with_capacity(1 + json.len());
    msg.push(CLIENT_RESIZE);
    msg.extend_from_slice(json.as_bytes());
    msg
}

/// Parse a binary server message. Returns (type_byte, payload).
pub fn parse_server_message(data: &[u8]) -> Option<(u8, &[u8])> {
    if data.is_empty() {
        return None;
    }
    Some((data[0], &data[1..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_message_format() {
        let msg = init_message(80, 24);
        assert_eq!(msg, r#"{"AuthToken":"","columns":80,"rows":24}"#);
    }

    #[test]
    fn init_message_is_valid_json() {
        let msg = init_message(155, 32);
        let parsed: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(parsed["columns"], 155);
        assert_eq!(parsed["rows"], 32);
        assert_eq!(parsed["AuthToken"], "");
    }

    #[test]
    fn frame_input_prepends_prefix() {
        let frame = frame_input(b"hello");
        assert_eq!(frame[0], b'0');
        assert_eq!(&frame[1..], b"hello");
    }

    #[test]
    fn frame_input_empty() {
        let frame = frame_input(b"");
        assert_eq!(frame, vec![b'0']);
    }

    #[test]
    fn frame_resize_format() {
        let frame = frame_resize(120, 40);
        assert_eq!(frame[0], b'1');
        let json: serde_json::Value = serde_json::from_slice(&frame[1..]).unwrap();
        assert_eq!(json["columns"], 120);
        assert_eq!(json["rows"], 40);
    }

    #[test]
    fn parse_server_output() {
        let data = [b'0', b'h', b'e', b'l', b'l', b'o'];
        let (typ, payload) = parse_server_message(&data).unwrap();
        assert_eq!(typ, SERVER_OUTPUT);
        assert_eq!(payload, b"hello");
    }

    #[test]
    fn parse_server_title() {
        let data = [b'1', b't', b'i', b't', b'l', b'e'];
        let (typ, payload) = parse_server_message(&data).unwrap();
        assert_eq!(typ, SERVER_TITLE);
        assert_eq!(payload, b"title");
    }

    #[test]
    fn parse_server_empty() {
        assert!(parse_server_message(&[]).is_none());
    }

    #[test]
    fn parse_server_single_byte() {
        let (typ, payload) = parse_server_message(&[b'0']).unwrap();
        assert_eq!(typ, SERVER_OUTPUT);
        assert!(payload.is_empty());
    }

    #[test]
    fn constants_are_ascii() {
        assert_eq!(CLIENT_INPUT, 0x30);
        assert_eq!(CLIENT_RESIZE, 0x31);
        assert_eq!(SERVER_OUTPUT, 0x30);
        assert_eq!(SERVER_TITLE, 0x31);
        assert_eq!(SERVER_PREFS, 0x32);
    }
}
