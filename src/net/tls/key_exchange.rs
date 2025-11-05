//! TLS 키 교환
//!
//! TLS 핸드셰이크에서 키 교환을 처리합니다.

use crate::net::tls::cipher::TlsCipherSuite;
use crate::net::tls::sha::Sha1Context;

/// Pre-Master Secret 크기 (RSA의 경우)
pub const PREMASTER_SECRET_SIZE: usize = 48;

/// Master Secret 크기
pub const MASTER_SECRET_SIZE: usize = 48;

/// Pre-Master Secret 생성 (RSA)
///
/// # Arguments
/// * `client_random` - 클라이언트 랜덤 (32바이트)
/// * `server_random` - 서버 랜덤 (32바이트)
pub fn generate_premaster_secret(client_random: &[u8; 32], server_random: &[u8; 32]) -> [u8; PREMASTER_SECRET_SIZE] {
    // Pre-Master Secret 구조:
    // [0-1]: TLS 버전 (0x03, 0x03 = TLS 1.2)
    // [2-47]: 랜덤 (46바이트)
    let mut premaster = [0u8; PREMASTER_SECRET_SIZE];
    
    // 버전
    premaster[0] = 0x03;
    premaster[1] = 0x03;
    
    // 랜덤 생성 (클라이언트와 서버 랜덤을 조합)
    // 실제로는 암호학적으로 안전한 랜덤 생성기가 필요
    for i in 0..46 {
        premaster[2 + i] = client_random[i % 32] ^ server_random[i % 32];
    }
    
    premaster
}

/// Master Secret 계산
///
/// PRF (Pseudo-Random Function)를 사용하여 Master Secret을 계산합니다.
///
/// # Arguments
/// * `premaster_secret` - Pre-Master Secret
/// * `client_random` - 클라이언트 랜덤
/// * `server_random` - 서버 랜덤
pub fn compute_master_secret(
    premaster_secret: &[u8; PREMASTER_SECRET_SIZE],
    client_random: &[u8; 32],
    server_random: &[u8; 32],
) -> [u8; MASTER_SECRET_SIZE] {
    // TLS 1.2 PRF: P_hash(secret, seed) = HMAC_hash(secret, A(1) + seed) || 
    //                                  HMAC_hash(secret, A(2) + seed) || ...
    // where A(0) = seed, A(i) = HMAC_hash(secret, A(i-1))
    
    // Seed = "master secret" + client_random + server_random
    let mut seed = alloc::vec::Vec::new();
    seed.extend_from_slice(b"master secret");
    seed.extend_from_slice(client_random);
    seed.extend_from_slice(server_random);
    
    // P_SHA1(secret, seed) 구현
    // P_hash(secret, seed) = HMAC_hash(secret, A(1) + seed) || HMAC_hash(secret, A(2) + seed) || ...
    // where A(0) = seed, A(i) = HMAC_hash(secret, A(i-1))
    let mut master_secret = [0u8; MASTER_SECRET_SIZE];
    let mut a = seed.clone();
    
    for i in 0..((MASTER_SECRET_SIZE + 19) / 20) {
        // A(i+1) = HMAC-SHA1(secret, A(i))
        let a_hmac = compute_hmac_sha1(premaster_secret, &a);
        a = a_hmac.to_vec();
        
        // HMAC-SHA1(secret, A(i+1) + seed)
        let mut prf_input = alloc::vec::Vec::new();
        prf_input.extend_from_slice(&a);
        prf_input.extend_from_slice(&seed);
        let prf_output = compute_hmac_sha1(premaster_secret, &prf_input);
        
        // Master Secret에 추가
        let start = i * 20;
        let end = (start + 20).min(MASTER_SECRET_SIZE);
        master_secret[start..end].copy_from_slice(&prf_output[..end - start]);
    }
    
    master_secret
}

/// 키 유도 (Key Derivation)
///
/// Master Secret에서 암호화 키, MAC 키, IV를 생성합니다.
///
/// # Arguments
/// * `master_secret` - Master Secret
/// * `client_random` - 클라이언트 랜덤
/// * `server_random` - 서버 랜덤
/// * `cipher_suite` - 암호 스위트
pub fn derive_keys(
    master_secret: &[u8; MASTER_SECRET_SIZE],
    client_random: &[u8; 32],
    server_random: &[u8; 32],
    cipher_suite: TlsCipherSuite,
) -> KeyMaterial {
    // 키 길이 결정
    let (key_length, mac_length, iv_length) = match cipher_suite {
        TlsCipherSuite::TlsRsaWithAes128CbcSha => (16, 20, 16),  // AES-128, SHA-1
        TlsCipherSuite::TlsRsaWithAes256CbcSha => (32, 20, 16),  // AES-256, SHA-1
        TlsCipherSuite::TlsRsaWithAes128GcmSha256 => (16, 0, 4),  // AES-128-GCM
        TlsCipherSuite::TlsRsaWithAes256GcmSha384 => (32, 0, 4),  // AES-256-GCM
    };
    
    // Seed = "key expansion" + server_random + client_random
    let mut seed = alloc::vec::Vec::new();
    seed.extend_from_slice(b"key expansion");
    seed.extend_from_slice(server_random);
    seed.extend_from_slice(client_random);
    
    // 필요한 키 재료 길이
    let key_block_length = (key_length * 2) + (mac_length * 2) + (iv_length * 2);
    
    // P_SHA1로 키 블록 생성
    let mut key_block = alloc::vec::Vec::new();
    let mut a = seed.clone();
    
    while key_block.len() < key_block_length {
        // A(i) = HMAC-SHA1(secret, A(i-1))
        let a_hmac = compute_hmac_sha1(master_secret, &a);
        a = a_hmac.to_vec();
        
        // HMAC-SHA1(secret, A(i) + seed)
        let mut prf_input = alloc::vec::Vec::new();
        prf_input.extend_from_slice(&a);
        prf_input.extend_from_slice(&seed);
        let prf_output = compute_hmac_sha1(master_secret, &prf_input);
        
        key_block.extend_from_slice(&prf_output);
    }
    
    // 키 블록에서 키 분리
    let mut offset = 0;
    
    let client_write_mac_key = key_block[offset..offset + mac_length].to_vec();
    offset += mac_length;
    
    let server_write_mac_key = key_block[offset..offset + mac_length].to_vec();
    offset += mac_length;
    
    let client_write_key = key_block[offset..offset + key_length].to_vec();
    offset += key_length;
    
    let server_write_key = key_block[offset..offset + key_length].to_vec();
    offset += key_length;
    
    let client_write_iv = key_block[offset..offset + iv_length].to_vec();
    offset += iv_length;
    
    let server_write_iv = key_block[offset..offset + iv_length].to_vec();
    
    KeyMaterial {
        client_write_mac_key,
        server_write_mac_key,
        client_write_key,
        server_write_key,
        client_write_iv: client_write_iv.try_into().unwrap_or([0; 16]),
        server_write_iv: server_write_iv.try_into().unwrap_or([0; 16]),
    }
}

/// 키 재료
#[derive(Debug, Clone)]
pub struct KeyMaterial {
    /// 클라이언트 쓰기 MAC 키
    pub client_write_mac_key: alloc::vec::Vec<u8>,
    /// 서버 쓰기 MAC 키
    pub server_write_mac_key: alloc::vec::Vec<u8>,
    /// 클라이언트 쓰기 암호화 키
    pub client_write_key: alloc::vec::Vec<u8>,
    /// 서버 쓰기 암호화 키
    pub server_write_key: alloc::vec::Vec<u8>,
    /// 클라이언트 쓰기 IV
    pub client_write_iv: [u8; 16],
    /// 서버 쓰기 IV
    pub server_write_iv: [u8; 16],
}

/// HMAC-SHA1 계산
fn compute_hmac_sha1(key: &[u8], data: &[u8]) -> [u8; 20] {
    use crate::net::tls::sha::Sha1Context;
    
    // HMAC-SHA1 구현
    let mut inner_pad = alloc::vec::Vec::new();
    inner_pad.extend_from_slice(key);
    inner_pad.resize(64, 0x36);
    for i in 0..key.len().min(64) {
        inner_pad[i] ^= 0x36;
    }
    inner_pad.extend_from_slice(data);
    
    let inner_hash = Sha1Context::hash(&inner_pad);
    
    let mut outer_pad = alloc::vec::Vec::new();
    outer_pad.extend_from_slice(key);
    outer_pad.resize(64, 0x5c);
    for i in 0..key.len().min(64) {
        outer_pad[i] ^= 0x5c;
    }
    outer_pad.extend_from_slice(&inner_hash);
    
    Sha1Context::hash(&outer_pad)
}

/// Finished 메시지 해시 계산
///
/// # Arguments
/// * `master_secret` - Master Secret
/// * `label` - 라벨 ("client finished" 또는 "server finished")
/// * `handshake_hash` - 핸드셰이크 메시지들의 해시
pub fn compute_finished_hash(
    master_secret: &[u8; MASTER_SECRET_SIZE],
    label: &[u8],
    handshake_hash: &[u8; 20],
) -> [u8; 12] {
    // Seed = label + handshake_hash
    let mut seed = alloc::vec::Vec::new();
    seed.extend_from_slice(label);
    seed.extend_from_slice(handshake_hash);
    
    // PRF(secret, label + handshake_hash)[0..12]
    let mut a = seed.clone();
    let mut finished = [0u8; 12];
    
    // A(1) = HMAC-SHA1(secret, seed)
    let a_hmac = compute_hmac_sha1(master_secret, &a);
    a = a_hmac.to_vec();
    
    // HMAC-SHA1(secret, A(1) + seed)
    let mut prf_input = alloc::vec::Vec::new();
    prf_input.extend_from_slice(&a);
    prf_input.extend_from_slice(&seed);
    let prf_output = compute_hmac_sha1(master_secret, &prf_input);
    
    finished.copy_from_slice(&prf_output[..12]);
    finished
}

