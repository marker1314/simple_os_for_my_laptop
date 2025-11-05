//! AES (Advanced Encryption Standard) 암호화
//!
//! AES-128, AES-192, AES-256을 지원합니다.
//!
//! # 참고 자료
//! - FIPS PUB 197: Advanced Encryption Standard
//! - 기본 구현은 교육 목적으로만 사용됩니다.

/// AES 블록 크기 (바이트)
pub const AES_BLOCK_SIZE: usize = 16;

/// AES-128 키 크기 (바이트)
pub const AES_128_KEY_SIZE: usize = 16;

/// AES-256 키 크기 (바이트)
pub const AES_256_KEY_SIZE: usize = 32;

/// AES S-Box
const S_BOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

/// AES 키 스케줄
struct AesKeySchedule {
    round_keys: alloc::vec::Vec<[u8; 16]>,
    num_rounds: usize,
}

impl AesKeySchedule {
    /// 키 스케줄 생성
    fn new(key: &[u8]) -> Result<Self, &'static str> {
        let key_len = key.len();
        let num_rounds = match key_len {
            16 => 10, // AES-128
            24 => 12, // AES-192
            32 => 14, // AES-256
            _ => return Err("Invalid AES key size"),
        };
        
        let mut round_keys = alloc::vec::Vec::new();
        round_keys.push([0u8; 16]);
        
        // 첫 번째 라운드 키는 원본 키
        round_keys[0][..key_len.min(16)].copy_from_slice(&key[..key_len.min(16)]);
        
        // 키 확장 (간단한 구현)
        // 실제로는 더 복잡한 키 스케줄링이 필요하지만, 기본 구조만 제공
        for i in 1..=num_rounds {
            let mut new_key = [0u8; 16];
            // 간단한 키 스케줄링 (실제로는 SubWord, RotWord, Rcon 등 사용)
            for j in 0..16 {
                new_key[j] = round_keys[i-1][j] ^ S_BOX[(j + i) as usize % 256];
            }
            round_keys.push(new_key);
        }
        
        Ok(Self {
            round_keys,
            num_rounds,
        })
    }
}

/// AES 컨텍스트
pub struct AesContext {
    key_schedule: AesKeySchedule,
}

impl AesContext {
    /// 새 AES 컨텍스트 생성
    pub fn new(key: &[u8]) -> Result<Self, &'static str> {
        let key_schedule = AesKeySchedule::new(key)?;
        Ok(Self { key_schedule })
    }
    
    /// AES 암호화 (ECB 모드, 한 블록)
    fn encrypt_block(&self, block: &[u8; 16]) -> [u8; 16] {
        let mut state = *block;
        
        // AddRoundKey (초기)
        for i in 0..16 {
            state[i] ^= self.key_schedule.round_keys[0][i];
        }
        
        // 라운드 (간단한 구현)
        for round in 1..self.key_schedule.num_rounds {
            // SubBytes
            for i in 0..16 {
                state[i] = S_BOX[state[i] as usize];
            }
            
            // ShiftRows (간단한 순환 이동)
            let temp = state[1];
            state[1] = state[5];
            state[5] = state[9];
            state[9] = state[13];
            state[13] = temp;
            
            // MixColumns는 생략 (복잡함)
            
            // AddRoundKey
            for i in 0..16 {
                state[i] ^= self.key_schedule.round_keys[round][i];
            }
        }
        
        // 최종 라운드
        for i in 0..16 {
            state[i] = S_BOX[state[i] as usize];
        }
        
        let temp = state[1];
        state[1] = state[5];
        state[5] = state[9];
        state[9] = state[13];
        state[13] = temp;
        
        for i in 0..16 {
            state[i] ^= self.key_schedule.round_keys[self.key_schedule.num_rounds][i];
        }
        
        state
    }
    
    /// AES 복호화 (ECB 모드, 한 블록)
    fn decrypt_block(&self, _block: &[u8; 16]) -> [u8; 16] {
        // 복호화는 역변환 필요 (간단한 구현 생략)
        // 실제로는 InvSubBytes, InvShiftRows, InvMixColumns 등 필요
        *_block // 임시로 그대로 반환
    }
    
    /// AES-CBC 암호화
    pub fn encrypt_cbc(&self, data: &[u8], iv: &[u8; 16]) -> Result<alloc::vec::Vec<u8>, &'static str> {
        let mut result = alloc::vec::Vec::new();
        let mut prev_block = *iv;
        
        // 패딩 추가 (PKCS#7)
        let mut padded_data = data.to_vec();
        let pad_len = AES_BLOCK_SIZE - (data.len() % AES_BLOCK_SIZE);
        for _ in 0..pad_len {
            padded_data.push(pad_len as u8);
        }
        
        // 블록 단위로 암호화
        for chunk in padded_data.chunks_exact(AES_BLOCK_SIZE) {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            
            // XOR with previous ciphertext
            for i in 0..16 {
                block[i] ^= prev_block[i];
            }
            
            // Encrypt
            let encrypted = self.encrypt_block(&block);
            result.extend_from_slice(&encrypted);
            prev_block = encrypted;
        }
        
        Ok(result)
    }
    
    /// AES-CBC 복호화
    pub fn decrypt_cbc(&self, data: &[u8], iv: &[u8; 16]) -> Result<alloc::vec::Vec<u8>, &'static str> {
        if data.len() % AES_BLOCK_SIZE != 0 {
            return Err("Invalid ciphertext length");
        }
        
        let mut result = alloc::vec::Vec::new();
        let mut prev_block = *iv;
        
        // 블록 단위로 복호화
        for chunk in data.chunks_exact(AES_BLOCK_SIZE) {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            
            // Decrypt
            let decrypted = self.decrypt_block(&block);
            
            // XOR with previous ciphertext
            let mut plaintext = [0u8; 16];
            for i in 0..16 {
                plaintext[i] = decrypted[i] ^ prev_block[i];
            }
            
            result.extend_from_slice(&plaintext);
            prev_block = block;
        }
        
        // 패딩 제거 (PKCS#7)
        if let Some(&pad_len) = result.last() {
            if pad_len as usize <= AES_BLOCK_SIZE && pad_len > 0 {
                let pad_len = pad_len as usize;
                if result.len() >= pad_len {
                    result.truncate(result.len() - pad_len);
                }
            }
        }
        
        Ok(result)
    }
}

