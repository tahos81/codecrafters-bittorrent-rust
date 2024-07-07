use std::fmt::Display;

#[derive(Debug)]
pub struct BitMap {
    data: Vec<u8>,
}

impl BitMap {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn get(&self, idx: usize) -> bool {
        let byte = idx / 8;
        let bit = idx % 8;
        self.data[byte as usize] & (1 << bit) != 0
    }

    pub fn set(&mut self, idx: usize) {
        let byte = idx / 8;
        let bit = idx % 8;
        if byte >= self.data.len() {
            self.data.resize(byte + 1, 0);
        }
        self.data[byte] |= 1 << bit;
    }

    pub fn unset(&mut self, idx: usize) {
        let byte = idx / 8;
        let bit = idx % 8;
        if byte < self.data.len() {
            self.data[byte] &= !(1 << bit);
        }
    }
}

impl From<Vec<u8>> for BitMap {
    fn from(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Display for BitMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.data {
            for i in 0..8 {
                write!(f, "{}", (byte >> (7 - i)) & 1)?;
            }
        }
        Ok(())
    }
}
