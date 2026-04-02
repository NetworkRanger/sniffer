use tracing::{debug, info, trace, warn};
use hmac_sha256::HKDF;
use aes::cipher::{KeyIvInit, StreamCipher};
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyInit};
use aes::Aes128;
use ecb::{Decryptor, Encryptor};
use ctr::Ctr128BE;
use crate::networking::types::packet;
use crate::utils::registry::Registry;

#[derive(Debug, Clone, Default)]
struct Crypto {
    frame_type: u8,
    offset: u16,
    length: u16,
    end: u16,
    data: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Quic {
    packet_number: u32,
    dst_conn_id: Vec<u8>,
    client_key: Vec<u8>,
    client_iv: Vec<u8>,
    client_hp: Vec<u8>,
}

impl Quic {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_payload(&mut self, data: Vec<u8>) -> Vec<u8> {
        let mut packet = packet::Packet::new(data);
        let flag = packet.read_u8();
        let mut packet_no_length = flag & 0x0f;
        let version = packet.read_u32();
        let dst_conn_length = packet.read_u8();
        debug!("DstConnLength: {}", dst_conn_length);
        self.dst_conn_id = packet.read_bytes(dst_conn_length as usize);
        debug!("DstConnId: {}", hex::encode(self.dst_conn_id.clone()));
        let src_conn_length = packet.read_u8();
        let src_conn_id = packet.read_bytes(src_conn_length as usize);
        let token_length = packet.read_u8();
        let token = packet.read_bytes(token_length as usize);
        let mut packet_length = packet.read_u16();
        packet_length &= 0x7ff;
        debug!("PacketLength: {}", packet_length);

        let _ = packet.read_bytes(4); // skip 4 bytes
        let sample = packet.read_bytes(16);
        packet.revert(20);
        debug!("sample {}", hex::encode(sample.clone()));
        self.get_client_key_and_iv();
        let mask = self.get_hp_mask(sample);
        packet_no_length ^= mask[0] & 0x0f;
        packet_no_length &= 0b11;
        packet_no_length += 1;
        debug!("PacketNoLength: {}", packet_no_length);
        packet_length -= packet_no_length as u16 + 16; // 去除packet_no和gcm 16字节tag
        debug!("PacketLength: {}", packet_length);
        let mut packet_no =  packet.read_bytes(packet_no_length as usize);
        for i in 0..packet_no.len() {
            packet_no[i] ^= mask[i + 1];
        }
        let mut packet_no_bytes = [0u8; 4];
        packet_no_bytes[4-packet_no.len()..].copy_from_slice(packet_no.as_slice());
        debug!("PacketNo: {}", hex::encode(packet_no_bytes.clone()));
        self.packet_number = u32::from_be_bytes(packet_no_bytes);
        debug!("packet_number: {}", self.packet_number);

        let encrypted_packet = packet.read_bytes(packet_length as usize);
        debug!("EncryptedPacket: {}", hex::encode(encrypted_packet.clone()));
        encrypted_packet
    }

    pub fn get_hp_mask(&mut self, sample: Vec<u8>) -> Vec<u8> {
        // 在一个缓冲区中加/解密（会自动填充）
        let mut buf = [0u8; 64];
        buf[..sample.len()].copy_from_slice(sample.as_slice());

        let key :[u8; 16] = self.client_hp.clone().try_into().unwrap();
        let e = Encryptor::<Aes128>::new(&key.into());
        e.encrypt_padded_mut::<Pkcs7>(&mut buf, sample.len()).unwrap();
        let mask = &buf[..5];
        debug!("mask: {}", hex::encode(mask));

        mask.to_vec()
    }

    fn get_client_key_and_iv(&mut self) {
        let salt = hex::decode("38762cf7f55934b34d179ae6a4c80cadccbb7f0a").unwrap();
        let prk = HKDF::extract(salt, self.dst_conn_id.clone());
        debug!("PRK: {}", hex::encode(prk));

        let client_in = self.hkdf_expand_label(prk, b"client in".to_vec(), 32);
        debug!("client_in (32 bytes): {}", hex::encode(client_in.clone()));

        self.client_key = self.hkdf_expand_label(client_in.clone(), b"quic key".to_vec(), 16);
        debug!("client_key (16 bytes): {}", hex::encode(self.client_key.clone()));

        self.client_iv = self.hkdf_expand_label(client_in.clone(), b"quic iv".to_vec(), 12);
        debug!("client_iv (12 bytes): {}", hex::encode(self.client_iv.clone()));

        self.client_hp = self.hkdf_expand_label(client_in.clone(), b"quic hp".to_vec(), 16);
        debug!("client_hp (16 bytes): {}", hex::encode(self.client_hp.clone()));
    }

    pub fn parse_inital_packet(&mut self, data: Vec<u8>) -> Option<String> {
        let encrypted_packet = self.get_payload(data);
        if self.packet_number >= 2 { // packet number 从0开始，比较小的包应该就够用，不会太大，暂时将大于2的忽略
            return None;
        }
        let mut plaintext = self.gcm_ctr_decrypt(encrypted_packet);

        debug!("plaintext: {}", hex::encode(plaintext.clone()));
        let mut packet = packet::Packet::new(plaintext);

        let key = "dst_conn_id:".to_string() + hex::encode(self.dst_conn_id.clone()).as_str();
        let mut crypto_list: Vec<Crypto> = match Registry::get::<Vec<Crypto>>(key.clone()) {
            Some(crypto_list) => crypto_list,
            None => vec![],
        };
        while !packet.is_end() {
            let frame_type = packet.read_u8();
            match frame_type {
                0x02 => { // ACK
                    packet.read_bytes(4); // skip
                },
                0x00 => { // PADDING
                    packet.read_padding();
                },
                0x06 => { // CRYPTO
                    let mut crypto = Crypto::default();
                    crypto.frame_type = frame_type;
                    debug!("frame_type: {}", crypto.frame_type);
                    crypto.offset = packet.read_length();
                    debug!("offset: {}", crypto.offset);
                    crypto.length = packet.read_length();
                    debug!("length: {}", crypto.length);
                    crypto.end = crypto.offset + crypto.length;
                    crypto.data = packet.read_bytes(crypto.length as usize);
                    crypto_list.push(crypto);
                }
                _ => {
                    warn!("unknown frame_type: {}", frame_type);
                    return None;
                }
            }
        }
        if crypto_list.is_empty() {
            return None;
        }

        // 如果抓到1号包，并且多个offset和length拼起来的包不完整，在没有0号包的时候，无法解密；反之，如果1号包拼起来完整，则可以尝试单独解密
        let mut i = 0;
        let mut reassembled_packet: Vec<u8> = vec![];
        let mut offset = 0;
        loop {
            if let Some(packet) = crypto_list.iter().find(|c| c.offset == offset) {
                reassembled_packet.extend(packet.data.clone());
                i += 1;
                offset = packet.end;
            } else {
                break;
            }
        }

        if crypto_list.len() == i {
            trace!("reassembled_packet: {}", hex::encode(reassembled_packet.clone()));
            Registry::remove(key);
            return self.parse_ext_domain(reassembled_packet);
        } else {
            Registry::set(key, crypto_list);
        }

        None
    }

    fn make_nonce(&mut self, iv: &[u8], counter: u32) -> [u8; 12] {
        let mut nonce = [0; 12];
        nonce.copy_from_slice(iv);

        // XOR the last bytes of the IV with the counter. This is equivalent to
        // left-padding the counter with zero bytes.
        for (a, b) in nonce[8..].iter_mut().zip(counter.to_be_bytes().iter()) {
            *a ^= b;
        }

        nonce
    }

    fn gcm_ctr_decrypt(
        &mut self,
        ciphertext: Vec<u8>,
    ) -> Vec<u8> {
        let mut full_iv = [0u8; 16];
        full_iv[..12].copy_from_slice(
            self.make_nonce(self.client_iv.clone().as_slice(), self.packet_number.clone()).as_slice()
        );
        full_iv[15] = 2;
        let key: [u8; 16] = self.client_key.clone().try_into().unwrap();

        let mut cipher = Ctr128BE::<Aes128>::new(&key.into(), &full_iv.into());
        let mut plaintext = ciphertext.to_vec();
        cipher.apply_keystream(&mut plaintext);

        plaintext
    }

    fn parse_ext_domain(&mut self, data: Vec<u8>) -> Option<String> {
        let mut packet = packet::Packet::new(data);
        let msg_type = packet.read_u8();
        let len = (packet.read_u16() as usize) << 8
            | (packet.read_u8() as usize);
        let legacy_version = packet.read_u16();
        let random = packet.read_bytes(32);

        let sid_len = packet.read_u8() as usize;
        let session = packet.read_bytes(sid_len);
        let cs_len = packet.read_u16() as usize;
        let cipher_suites = packet.read_bytes(cs_len);
        let comp_len = packet.read_u8() as usize;
        let compression_methods = packet.read_bytes(comp_len);
        let ext_len = packet.read_u16() as usize;
        let ext = packet.read_bytes(ext_len);

        let mut ext_packet = packet::Packet::new(ext);

        while !ext_packet.is_empty() {
            let ext_type = ext_packet.read_u16();
            let ext_l = ext_packet.read_u16() as usize;
            let ext_data = ext_packet.read_bytes(ext_l);
            debug!("ext_type={ext_type:#06x}, ext_len={ext_l}, data={:02x?}", ext_data);
            if ext_type == 0 { // server_name
                return Some(String::from_utf8_lossy(&ext_data[5..]).into_owned());
            }
        }
        None
    }

    fn hkdf_expand_label(&mut self, secret: impl AsRef<[u8]>, label: Vec<u8>, length: u16) -> Vec<u8> {
        let mut okm = vec![0; length as usize];
        let mut tls_label = b"tls13 ".to_vec();
        tls_label.extend(label);
        let mut hkdf_label = vec![0u8; 0];
        hkdf_label.extend(length.to_be_bytes());
        hkdf_label.extend((tls_label.len() as u8).to_be_bytes());
        hkdf_label.extend(tls_label);
        hkdf_label.extend([0]);
        trace!("hkdf_label: {}", hex::encode(hkdf_label.clone()));
        HKDF::expand(&mut okm, secret, hkdf_label);
        trace!("OKM (32 bytes): {}", hex::encode(okm.clone()));
        okm.to_vec()
    }
}