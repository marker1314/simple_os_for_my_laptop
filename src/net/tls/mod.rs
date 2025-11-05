//! TLS (Transport Layer Security) 모듈
//!
//! 이 모듈은 TLS/SSL 프로토콜을 구현합니다.
//!
//! # 구현 상태
//!
//! - [x] 기본 구조 및 모듈
//! - [ ] TLS 핸드셰이크
//! - [ ] 암호화/복호화
//! - [ ] 인증서 검증
//! - [ ] TLS 1.2/1.3 지원

pub mod record;
pub mod handshake;
pub mod cipher;
pub mod certificate;
pub mod key_exchange;
pub mod rsa;
mod aes;
mod sha;
mod sha256;
pub mod config;
pub mod pem;
pub mod oid;
pub mod crl;

pub use record::{TlsRecord, TlsRecordType, TlsContentType};
pub use handshake::{TlsHandshake, TlsHandshakeType, TlsConnection};
pub use cipher::{TlsCipher, TlsCipherSuite};
pub use certificate::{TlsCertificate, TlsCertificateError};

use crate::net::tcp::TcpPort;
use crate::net::ip::Ipv4Address;
use crate::net::NetworkError;

/// TLS 버전
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsVersion {
    /// TLS 1.0
    Tls10,
    /// TLS 1.1
    Tls11,
    /// TLS 1.2
    Tls12,
    /// TLS 1.3
    Tls13,
}

impl TlsVersion {
    /// 버전을 u16으로 변환 (예: TLS 1.2 = 0x0303)
    pub fn to_u16(&self) -> u16 {
        match self {
            TlsVersion::Tls10 => 0x0301,
            TlsVersion::Tls11 => 0x0302,
            TlsVersion::Tls12 => 0x0303,
            TlsVersion::Tls13 => 0x0304,
        }
    }
    
    /// u16에서 버전 생성
    pub fn from_u16(version: u16) -> Option<Self> {
        match version {
            0x0301 => Some(TlsVersion::Tls10),
            0x0302 => Some(TlsVersion::Tls11),
            0x0303 => Some(TlsVersion::Tls12),
            0x0304 => Some(TlsVersion::Tls13),
            _ => None,
        }
    }
}

/// TLS 에러
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsError {
    /// 잘못된 레코드
    InvalidRecord,
    /// 알 수 없는 버전
    UnknownVersion,
    /// 핸드셰이크 실패
    HandshakeFailed,
    /// 인증서 검증 실패
    CertificateError,
    /// 암호화 오류
    EncryptionError,
    /// 복호화 오류
    DecryptionError,
    /// 네트워크 오류
    NetworkError,
}

impl core::fmt::Display for TlsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TlsError::InvalidRecord => write!(f, "Invalid TLS record"),
            TlsError::UnknownVersion => write!(f, "Unknown TLS version"),
            TlsError::HandshakeFailed => write!(f, "TLS handshake failed"),
            TlsError::CertificateError => write!(f, "TLS certificate error"),
            TlsError::EncryptionError => write!(f, "TLS encryption error"),
            TlsError::DecryptionError => write!(f, "TLS decryption error"),
            TlsError::NetworkError => write!(f, "TLS network error"),
        }
    }
}

impl From<NetworkError> for TlsError {
    fn from(_: NetworkError) -> Self {
        TlsError::NetworkError
    }
}

/// TLS 연결 초기화
/// 
/// # Arguments
/// * `server_ip` - 서버 IP 주소
/// * `server_port` - 서버 포트 (일반적으로 443)
/// 
/// # Returns
/// TLS 연결 핸들 또는 에러
pub fn init_tls_connection(server_ip: Ipv4Address, server_port: TcpPort) -> Result<TlsConnection, TlsError> {
    TlsConnection::new(server_ip, server_port)
}

