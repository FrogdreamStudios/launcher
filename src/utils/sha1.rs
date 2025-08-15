//! SHA1 hash.

pub struct Sha1 {
    state: [u32; 5],
    buffer: [u8; 64],
    buffer_len: usize,
    total_len: u64,
}

pub struct GenericArray([u8; 20]);

impl AsRef<[u8]> for GenericArray {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Display for GenericArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in &self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

pub trait Digest {
    type OutputSize;
    fn new() -> Self;
    fn update(&mut self, data: &[u8]);
    fn finalize(self) -> GenericArray;
}

impl Sha1 {
    const fn left_rotate(x: u32, n: u32) -> u32 {
        x.rotate_left(n)
    }

    fn process_block(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = Self::left_rotate(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (
            self.state[0],
            self.state[1],
            self.state[2],
            self.state[3],
            self.state[4],
        );
        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let temp = Self::left_rotate(a, 5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = Self::left_rotate(b, 30);
            b = a;
            a = temp;
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
    }
}

impl Digest for Sha1 {
    type OutputSize = [u8; 20];

    fn new() -> Self {
        Self {
            state: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
            buffer: [0; 64],
            buffer_len: 0,
            total_len: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.total_len += data.len() as u64;

        // If we already have some data in the buffer, fill it up
        if self.buffer_len > 0 {
            let need = 64 - self.buffer_len;
            if data.len() >= need {
                self.buffer[self.buffer_len..self.buffer_len + need].copy_from_slice(&data[..need]);
                let buffer_copy = self.buffer;
                self.process_block(&buffer_copy);
                self.buffer_len = 0;
                data = &data[need..];
            } else {
                self.buffer[self.buffer_len..self.buffer_len + data.len()].copy_from_slice(data);
                self.buffer_len += data.len();
                return;
            }
        }

        while data.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[..64]);
            self.process_block(&block);
            data = &data[64..];
        }

        // Save leftover data in buffer
        if !data.is_empty() {
            self.buffer[..data.len()].copy_from_slice(data);
            self.buffer_len = data.len();
        }
    }

    fn finalize(mut self) -> GenericArray {
        let bit_len = self.total_len * 8;

        // Add 0x80
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;

        // 0's pad to 56 bytes
        if self.buffer_len > 56 {
            for i in self.buffer_len..64 {
                self.buffer[i] = 0;
            }
            let buffer_copy = self.buffer;
            self.process_block(&buffer_copy);
            self.buffer = [0; 64];
            self.buffer_len = 0;
        }

        // Add 0's to 56 bytes
        for i in self.buffer_len..56 {
            self.buffer[i] = 0;
        }

        // Add length in bits to the end
        self.buffer[56..64].copy_from_slice(&bit_len.to_be_bytes());
        let buffer_copy = self.buffer;
        self.process_block(&buffer_copy);

        // Result
        let mut out = [0u8; 20];
        for (i, &v) in self.state.iter().enumerate() {
            let bytes = v.to_be_bytes();
            out[i * 4..i * 4 + 4].copy_from_slice(&bytes);
        }
        GenericArray(out)
    }
}

impl Default for Sha1 {
    fn default() -> Self {
        Self::new()
    }
}
