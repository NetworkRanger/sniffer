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