//! SHA (Secure Hash Algorithm) 해시 함수
//!
//! SHA-1, SHA-256을 지원합니다.
//!
//! # 참고 자료
//! - FIPS PUB 180-4: Secure Hash Standard

/// SHA-1 해시 길이 (바이트)
pub const SHA1_HASH_SIZE: usize = 20;

/// SHA-1 컨텍스트
pub struct Sha1Context {
    h: [u32; 5],
    message_len: u64,
    buffer: [u8; 64],
    buffer_len: usize,
}

impl Sha1Context {
    /// 새 SHA-1 컨텍스트 생성
    pub fn new() -> Self {
        Self {
            h: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
            message_len: 0,
            buffer: [0; 64],
            buffer_len: 0,
        }
    }
    
    /// SHA-1 라운드 함수
    fn f(t: usize, b: u32, c: u32, d: u32) -> u32 {
        match t {
            0..=19 => (b & c) | ((!b) & d),
            20..=39 => b ^ c ^ d,
            40..=59 => (b & c) | (b & d) | (c & d),
            60..=79 => b ^ c ^ d,
            _ => 0,
        }
    }
    
    /// SHA-1 상수 K
    fn k(t: usize) -> u32 {
        match t {
            0..=19 => 0x5A827999,
            20..=39 => 0x6ED9EBA1,
            40..=59 => 0x8F1BBCDC,
            60..=79 => 0xCA62C1D6,
            _ => 0,
        }
    }
    
    /// SHA-1 블록 처리
    fn process_block(&mut self, block: &[u8; 64]) {
        // W 배열 생성
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        
        // W 확장
        for i in 16..80 {
            w[i] = (w[i-3] ^ w[i-8] ^ w[i-14] ^ w[i-16]).rotate_left(1);
        }
        
        let mut a = self.h[0];
        let mut b = self.h[1];
        let mut c = self.h[2];
        let mut d = self.h[3];
        let mut e = self.h[4];
        
        // 메인 루프
        for t in 0..80 {
            let temp = a.rotate_left(5)
                .wrapping_add(Self::f(t, b, c, d))
                .wrapping_add(e)
                .wrapping_add(w[t])
                .wrapping_add(Self::k(t));
            
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        
        // 해시 업데이트
        self.h[0] = self.h[0].wrapping_add(a);
        self.h[1] = self.h[1].wrapping_add(b);
        self.h[2] = self.h[2].wrapping_add(c);
        self.h[3] = self.h[3].wrapping_add(d);
        self.h[4] = self.h[4].wrapping_add(e);
    }
    
    /// 데이터 업데이트
    pub fn update(&mut self, data: &[u8]) {
        self.message_len += data.len() as u64;
        
        let mut data_offset = 0;
        
        // 버퍼에 데이터 추가
        if self.buffer_len > 0 {
            let copy_len = (64 - self.buffer_len).min(data.len());
            self.buffer[self.buffer_len..self.buffer_len + copy_len]
                .copy_from_slice(&data[0..copy_len]);
            self.buffer_len += copy_len;
            data_offset = copy_len;
            
            if self.buffer_len == 64 {
                let block: [u8; 64] = self.buffer;
                self.process_block(&block);
                self.buffer_len = 0;
            }
        }
        
        // 전체 블록 처리
        let mut remaining = &data[data_offset..];
        while remaining.len() >= 64 {
            let block: [u8; 64] = remaining[..64].try_into().unwrap();
            self.process_block(&block);
            remaining = &remaining[64..];
        }
        
        // 나머지 버퍼에 저장
        if !remaining.is_empty() {
            self.buffer[..remaining.len()].copy_from_slice(remaining);
            self.buffer_len = remaining.len();
        }
    }
    
    /// 최종 해시 계산
    pub fn finalize(mut self) -> [u8; SHA1_HASH_SIZE] {
        // 패딩 추가
        let mut padding = alloc::vec::Vec::new();
        padding.push(0x80);
        
        let message_len_bits = self.message_len * 8;
        let padding_len = 64 - ((self.buffer_len + 9) % 64);
        padding.extend_from_slice(&vec![0; padding_len]);
        
        // 길이 추가 (64비트, big-endian)
        padding.extend_from_slice(&message_len_bits.to_be_bytes());
        
        self.update(&padding);
        
        // 해시 반환
        let mut hash = [0u8; SHA1_HASH_SIZE];
        for i in 0..5 {
            let bytes = self.h[i].to_be_bytes();
            hash[i * 4..i * 4 + 4].copy_from_slice(&bytes);
        }
        
        hash
    }
    
    /// 데이터의 SHA-1 해시 계산
    pub fn hash(data: &[u8]) -> [u8; SHA1_HASH_SIZE] {
        let mut ctx = Self::new();
        ctx.update(data);
        ctx.finalize()
    }
}

impl Default for Sha1Context {
    fn default() -> Self {
        Self::new()
    }
}

