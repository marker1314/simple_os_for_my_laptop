//! TLS 암호화
//!
//! TLS 암호 스위트 및 암호화/복호화를 구현합니다.

use crate::net::tls::aes::AesContext;
use crate::net::tls::sha::Sha1Context;

/// TLS 암호 스위트
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsCipherSuite {
    /// TLS_RSA_WITH_AES_128_CBC_SHA
    TlsRsaWithAes128CbcSha = 0x002F,
    /// TLS_RSA_WITH_AES_256_CBC_SHA
    TlsRsaWithAes256CbcSha = 0x0035,
    /// TLS_RSA_WITH_AES_128_GCM_SHA256
    TlsRsaWithAes128GcmSha256 = 0x009C,
    /// TLS_RSA_WITH_AES_256_GCM_SHA384
    TlsRsaWithAes256GcmSha384 = 0x009D,
}

impl TlsCipherSuite {
    /// u16에서 암호 스위트 생성
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x002F => Some(TlsCipherSuite::TlsRsaWithAes128CbcSha),
            0x0035 => Some(TlsCipherSuite::TlsRsaWithAes256CbcSha),
            0x009C => Some(TlsCipherSuite::TlsRsaWithAes128GcmSha256),
            0x009D => Some(TlsCipherSuite::TlsRsaWithAes256GcmSha384),
            _ => None,
        }
    }
    
    /// 암호 스위트를 u16으로 변환
    pub fn to_u16(&self) -> u16 {
        *self as u16
    }
}

/// TLS 암호화 컨텍스트
pub struct TlsCipher {
    /// 암호 스위트
    cipher_suite: TlsCipherSuite,
    /// 암호화 키
    encryption_key: Option<alloc::vec::Vec<u8>>,
    /// MAC 키
    mac_key: Option<alloc::vec::Vec<u8>>,
    /// AES 컨텍스트
    aes_context: Option<AesContext>,
    /// IV (Initialization Vector)
    iv: Option<[u8; 16]>,
}

impl TlsCipher {
    /// 새 암호 컨텍스트 생성
    pub fn new(cipher_suite: TlsCipherSuite) -> Self {
        Self {
            cipher_suite,
            encryption_key: None,
            mac_key: None,
            aes_context: None,
            iv: None,
        }
    }
    
    /// 키 설정
    pub fn set_key(&mut self, encryption_key: alloc::vec::Vec<u8>, mac_key: alloc::vec::Vec<u8>, iv: [u8; 16]) -> Result<(), &'static str> {
        if encryption_key.is_empty() || mac_key.is_empty() {
            return Err("Empty key");
        }
        self.encryption_key = Some(encryption_key.clone());
        self.mac_key = Some(mac_key);
        self.iv = Some(iv);
        
        // AES 컨텍스트 생성
        match self.cipher_suite {
            TlsCipherSuite::TlsRsaWithAes128CbcSha | TlsCipherSuite::TlsRsaWithAes128GcmSha256 => {
                if encryption_key.len() != 16 {
                    return Err("Invalid AES-128 key size");
                }
                self.aes_context = Some(AesContext::new(&encryption_key)?);
            }
            TlsCipherSuite::TlsRsaWithAes256CbcSha | TlsCipherSuite::TlsRsaWithAes256GcmSha384 => {
                if encryption_key.len() != 32 {
                    return Err("Invalid AES-256 key size");
                }
                self.aes_context = Some(AesContext::new(&encryption_key)?);
            }
        }
        
        Ok(())
    }
    
    /// MAC 계산 (HMAC-SHA1)
    fn compute_mac(&self, sequence_num: u64, content_type: u8, data: &[u8]) -> Result<[u8; 20], &'static str> {
        let mac_key = self.mac_key.as_ref().ok_or("MAC key not set")?;
        
        // HMAC-SHA1 계산 (간단한 구현)
        let mut mac_input = alloc::vec::Vec::new();
        mac_input.extend_from_slice(&sequence_num.to_be_bytes());
        mac_input.push(content_type);
        mac_input.extend_from_slice(&(data.len() as u16).to_be_bytes());
        mac_input.extend_from_slice(data);
        
        // HMAC (간단한 구현)
        let mut inner_pad = alloc::vec::Vec::new();
        inner_pad.extend_from_slice(mac_key);
        inner_pad.resize(64, 0x36);
        for i in 0..mac_key.len().min(64) {
            inner_pad[i] ^= 0x36;
        }
        inner_pad.extend_from_slice(&mac_input);
        
        let inner_hash = Sha1Context::hash(&inner_pad);
        
        let mut outer_pad = alloc::vec::Vec::new();
        outer_pad.extend_from_slice(mac_key);
        outer_pad.resize(64, 0x5c);
        for i in 0..mac_key.len().min(64) {
            outer_pad[i] ^= 0x5c;
        }
        outer_pad.extend_from_slice(&inner_hash);
        
        Ok(Sha1Context::hash(&outer_pad))
    }
    
    /// 데이터 암호화
    pub fn encrypt(&self, sequence_num: u64, content_type: u8, data: &[u8]) -> Result<alloc::vec::Vec<u8>, &'static str> {
        let aes_ctx = self.aes_context.as_ref().ok_or("Encryption key not set")?;
        let iv = self.iv.ok_or("IV not set")?;
        
        // MAC 계산
        let mac = self.compute_mac(sequence_num, content_type, data)?;
        
        // 데이터 + MAC
        let mut plaintext = alloc::vec::Vec::new();
        plaintext.extend_from_slice(data);
        plaintext.extend_from_slice(&mac);
        
        // AES-CBC 암호화
        aes_ctx.encrypt_cbc(&plaintext, &iv)
    }
    
    /// 데이터 복호화
    pub fn decrypt(&self, sequence_num: u64, content_type: u8, data: &[u8]) -> Result<alloc::vec::Vec<u8>, &'static str> {
        let aes_ctx = self.aes_context.as_ref().ok_or("Encryption key not set")?;
        let iv = self.iv.ok_or("IV not set")?;
        
        // AES-CBC 복호화
        let mut plaintext = aes_ctx.decrypt_cbc(data, &iv)?;
        
        // MAC 검증
        if plaintext.len() < 20 {
            return Err("Ciphertext too short");
        }
        
        let mac = plaintext.split_off(plaintext.len() - 20);
        let received_mac: [u8; 20] = mac.try_into().unwrap();
        
        let expected_mac = self.compute_mac(sequence_num, content_type, &plaintext)?;
        
        // MAC 비교 (타이밍 공격 방지를 위해 상수 시간 비교가 이상적)
        if received_mac != expected_mac {
            return Err("MAC verification failed");
        }
        
        Ok(plaintext)
    }
    
    /// 암호 스위트 가져오기
    pub fn cipher_suite(&self) -> TlsCipherSuite {
        self.cipher_suite
    }
}

