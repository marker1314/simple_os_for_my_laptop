//! TLS 핸드셰이크
//!
//! TLS 핸드셰이크 프로토콜을 구현합니다.

use crate::net::tcp::TcpPort;
use crate::net::ip::Ipv4Address;
use crate::net::tls::{TlsVersion, TlsError};
use crate::net::tls::cipher::{TlsCipherSuite, TlsCipher};
use crate::net::tls::record::TlsRecord;
use crate::net::tls::certificate::TlsCertificate;
use crate::net::tls::key_exchange::{KeyMaterial, generate_premaster_secret, compute_master_secret, derive_keys, compute_finished_hash};
use crate::net::tls::rsa::{RsaPublicKey, rsa_encrypt_pkcs1_v15};
use alloc::vec::Vec;

/// TLS 핸드셰이크 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsHandshakeType {
    /// Hello Request
    HelloRequest = 0,
    /// Client Hello
    ClientHello = 1,
    /// Server Hello
    ServerHello = 2,
    /// Certificate
    Certificate = 11,
    /// Server Key Exchange
    ServerKeyExchange = 12,
    /// Certificate Request
    CertificateRequest = 13,
    /// Server Hello Done
    ServerHelloDone = 14,
    /// Certificate Verify
    CertificateVerify = 15,
    /// Client Key Exchange
    ClientKeyExchange = 16,
    /// Finished
    Finished = 20,
}

impl TlsHandshakeType {
    /// 바이트에서 핸드셰이크 타입 생성
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(TlsHandshakeType::HelloRequest),
            1 => Some(TlsHandshakeType::ClientHello),
            2 => Some(TlsHandshakeType::ServerHello),
            11 => Some(TlsHandshakeType::Certificate),
            12 => Some(TlsHandshakeType::ServerKeyExchange),
            13 => Some(TlsHandshakeType::CertificateRequest),
            14 => Some(TlsHandshakeType::ServerHelloDone),
            15 => Some(TlsHandshakeType::CertificateVerify),
            16 => Some(TlsHandshakeType::ClientKeyExchange),
            20 => Some(TlsHandshakeType::Finished),
            _ => None,
        }
    }
    
    /// 핸드셰이크 타입을 바이트로 변환
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}

/// TLS 핸드셰이크 메시지
pub struct TlsHandshake {
    /// 메시지 타입
    pub msg_type: TlsHandshakeType,
    /// 메시지 길이 (3바이트)
    pub length: u32,
    /// 메시지 데이터
    pub data: alloc::vec::Vec<u8>,
}

impl TlsHandshake {
    /// 새 핸드셰이크 메시지 생성
    pub fn new(msg_type: TlsHandshakeType, data: alloc::vec::Vec<u8>) -> Self {
        let length = data.len().min(0xFFFFFF) as u32;
        Self {
            msg_type,
            length,
            data,
        }
    }
    
    /// Client Hello 메시지 생성
    pub fn create_client_hello(version: TlsVersion, cipher_suites: &[TlsCipherSuite]) -> Self {
        let mut data = alloc::vec::Vec::new();
        
        // 버전
        data.extend_from_slice(&version.to_u16().to_be_bytes());
        
        // 랜덤 (32바이트)
        let mut random = [0u8; 32];
        // TODO: 실제 랜덤 생성
        data.extend_from_slice(&random);
        
        // 세션 ID 길이 (0 = 새 세션)
        data.push(0);
        
        // 암호 스위트 목록
        let cipher_suites_len = (cipher_suites.len() * 2) as u16;
        data.extend_from_slice(&cipher_suites_len.to_be_bytes());
        for suite in cipher_suites {
            data.extend_from_slice(&suite.to_u16().to_be_bytes());
        }
        
        // 압축 방법 (1 = NULL)
        data.push(1);
        data.push(0);
        
        Self::new(TlsHandshakeType::ClientHello, data)
    }
    
    /// 바이트 배열에서 파싱
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 4 {
            return Err("TLS handshake message too short");
        }
        
        let msg_type = TlsHandshakeType::from_u8(data[0])
            .ok_or("Invalid TLS handshake type")?;
        
        let length = ((data[1] as u32) << 16) | ((data[2] as u32) << 8) | (data[3] as u32);
        
        if data.len() < 4 + length as usize {
            return Err("TLS handshake message incomplete");
        }
        
        let msg_data = data[4..4 + length as usize].to_vec();
        
        Ok(Self {
            msg_type,
            length,
            data: msg_data,
        })
    }
    
    /// 바이트 배열로 직렬화
    pub fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        let mut result = alloc::vec::Vec::new();
        result.push(self.msg_type.to_u8());
        
        let length_bytes = [
            ((self.length >> 16) & 0xFF) as u8,
            ((self.length >> 8) & 0xFF) as u8,
            (self.length & 0xFF) as u8,
        ];
        result.extend_from_slice(&length_bytes);
        
        result.extend_from_slice(&self.data);
        
        result
    }
}

/// TLS 연결 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsConnectionState {
    /// 초기 상태
    Idle,
    /// Client Hello 전송됨
    ClientHelloSent,
    /// Server Hello 수신됨
    ServerHelloReceived,
    /// 핸드셰이크 완료
    HandshakeComplete,
    /// 연결 종료
    Closed,
}

/// TLS 연결
pub struct TlsConnection {
    /// 서버 IP
    server_ip: Ipv4Address,
    /// 서버 포트
    server_port: TcpPort,
    /// TLS 버전
    version: TlsVersion,
    /// 연결 상태
    state: TlsConnectionState,
    /// 선택된 암호 스위트
    cipher_suite: Option<TlsCipherSuite>,
    /// 암호 컨텍스트
    cipher: Option<TlsCipher>,
    /// 클라이언트 랜덤
    client_random: [u8; 32],
    /// 서버 랜덤
    server_random: [u8; 32],
    /// Master Secret
    master_secret: Option<[u8; 48]>,
    /// 키 재료
    key_material: Option<KeyMaterial>,
    /// 서버 인증서
    server_certificate: Option<TlsCertificate>,
    /// 핸드셰이크 메시지 해시
    handshake_hash: [u8; 20],
}

impl TlsConnection {
    /// 새 TLS 연결 생성
    pub fn new(server_ip: Ipv4Address, server_port: TcpPort) -> Result<Self, TlsError> {
        // 클라이언트 랜덤 생성 (실제로는 암호학적으로 안전한 RNG 필요)
        let mut client_random = [0u8; 32];
        // 간단한 구현: 타임스탬프 기반
        let timestamp = crate::drivers::timer::get_milliseconds();
        for i in 0..8 {
            client_random[i] = ((timestamp >> (i * 8)) & 0xFF) as u8;
        }
        
        Ok(Self {
            server_ip,
            server_port,
            version: TlsVersion::Tls12, // 기본 TLS 1.2
            state: TlsConnectionState::Idle,
            cipher_suite: None,
            cipher: None,
            client_random,
            server_random: [0; 32],
            master_secret: None,
            key_material: None,
            server_certificate: None,
            handshake_hash: [0; 20],
        })
    }
    
    /// TLS 핸드셰이크 시작
    pub fn start_handshake(&mut self) -> Result<TlsRecord, TlsError> {
        if self.state != TlsConnectionState::Idle {
            return Err(TlsError::HandshakeFailed);
        }
        
        // 지원하는 암호 스위트 목록
        let cipher_suites = vec![
            TlsCipherSuite::TlsRsaWithAes128CbcSha,
            TlsCipherSuite::TlsRsaWithAes256CbcSha,
        ];
        
        // Client Hello 생성
        let client_hello = TlsHandshake::create_client_hello(self.version, &cipher_suites);
        let handshake_data = client_hello.to_bytes();
        
        // 핸드셰이크 해시 업데이트
        self.update_handshake_hash(&handshake_data);
        
        // TLS 레코드로 래핑
        use crate::net::tls::record::TlsContentType;
        let record = TlsRecord::new(
            TlsContentType::Handshake,
            self.version,
            handshake_data,
        );
        
        self.state = TlsConnectionState::ClientHelloSent;
        Ok(record)
    }
    
    /// 핸드셰이크 해시 업데이트
    fn update_handshake_hash(&mut self, handshake_data: &[u8]) {
        use crate::net::tls::sha::Sha1Context;
        let mut ctx = Sha1Context::new();
        ctx.update(&self.handshake_hash);
        ctx.update(handshake_data);
        self.handshake_hash = ctx.finalize();
    }
    
    /// 서버 응답 처리
    pub fn handle_server_response(&mut self, record: &TlsRecord) -> Result<(), TlsError> {
        use crate::net::tls::record::TlsContentType;
        
        match record.content_type {
            TlsContentType::Handshake => {
                // 핸드셰이크 메시지 파싱
                let handshake = TlsHandshake::from_bytes(&record.data)
                    .map_err(|_| TlsError::InvalidRecord)?;
                
                // 핸드셰이크 해시 업데이트
                self.update_handshake_hash(&record.data);
                
                match handshake.msg_type {
                    TlsHandshakeType::ServerHello => {
                        // Server Hello 처리
                        self.handle_server_hello(&handshake)?;
                    }
                    TlsHandshakeType::Certificate => {
                        // Certificate 처리
                        self.handle_certificate(&handshake)?;
                    }
                    TlsHandshakeType::ServerHelloDone => {
                        // Server Hello Done 처리
                        self.handle_server_hello_done()?;
                    }
                    TlsHandshakeType::Finished => {
                        // Finished 처리
                        self.handle_server_finished(&handshake)?;
                    }
                    _ => {
                        // 기타 핸드셰이크 메시지 처리
                    }
                }
            }
            TlsContentType::Alert => {
                // Alert 처리
                return Err(TlsError::HandshakeFailed);
            }
            _ => {
                return Err(TlsError::InvalidRecord);
            }
        }
        
        Ok(())
    }
    
    /// Server Hello 처리
    fn handle_server_hello(&mut self, handshake: &TlsHandshake) -> Result<(), TlsError> {
        if handshake.data.len() < 38 {
            return Err(TlsError::InvalidRecord);
        }
        
        // 버전 읽기
        let server_version = u16::from_be_bytes([handshake.data[0], handshake.data[1]]);
        let server_version = TlsVersion::from_u16(server_version)
            .ok_or(TlsError::UnknownVersion)?;
        
        // 서버 랜덤 읽기 (32바이트)
        self.server_random.copy_from_slice(&handshake.data[2..34]);
        
        // 세션 ID 길이
        let session_id_len = handshake.data[34] as usize;
        if handshake.data.len() < 35 + session_id_len + 2 {
            return Err(TlsError::InvalidRecord);
        }
        
        // 암호 스위트 선택
        let cipher_suite_bytes = [
            handshake.data[35 + session_id_len],
            handshake.data[35 + session_id_len + 1],
        ];
        let cipher_suite_value = u16::from_be_bytes(cipher_suite_bytes);
        let cipher_suite = TlsCipherSuite::from_u16(cipher_suite_value)
            .ok_or(TlsError::InvalidRecord)?;
        
        self.cipher_suite = Some(cipher_suite);
        self.version = server_version;
        
        crate::log_info!("TLS: Server Hello received, cipher suite: {:?}", cipher_suite);
        
        self.state = TlsConnectionState::ServerHelloReceived;
        Ok(())
    }
    
    /// Certificate 처리
    fn handle_certificate(&mut self, handshake: &TlsHandshake) -> Result<(), TlsError> {
        if handshake.data.len() < 3 {
            return Err(TlsError::InvalidRecord);
        }
        
        // 인증서 목록 길이 (3바이트)
        let cert_list_len = ((handshake.data[0] as u32) << 16)
            | ((handshake.data[1] as u32) << 8)
            | (handshake.data[2] as u32);
        
        if handshake.data.len() < 3 + cert_list_len as usize {
            return Err(TlsError::InvalidRecord);
        }
        
        // 첫 번째 인증서 읽기
        let mut offset = 3;
        if offset + 3 > handshake.data.len() {
            return Err(TlsError::InvalidRecord);
        }
        
        let cert_len = ((handshake.data[offset] as u32) << 16)
            | ((handshake.data[offset + 1] as u32) << 8)
            | (handshake.data[offset + 2] as u32);
        offset += 3;
        
        if offset + cert_len as usize > handshake.data.len() {
            return Err(TlsError::InvalidRecord);
        }
        
        let cert_data = handshake.data[offset..offset + cert_len as usize].to_vec();
        let mut certificate = TlsCertificate::new(cert_data);
        
        // 인증서 검증
        if let Err(e) = certificate.verify() {
            crate::log_warn!("TLS: Certificate verification failed: {}", e);
            if crate::net::tls::config::STRICT_CERT_VERIFY {
                return Err(TlsError::CertificateError);
            }
        }
        
        self.server_certificate = Some(certificate);
        crate::log_info!("TLS: Server certificate received and verified");
        
        Ok(())
    }
    
    /// Server Hello Done 처리
    fn handle_server_hello_done(&mut self) -> Result<(), TlsError> {
        // 키 교환 준비
        // Pre-Master Secret 생성
        let premaster_secret = generate_premaster_secret(&self.client_random, &self.server_random);
        
        // Master Secret 계산
        let master_secret = compute_master_secret(
            &premaster_secret,
            &self.client_random,
            &self.server_random,
        );
        self.master_secret = Some(master_secret);
        
        // 키 유도
        let cipher_suite = self.cipher_suite.ok_or(TlsError::HandshakeFailed)?;
        let key_material = derive_keys(
            &master_secret,
            &self.client_random,
            &self.server_random,
            cipher_suite,
        );
        self.key_material = Some(key_material);
        
        // 암호 컨텍스트 생성
        let mut cipher = TlsCipher::new(cipher_suite);
        let key_material = self.key_material.as_ref().ok_or(TlsError::HandshakeFailed)?;
        
        // 키 설정 (클라이언트 관점)
        cipher.set_key(
            key_material.client_write_key.clone(),
            key_material.client_write_mac_key.clone(),
            key_material.client_write_iv,
        )?;
        
        self.cipher = Some(cipher);
        
        crate::log_info!("TLS: Keys derived, ready for Client Key Exchange");
        
        Ok(())
    }
    
    /// Server Finished 처리
    fn handle_server_finished(&mut self, handshake: &TlsHandshake) -> Result<(), TlsError> {
        if handshake.data.len() < 12 {
            return Err(TlsError::InvalidRecord);
        }
        
        // Finished 메시지 검증
        let master_secret = self.master_secret.ok_or(TlsError::HandshakeFailed)?;
        let server_finished = compute_finished_hash(
            &master_secret,
            b"server finished",
            &self.handshake_hash,
        );
        
        let received_finished = &handshake.data[0..12];
        if received_finished != server_finished {
            crate::log_error!("TLS: Server Finished verification failed");
            return Err(TlsError::HandshakeFailed);
        }
        
        crate::log_info!("TLS: Server Finished verified");
        
        // 클라이언트 Finished 전송 준비
        // (실제로는 별도 메서드로 전송)
        
        self.state = TlsConnectionState::HandshakeComplete;
        Ok(())
    }
    
    /// Client Key Exchange 메시지 생성
    pub fn create_client_key_exchange(&self) -> Result<TlsHandshake, TlsError> {
        // Pre-Master Secret 생성 (이미 handle_server_hello_done에서 생성됨)
        let premaster_secret = generate_premaster_secret(&self.client_random, &self.server_random);
        
        // RSA 암호화
        let encrypted_premaster = if let Some(cert) = &self.server_certificate {
            // 인증서에서 RSA 공개키 추출 시도
            match RsaPublicKey::from_certificate(cert.data()) {
                Ok(public_key) => {
                    // RSA PKCS#1 v1.5 암호화
                    rsa_encrypt_pkcs1_v15(&public_key, &premaster_secret)?
                }
                Err(_) => {
                    // 공개키 추출 실패 시 경고하고 플레이스홀더 사용
                    crate::log_warn!("TLS: Failed to extract RSA public key, using placeholder");
                    premaster_secret.to_vec()
                }
            }
        } else {
            // 인증서가 없으면 플레이스홀더 사용
            crate::log_warn!("TLS: No server certificate, using unencrypted Pre-Master Secret");
            premaster_secret.to_vec()
        };
        
        // Client Key Exchange 메시지 생성
        let mut data = Vec::new();
        data.push((encrypted_premaster.len() >> 8) as u8);
        data.push(encrypted_premaster.len() as u8);
        data.extend_from_slice(&encrypted_premaster);
        
        let handshake = TlsHandshake::new(TlsHandshakeType::ClientKeyExchange, data);
        Ok(handshake)
    }
    
    /// Client Finished 메시지 생성
    pub fn create_client_finished(&self) -> Result<TlsHandshake, TlsError> {
        let master_secret = self.master_secret.ok_or(TlsError::HandshakeFailed)?;
        
        // Finished 해시 계산
        let finished = compute_finished_hash(
            &master_secret,
            b"client finished",
            &self.handshake_hash,
        );
        
        let handshake = TlsHandshake::new(TlsHandshakeType::Finished, finished.to_vec());
        Ok(handshake)
    }
    
    /// 연결 상태 가져오기
    pub fn state(&self) -> TlsConnectionState {
        self.state
    }
    
    /// 암호 컨텍스트 가져오기
    pub fn cipher(&self) -> Option<&TlsCipher> {
        self.cipher.as_ref()
    }
}

