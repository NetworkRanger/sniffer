use std::collections::HashMap;
use hpack_codec::{Decoder};

pub fn decode_hpack_codec(headers_encoded: &[u8]) -> HashMap<String, String> {
    let mut decoder = Decoder::new(4096);
    let mut header = decoder.enter_header_block(&headers_encoded[..]).unwrap();
    let mut headers: HashMap<String, String> = HashMap::new();
    loop {
        match header.decode_field().unwrap() {
            Some(field) => {
                headers.insert(
                  String::from_utf8_lossy(field.name()).to_string(),
                  String::from_utf8_lossy(field.value()).to_string(),
                );
            },
            None => break,
        }
    }
    headers
}