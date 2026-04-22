use std::io::{Cursor, Read, Seek, SeekFrom};

pub struct Packet {
    cursor: Cursor<Vec<u8>>,
}

#[allow(dead_code)]
impl Packet {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            cursor: Cursor::new(data),
        }
    }
    
    pub fn len(&self) -> usize {
        self.cursor.get_ref().len()
    }
    
    pub fn is_end(&self) -> bool {
        self.cursor.position() == self.cursor.get_ref().len() as u64
    }

    pub fn read_u8(&mut self) -> u8 {
        self.read_bytes(1)[0]
    }
    
    pub fn read_u16(&mut self) -> u16 {
        u16::from_be_bytes(self.read_bytes(2).try_into().unwrap())
    }
    
    pub fn read_h2c_length(&mut self) -> u32 {
        let high = self.read_u8();
        let low = self.read_u16();
        ((high as u32) << 16) | low as u32
    }
    
    pub fn read_u32(&mut self) -> u32 {
        u32::from_be_bytes(self.read_bytes(4).try_into().unwrap())
    }
    
    pub fn read_u64(&mut self) -> u64 {
        u64::from_be_bytes(self.read_bytes(8).try_into().unwrap())
    }
    
    pub fn read_length(&mut self) -> u16 {
        let n = self.read_u8();
        if n >> 4 != 4 {
            return n as u16;
        }
        self.revert(1);
        let n = self.read_u16();
        n & 0x7ff
    }
    
    pub fn read_padding(&mut self) {
        loop {
            if self.read_u8() != 0 {
                self.revert(1);
                break;
            }
        }
    }
    
    pub fn read_bytes(&mut self, len: usize) -> Vec<u8> {
        let mut buf = vec![0u8; len];
        self.cursor.read_exact(&mut buf).unwrap();
        buf
    }
    
    pub fn revert(&mut self, len: i64) {
        let _ = self.cursor.seek(SeekFrom::Current(-len));
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_len() {
        let p = Packet::new(vec![1, 2, 3]);
        assert_eq!(p.len(), 3);
    }

    #[test]
    fn test_is_end_empty() {
        let p = Packet::new(vec![]);
        assert!(p.is_end());
    }

    #[test]
    fn test_is_end_after_read_all() {
        let mut p = Packet::new(vec![0xAA]);
        assert!(!p.is_end());
        p.read_u8();
        assert!(p.is_end());
    }

    #[test]
    fn test_read_u8() {
        let mut p = Packet::new(vec![0x42]);
        assert_eq!(p.read_u8(), 0x42);
    }

    #[test]
    fn test_read_u16() {
        let mut p = Packet::new(vec![0x01, 0x02]);
        assert_eq!(p.read_u16(), 0x0102);
    }

    #[test]
    fn test_read_u32() {
        let mut p = Packet::new(vec![0x00, 0x01, 0x00, 0x02]);
        assert_eq!(p.read_u32(), 0x00010002);
    }

    #[test]
    fn test_read_u64() {
        let mut p = Packet::new(vec![0, 0, 0, 0, 0, 0, 0, 0xFF]);
        assert_eq!(p.read_u64(), 255);
    }

    #[test]
    fn test_read_bytes() {
        let mut p = Packet::new(vec![10, 20, 30, 40]);
        assert_eq!(p.read_bytes(2), vec![10, 20]);
        assert_eq!(p.read_bytes(2), vec![30, 40]);
    }

    #[test]
    fn test_read_h2c_length() {
        // 3-byte big-endian: high=0x01, low=0x0002 → (1 << 16) | 2 = 65538
        let mut p = Packet::new(vec![0x01, 0x00, 0x02]);
        assert_eq!(p.read_h2c_length(), 65538);
    }

    #[test]
    fn test_read_h2c_length_zero() {
        let mut p = Packet::new(vec![0x00, 0x00, 0x00]);
        assert_eq!(p.read_h2c_length(), 0);
    }

    #[test]
    fn test_revert() {
        let mut p = Packet::new(vec![0xAA, 0xBB, 0xCC]);
        p.read_u8(); // pos=1
        p.read_u8(); // pos=2
        p.revert(2); // pos=0
        assert_eq!(p.read_u8(), 0xAA);
    }

    #[test]
    fn test_read_length_small() {
        // n=0x05, high nibble=0 != 4, returns 5 as u16
        let mut p = Packet::new(vec![0x05]);
        assert_eq!(p.read_length(), 5);
    }

    #[test]
    fn test_read_length_two_byte() {
        // n=0x40 → high nibble=4, revert 1, read u16=0x4001, masked &0x7ff = 1
        let mut p = Packet::new(vec![0x40, 0x01]);
        assert_eq!(p.read_length(), 1);
    }

    #[test]
    fn test_read_padding_skips_zeros() {
        let mut p = Packet::new(vec![0x00, 0x00, 0x00, 0x42, 0x99]);
        p.read_padding();
        // Should have consumed 3 zeros, reverted on 0x42
        assert_eq!(p.read_u8(), 0x42);
    }

    #[test]
    fn test_sequential_reads() {
        let mut p = Packet::new(vec![0x01, 0x00, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09]);
        assert_eq!(p.read_u8(), 0x01);
        assert_eq!(p.read_u16(), 0x0002);
        assert_eq!(p.read_u32(), 0x03040506);
        assert_eq!(p.read_bytes(3), vec![0x07, 0x08, 0x09]);
        assert!(p.is_end());
    }
}
