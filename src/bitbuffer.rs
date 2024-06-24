use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BitBuffer {
    pub data: Vec<u8>,
    pub read_pos: usize,
    pub num_bits: usize,
}

impl BitBuffer {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }

    pub fn deserialize(input: &[u8]) -> Self {
        bincode::deserialize(input).unwrap()
    }

    pub fn new() -> Self {
        BitBuffer {
            data: Vec::new(),
            read_pos: 0,
            num_bits: 0,
        }
    }

    pub fn write_bit(&mut self, bit: bool) {
        if self.num_bits % 8 == 0 {
            self.data.push(0);
        }
        if bit {
            self.data[self.num_bits / 8] |= 1 << (self.num_bits % 8);
        }
        self.num_bits += 1;
    }

    pub fn write_bits(&mut self, bits: u32, num_bits: u8) {
        for i in 0..num_bits {
            self.write_bit(bits & (1 << i) != 0);
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        for i in 0..8 {
            self.write_bit(byte & (1 << i) != 0);
        }
    }

    pub fn read_bit(&mut self) -> Option<bool> {
        if self.read_pos >= self.num_bits {
            return None;
        }
        let bit = self.data[self.read_pos / 8] & (1 << (self.read_pos % 8)) != 0;
        self.read_pos += 1;
        Some(bit)
    }

    pub fn read_byte(&mut self) -> Option<u8> {
        if self.read_pos + 8 > self.data.len() * 8 {
            return None;
        }
        let mut byte = 0;
        for i in 0..8 {
            if self.read_bit().unwrap() {
                byte |= 1 << i;
            }
        }
        Some(byte)
    }

    pub fn read_bits(&mut self, num_bits: u8) -> Option<u32> {
        if self.read_pos + num_bits as usize > self.num_bits {
            return None;
        }
        let mut bits = 0;
        for i in 0..num_bits {
            if self.read_bit().unwrap() {
                bits |= 1 << i;
            }
        }
        Some(bits)
    }
}