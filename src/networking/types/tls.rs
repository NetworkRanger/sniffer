use pnet::packet::ip::IpNextHeaderProtocols::St;
use tls_parser::{
    TlsClientHelloContents, TlsExtension, TlsMessage, TlsMessageHandshake, parse_tls_extension_sni,
    parse_tls_plaintext,
};
use tracing::{info, trace};
use tracing::debug;
use tracing::warn;
use tracing_subscriber::fmt::format;
use crate::packet::Connection;
use crate::utils::registry::Registry;

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
        Err(e) => {
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
            for (sni_type, sni_value) in snis {
                debug!("HTTPS server_name: {}, connection_id: {}", String::from_utf8_lossy(sni_value), connection.id);
                connection.domain = Some(String::from_utf8_lossy(sni_value).to_string());
            }
        }
    }
}
