use ring::{hkdf, aead};
use tracing::info;

// QUIC Initial Salt (RFC 9001)
const INITIAL_SALT_V1: &[u8] = &[
    0x38, 0x76, 0x2c, 0xf7, 0xf5, 0x59, 0x34, 0xb3,
    0x4d, 0x17, 0x9a, 0xe6, 0xa4, 0xc8, 0x0c, 0xad,
    0xcc, 0xbb, 0x7f, 0x0a,
];

fn derive_initial_secret(dcid: &[u8]) -> hkdf::Prk {
    // initial_secret = HKDF-Extract(INITIAL_SALT, client_dst_connection_id)
    hkdf::Salt::new(hkdf::HKDF_SHA256, INITIAL_SALT_V1)
        .extract(dcid)
}

// fn derive_client_initial_secret(prk: &hkdf::Prk) {
//     // client_initial_secret = HKDF-Expand-Label(initial_secret, "client in", "", 32)
//     let mut okm = [0u8; 32];
//     prk.expand(&[b"client in"], &mut okm).unwrap();
//     let a = hkdf::Okm::new(okm);
//     a
// }

fn main() {
    tracing_subscriber::fmt::init();

    
    let dcid = hex::decode("3e587cd500e165f3").unwrap();
    let prk = derive_initial_secret(&dcid);
    info!("prk {:?}", prk);
}