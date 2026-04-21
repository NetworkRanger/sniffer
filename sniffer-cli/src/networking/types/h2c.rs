use std::collections::HashMap;
use enum_primitive::enum_from_primitive;
use tracing::{debug, trace};
use crate::enum_primitive::FromPrimitive;
use crate::networking::types::hpack::decode_hpack_codec;
use crate::networking::types::packet;

#[derive(Debug, Clone, Default)]
struct Frame {
    frame_type: u8,
    flags: u8,
    stream_id: u32,
    payload: Vec<u8>,
}

enum_from_primitive! {
    #[derive(Debug, PartialEq)]
    enum FrameType {
        Data = 0x0,
        Headers = 0x1,
        Settings = 0x4,
        WindowUpdate = 0x8,
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct H2c {
    frames: Vec<Frame>,
}

impl H2c {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse_headers(&mut self, buf: Vec<u8>) -> Option<HashMap<String, String>> {
        let mut packet = packet::Packet::new(buf.clone());
        trace!("buf: {}", hex::encode(buf.clone()));
        while !packet.is_end() {
            let length = packet.read_h2c_length();
            trace!("length: {}", length);
            let mut frame = Frame::default();
            frame.frame_type = packet.read_u8();
            frame.flags = packet.read_u8();
            frame.stream_id = packet.read_u32();
            frame.stream_id &= 0x7fff_ffff;
            frame.payload = packet.read_bytes(length as usize);
            trace!("payload: {:?}", hex::encode(frame.payload.clone()));
            trace!("packet_length: {}", packet.len());
            if let Some(FrameType::Headers) = FrameType::from_u8(frame.frame_type) {
                let headers = decode_hpack_codec(frame.payload.as_ref());
                debug!("headers: {:?}", headers);
                return Some(headers);
            }
        }
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Build an HTTP/2 frame: [length(3) | type(1) | flags(1) | stream_id(4) | payload]
    fn build_h2_frame(frame_type: u8, flags: u8, stream_id: u32, payload: &[u8]) -> Vec<u8> {
        let len = payload.len() as u32;
        let mut buf = vec![];
        buf.push((len >> 16) as u8);
        buf.push((len >> 8) as u8);
        buf.push(len as u8);
        buf.push(frame_type);
        buf.push(flags);
        buf.extend_from_slice(&stream_id.to_be_bytes());
        buf.extend_from_slice(payload);
        buf
    }

    #[test]
    fn test_h2c_new() {
        let h2c = H2c::new();
        assert!(h2c.frames.is_empty());
    }

    #[test]
    fn test_parse_headers_with_headers_frame() {
        let mut h2c = H2c::new();

        // RFC 7541 C.3.1: GET http://www.example.com/
        // :method=GET, :scheme=http, :path=/, :authority=www.example.com
        let hpack_payload: Vec<u8> = vec![
            0x82, 0x86, 0x84, 0x41, 0x0f, 0x77, 0x77, 0x77, 0x2e, 0x65,
            0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x63, 0x6f, 0x6d,
        ];

        // frame_type=0x01 (HEADERS), flags=0x04 (END_HEADERS), stream_id=1
        let data = build_h2_frame(0x01, 0x04, 1, &hpack_payload);

        let result = h2c.parse_headers(data);
        assert!(result.is_some());
        let headers = result.unwrap();
        assert_eq!(headers.get(":method").unwrap(), "GET");
        assert_eq!(headers.get(":scheme").unwrap(), "http");
        assert_eq!(headers.get(":path").unwrap(), "/");
        assert_eq!(headers.get(":authority").unwrap(), "www.example.com");
    }

    #[test]
    fn test_parse_headers_no_headers_frame() {
        let mut h2c = H2c::new();

        // SETTINGS frame (type=0x04), no HEADERS frame present
        let data = build_h2_frame(0x04, 0x00, 0, &[0x00; 6]);

        let result = h2c.parse_headers(data);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_headers_settings_then_headers() {
        let mut h2c = H2c::new();

        // First frame: SETTINGS (type=0x04)
        let settings = build_h2_frame(0x04, 0x00, 0, &[0x00; 6]);

        // Second frame: HEADERS (type=0x01) with HPACK-encoded :method=GET, :path=/
        let hpack_payload: Vec<u8> = vec![0x82, 0x84]; // :method=GET, :path=/
        let headers_frame = build_h2_frame(0x01, 0x04, 1, &hpack_payload);

        let mut data = settings;
        data.extend_from_slice(&headers_frame);

        let result = h2c.parse_headers(data);
        assert!(result.is_some());
        let headers = result.unwrap();
        assert_eq!(headers.get(":method").unwrap(), "GET");
        assert_eq!(headers.get(":path").unwrap(), "/");
    }

    #[test]
    fn test_parse_headers_empty_input() {
        let mut h2c = H2c::new();
        let result = h2c.parse_headers(vec![]);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_headers_data_frame_only() {
        let mut h2c = H2c::new();

        // DATA frame (type=0x00) — should not be treated as headers
        let data = build_h2_frame(0x00, 0x01, 1, b"hello");

        let result = h2c.parse_headers(data);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_headers_stream_id_high_bit_masked() {
        let mut h2c = H2c::new();

        // HEADERS frame with stream_id that has the reserved high bit set (0x80000001)
        // parse_headers masks with 0x7fff_ffff, so it should still work
        let hpack_payload: Vec<u8> = vec![0x82]; // :method=GET
        let data = build_h2_frame(0x01, 0x04, 0x80000001, &hpack_payload);

        let result = h2c.parse_headers(data);
        assert!(result.is_some());
        let headers = result.unwrap();
        assert_eq!(headers.get(":method").unwrap(), "GET");
    }

    #[test]
    fn test_parse_headers_window_update_then_headers() {
        let mut h2c = H2c::new();

        // WINDOW_UPDATE frame (type=0x08), 4-byte payload
        let wu = build_h2_frame(0x08, 0x00, 0, &0x00010000u32.to_be_bytes());

        // HEADERS frame
        let hpack_payload: Vec<u8> = vec![0x86]; // :scheme=http
        let headers_frame = build_h2_frame(0x01, 0x04, 1, &hpack_payload);

        let mut data = wu;
        data.extend_from_slice(&headers_frame);

        let result = h2c.parse_headers(data);
        assert!(result.is_some());
        let headers = result.unwrap();
        assert_eq!(headers.get(":scheme").unwrap(), "http");
    }

    #[test]
    fn test_parse_headers_multiple_non_header_frames() {
        let mut h2c = H2c::new();

        // SETTINGS + WINDOW_UPDATE + DATA — no HEADERS at all
        let mut data = build_h2_frame(0x04, 0x00, 0, &[0x00; 6]);
        data.extend_from_slice(&build_h2_frame(0x08, 0x00, 0, &[0x00; 4]));
        data.extend_from_slice(&build_h2_frame(0x00, 0x00, 1, b"body"));

        let result = h2c.parse_headers(data);
        assert!(result.is_none());
    }
}
