use tls_parser::{
    TlsClientHelloContents, TlsExtension, TlsMessage, TlsMessageHandshake, parse_tls_extension_sni,
    parse_tls_plaintext,
};
use tracing::info;
use tracing_subscriber::fmt::format;
use crate::utils::registry::Registry;

#[derive(Debug, Clone)]
pub struct TlsPacket {
    pub len: u16,
    pub data: Vec<u8>,
}


pub fn parse_client_hello(data: &[u8]) -> Result<(), Box<dyn std::error::Error + '_>> {
    // 解析 TLS 记录
    let (_, record) = parse_tls_plaintext(data)?;

    for msg in record.msg {
        if let TlsMessage::Handshake(handshake) = msg {
            if let TlsMessageHandshake::ClientHello(ch) = handshake {
                // 扩展
                if let Some(ext_data) = ch.ext {
                    parse_extensions(ext_data)?;
                }
            }
        }
    }

    Ok(())
}

pub fn parse_extensions(data: &[u8]) -> Result<(), Box<dyn std::error::Error + '_>> {
    use tls_parser::parse_tls_client_hello_extensions;

    let (_, extensions): (_, Vec<TlsExtension>) = parse_tls_client_hello_extensions(data)?;

    for ext in extensions {
        if let TlsExtension::SNI(snis) = ext {
            for (sni_type, sni_value) in snis {
                info!("HTTPS server_name: {}", String::from_utf8_lossy(sni_value));
            }
        }
    }

    Ok(())
}
