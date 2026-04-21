use tls_parser::{
    TlsExtension, TlsMessage, TlsMessageHandshake,
    parse_tls_plaintext,
};
use tracing::{trace};
use tracing::debug;
use crate::packet::Connection;

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个仅用于测试的 Connection
    fn make_connection() -> Connection {
        Connection::new(
            "test-conn-id".to_string(),
            "tcp".to_string(),
            "192.168.1.100".to_string(),
            12345u16,
            "93.184.216.34".to_string(),
            443u16,
        )
    }

    // -------------------------------------------------------------------------
    // TLS ClientHello with SNI extension  (https://example.com)
    // -------------------------------------------------------------------------
    // TLS Record
    //   0x16 0x03 0x03 <len_hi> <len_lo>   — ContentType=Handshake, TLS 1.2
    // Handshake (ClientHello)
    //   0x01                                  — msg type ClientHello
    //   0x00 0x00 <ch_len_hi> <ch_len_lo>   — 3-byte length
    //   0x03 0x03                            — client version TLS 1.2
    //   0x00 … 0x00 (32 bytes)               — random
    //   0x00                                  — session_id length = 0
    //   0x00 0x02                            — cipher_suites length = 2
    //   0xc0 0x2c                            — TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
    //   0x01                                  — compression_methods length = 1
    //   0x00                                  — null compression
    //   0x00 0x11                            — extensions length = 17
    // SNI extension (0x0000)
    //   0x00 0x00                            — extension type SNI
    //   0x00 0x0d                            — extension length = 13
    //   0x00 0x0b                            — server_name list length = 11
    //   0x00                                  — server_name type = host_name
    //   0x00 0x0b                            — server_name length = 11
    //   "example.com" (11 bytes)
    fn tls_client_hello_with_sni() -> Vec<u8> {
        vec![
            // TLS record header: length = 4 (handshake hdr) + 63 (CH body) = 67 = 0x43
            0x16, 0x03, 0x03, 0x00, 0x43,
            // Handshake header: type=ClientHello, length = 63 = 0x3f
            0x01, 0x00, 0x00, 0x3f,
            // ClientVersion
            0x03, 0x03,
            // Random (32 bytes — all zeros for deterministic testing)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            // session_id length = 0
            0x00,
            // cipher_suites
            0x00, 0x02, 0xc0, 0x2c,
            // compression_methods
            0x01, 0x00,
            // extensions length = 20 = 0x14
            0x00, 0x14,
            // ----- SNI extension -----
            0x00, 0x00, // type SNI
            0x00, 0x10, // extension data length = 16 (2 list_len + 1 type + 2 name_len + 11 name)
            0x00, 0x0e, // server_name list length = 14 (1 type + 2 name_len + 11 name)
            0x00,       // type = host_name
            0x00, 0x0b, // server_name length = 11
            // "example.com"
            b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.',
            b'c', b'o', b'm',
        ]
    }

    // -------------------------------------------------------------------------
    // TLS ClientHello WITHOUT any extensions (no SNI)
    // -------------------------------------------------------------------------
    // Same structure as above but with extensions_length = 0x00 0x00
    fn tls_client_hello_no_ext() -> Vec<u8> {
        vec![
            0x16, 0x03, 0x03, 0x00, 0x2a,
            0x01, 0x00, 0x00, 0x26,
            0x03, 0x03,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00,
            0x00, 0x02, 0xc0, 0x2c,
            0x01, 0x00,
            // no extensions
            0x00, 0x00,
        ]
    }

    // -------------------------------------------------------------------------
    // Valid TLS record but NOT a ClientHello (ServerHello)
    // -------------------------------------------------------------------------
    fn tls_server_hello() -> Vec<u8> {
        vec![
            0x16, 0x03, 0x03, 0x00, 0x2a,
            0x02, 0x00, 0x00, 0x26, // 0x02 = ServerHello
            0x03, 0x03,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00,
            0x00, 0x02, 0xc0, 0x2c,
            0x01, 0x00,
            0x00, 0x00,
        ]
    }

    // -------------------------------------------------------------------------
    // Truncated / garbage data — should not panic parse_client_hello
    // -------------------------------------------------------------------------
    fn garbage_data() -> Vec<u8> {
        vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]
    }

    // -------------------------------------------------------------------------
    // TLS record header only — missing handshake body
    // -------------------------------------------------------------------------
    fn truncated_record() -> Vec<u8> {
        vec![0x16, 0x03, 0x03, 0x00, 0x40] // says length=64 but no data follows
    }

    // -------------------------------------------------------------------------
    // Extension data that contains SNI but with empty server_name list
    // -------------------------------------------------------------------------
    fn sni_empty_list_ext() -> Vec<u8> {
        // Extension type SNI (0x0000), length 2, but list length = 0
        vec![0x00, 0x00, 0x00, 0x02, 0x00, 0x00]
    }

    // -------------------------------------------------------------------------
    // Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_client_hello_with_sni() {
        let data = tls_client_hello_with_sni();
        let mut conn = make_connection();

        let result = parse_client_hello(&data, &mut conn);

        assert!(result.is_ok(), "parse_client_hello should not return Err");
        assert!(conn.domain.is_some(), "domain should be set from SNI");
        assert_eq!(conn.domain.as_deref(), Some("example.com"));
        assert_eq!(conn.protocol, "https");
    }

    #[test]
    fn test_parse_client_hello_no_ext() {
        let data = tls_client_hello_no_ext();
        let mut conn = make_connection();

        let result = parse_client_hello(&data, &mut conn);

        assert!(result.is_ok(), "parse_client_hello should not return Err");
        assert!(
            conn.domain.is_none(),
            "domain should NOT be set when no SNI extension present"
        );
    }

    #[test]
    fn test_parse_client_hello_server_hello() {
        // ServerHello is not a ClientHello — parse should still succeed
        // (no panic, domain stays None)
        let data = tls_server_hello();
        let mut conn = make_connection();

        let result = parse_client_hello(&data, &mut conn);

        assert!(result.is_ok());
        assert!(conn.domain.is_none());
    }

    #[test]
    fn test_parse_client_hello_garbage() {
        // Garbage data should be silently tolerated (function always returns Ok)
        let data = garbage_data();
        let mut conn = make_connection();

        let result = parse_client_hello(&data, &mut conn);

        assert!(result.is_ok());
        assert!(conn.domain.is_none());
    }

    #[test]
    fn test_parse_client_hello_truncated() {
        // Truncated TLS record (header only, no body)
        let data = truncated_record();
        let mut conn = make_connection();

        let result = parse_client_hello(&data, &mut conn);

        // parse_client_hello swallows the error internally
        assert!(result.is_ok());
        assert!(conn.domain.is_none());
    }

    #[test]
    fn test_parse_extensions_with_sni() {
        // Raw SNI extension bytes (no TLS record wrapper)
        let ext_data = tls_client_hello_with_sni();
        // Skip TLS record + handshake header to get extension data
        // TLS record = 5 bytes, Handshake type+len = 4 bytes,
        // ClientVersion(2) + random(32) + session_id(1) + cipher_suites(4) + compression(2) = 41 bytes
        // extensions length field starts at 5+4+41 = 50
        // extensions length field is 2 bytes at offset 50, actual extension data starts at 52
        let start = 52_usize;
        let ext_slice = &ext_data[start..];

        let mut conn = make_connection();
        parse_extensions(ext_slice, &mut conn);

        assert!(conn.domain.is_some());
        assert_eq!(conn.domain.as_deref(), Some("example.com"));
        assert_eq!(conn.protocol, "https");
    }

    #[test]
    fn test_parse_extensions_empty_sni_list() {
        // SNI extension exists but server_name list is empty
        let ext_data = sni_empty_list_ext();
        let mut conn = make_connection();

        // Should not panic; domain stays None
        parse_extensions(&ext_data, &mut conn);

        assert!(conn.domain.is_none());
    }
}

#[derive(Debug, Clone)]
pub struct TlsPacket {
    pub len: u16,
    pub data: Vec<u8>,
}


pub fn parse_client_hello(data: &[u8], connection: &mut Connection) -> Result<(), Box<dyn std::error::Error>> {
    // 解析 TLS 记录
    trace!("parse_client_hello: {}", data.len());

    match parse_tls_plaintext(data) {
        Ok((_, record)) => {
            trace!("record: {:?}", record);
            for msg in record.msg {
                if let TlsMessage::Handshake(handshake) = msg {
                    if let TlsMessageHandshake::ClientHello(ch) = handshake {
                        // 扩展
                        if let Some(ext_data) = ch.ext {
                            parse_extensions(ext_data, connection);
                        }
                    }
                }
            }
        }
        Err(_e) => {
            // warn!("parse_tls_plaintext error: {}", e);
            // warn!("data: {:?}", hex::encode(data));
        },
    }

    Ok(())
}

pub fn parse_extensions(data: &[u8], connection: &mut Connection)  {
    use tls_parser::parse_tls_client_hello_extensions;

    let (_, extensions): (_, Vec<TlsExtension>) = parse_tls_client_hello_extensions(data).unwrap();

    for ext in extensions {
        if let TlsExtension::SNI(snis) = ext {
            for (_sni_type, sni_value) in snis {
                debug!("HTTPS server_name: {}, connection_id: {}", String::from_utf8_lossy(sni_value), connection.id);
                connection.domain = Some(String::from_utf8_lossy(sni_value).to_string());
                connection.protocol = "https".to_string();
            }
        }
    }
}
