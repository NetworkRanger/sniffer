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
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_static_indexed_headers() {
        // RFC 7541 C.3.1: :method=GET, :scheme=http, :path=/, :authority=www.example.com
        let encoded: Vec<u8> = vec![
            0x82, 0x86, 0x84, 0x41, 0x0f, 0x77, 0x77, 0x77, 0x2e, 0x65,
            0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x63, 0x6f, 0x6d,
        ];
        let headers = decode_hpack_codec(&encoded);
        assert_eq!(headers.get(":method").unwrap(), "GET");
        assert_eq!(headers.get(":scheme").unwrap(), "http");
        assert_eq!(headers.get(":path").unwrap(), "/");
        assert_eq!(headers.get(":authority").unwrap(), "www.example.com");
    }

    #[test]
    fn test_decode_single_indexed() {
        // 0x82 = static index 2 = :method GET
        let headers = decode_hpack_codec(&[0x82]);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers.get(":method").unwrap(), "GET");
    }

    #[test]
    fn test_decode_literal_header() {
        // 0x40 = literal with indexing, new name
        // name_len=3 "foo", value_len=3 "bar"
        let encoded: Vec<u8> = vec![0x40, 0x03, 0x66, 0x6f, 0x6f, 0x03, 0x62, 0x61, 0x72];
        let headers = decode_hpack_codec(&encoded);
        assert_eq!(headers.get("foo").unwrap(), "bar");
    }

    #[test]
    fn test_decode_multiple_headers() {
        // :method=GET (0x82), :path=/ (0x84), :scheme=https (0x87)
        let headers = decode_hpack_codec(&[0x82, 0x84, 0x87]);
        assert_eq!(headers.len(), 3);
        assert_eq!(headers.get(":method").unwrap(), "GET");
        assert_eq!(headers.get(":path").unwrap(), "/");
        assert_eq!(headers.get(":scheme").unwrap(), "https");
    }
}
