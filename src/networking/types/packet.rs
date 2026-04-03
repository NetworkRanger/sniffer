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