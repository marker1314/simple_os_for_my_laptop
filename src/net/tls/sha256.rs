//! SHA-256 (FIPS 180-4)
//!
//! Minimal, no_std implementation used for certificate verification.

use alloc::vec::Vec;

pub const SHA256_HASH_SIZE: usize = 32;

pub struct Sha256Context {
    state: [u32; 8],
    buffer: [u8; 64],
    buffer_len: usize,
    len_bits: u64,
}

impl Sha256Context {
    pub fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
            ],
            buffer: [0u8; 64],
            buffer_len: 0,
            len_bits: 0,
        }
    }

    #[inline]
    fn rotr(x: u32, n: u32) -> u32 { (x >> n) | (x << (32 - n)) }

    #[inline]
    fn ch(x: u32, y: u32, z: u32) -> u32 { (x & y) ^ (!x & z) }

    #[inline]
    fn maj(x: u32, y: u32, z: u32) -> u32 { (x & y) ^ (x & z) ^ (y & z) }

    #[inline]
    fn bsig0(x: u32) -> u32 { Self::rotr(x, 2) ^ Self::rotr(x, 13) ^ Self::rotr(x, 22) }

    #[inline]
    fn bsig1(x: u32) -> u32 { Self::rotr(x, 6) ^ Self::rotr(x, 11) ^ Self::rotr(x, 25) }

    #[inline]
    fn ssig0(x: u32) -> u32 { Self::rotr(x, 7) ^ Self::rotr(x, 18) ^ (x >> 3) }

    #[inline]
    fn ssig1(x: u32) -> u32 { Self::rotr(x, 17) ^ Self::rotr(x, 19) ^ (x >> 10) }

    fn process_block(&mut self, block: &[u8; 64]) {
        const K: [u32; 64] = [
            0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
            0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
            0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
            0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
            0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
            0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
            0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
            0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
        ];

        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4], block[i * 4 + 1], block[i * 4 + 2], block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            w[i] = Self::ssig1(w[i-2]).wrapping_add(w[i-7]).wrapping_add(Self::ssig0(w[i-15])).wrapping_add(w[i-16]);
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];
        let mut f = self.state[5];
        let mut g = self.state[6];
        let mut h = self.state[7];

        for t in 0..64 {
            let t1 = h
                .wrapping_add(Self::bsig1(e))
                .wrapping_add(Self::ch(e, f, g))
                .wrapping_add(K[t])
                .wrapping_add(w[t]);
            let t2 = Self::bsig0(a).wrapping_add(Self::maj(a, b, c));
            h = g; g = f; f = e; e = d.wrapping_add(t1);
            d = c; c = b; b = a; a = t1.wrapping_add(t2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }

    pub fn update(&mut self, data: &[u8]) {
        self.len_bits = self.len_bits.wrapping_add((data.len() as u64) * 8);
        let mut off = 0;

        if self.buffer_len > 0 {
            let take = core::cmp::min(64 - self.buffer_len, data.len());
            self.buffer[self.buffer_len..self.buffer_len + take]
                .copy_from_slice(&data[..take]);
            self.buffer_len += take;
            off += take;
            if self.buffer_len == 64 {
                let block = self.buffer;
                self.process_block(&block);
                self.buffer_len = 0;
            }
        }

        while off + 64 <= data.len() {
            let block: [u8; 64] = data[off..off + 64].try_into().unwrap();
            self.process_block(&block);
            off += 64;
        }

        if off < data.len() {
            let rem = &data[off..];
            self.buffer[..rem.len()].copy_from_slice(rem);
            self.buffer_len = rem.len();
        }
    }

    pub fn finalize(mut self) -> [u8; SHA256_HASH_SIZE] {
        // padding: 0x80, zeros, length(64-bit big-endian)
        let mut pad = Vec::new();
        pad.push(0x80);
        let len_mod = (self.buffer_len + 1 + 8) % 64;
        let pad_zeros = if len_mod == 0 { 0 } else { 64 - len_mod };
        pad.extend_from_slice(&vec![0u8; pad_zeros]);
        pad.extend_from_slice(&self.len_bits.to_be_bytes());
        self.update(&pad);

        let mut out = [0u8; SHA256_HASH_SIZE];
        for i in 0..8 {
            out[i * 4..i * 4 + 4].copy_from_slice(&self.state[i].to_be_bytes());
        }
        out
    }

    pub fn hash(data: &[u8]) -> [u8; SHA256_HASH_SIZE] {
        let mut ctx = Self::new();
        ctx.update(data);
        ctx.finalize()
    }
}

impl Default for Sha256Context {
    fn default() -> Self { Self::new() }
}


