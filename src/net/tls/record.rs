//! TLS 레코드 레이어
//!
//! TLS 레코드 프로토콜을 구현합니다.

use crate::net::tls::TlsVersion;

/// TLS 콘텐츠 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsContentType {
    /// Change Cipher Spec
    ChangeCipherSpec = 20,
    /// Alert
    Alert = 21,
    /// Handshake
    Handshake = 22,
    /// Application Data
    ApplicationData = 23,
}

impl TlsContentType {
    /// 바이트에서 콘텐츠 타입 생성
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            20 => Some(TlsContentType::ChangeCipherSpec),
            21 => Some(TlsContentType::Alert),
            22 => Some(TlsContentType::Handshake),
            23 => Some(TlsContentType::ApplicationData),
            _ => None,
        }
    }
    
    /// 콘텐츠 타입을 바이트로 변환
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
}

/// TLS 레코드
pub struct TlsRecord {
    /// 콘텐츠 타입
    pub content_type: TlsContentType,
    /// TLS 버전
    pub version: TlsVersion,
    /// 데이터 길이
    pub length: u16,
    /// 레코드 데이터
    pub data: alloc::vec::Vec<u8>,
}

impl TlsRecord {
    /// 새 TLS 레코드 생성
    pub fn new(content_type: TlsContentType, version: TlsVersion, data: alloc::vec::Vec<u8>) -> Self {
        let length = data.len().min(0xFFFF) as u16;
        Self {
            content_type,
            version,
            length,
            data,
        }
    }
    
    /// 바이트 배열에서 TLS 레코드 파싱
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 5 {
            return Err("TLS record too short");
        }
        
        let content_type = TlsContentType::from_u8(data[0])
            .ok_or("Invalid TLS content type")?;
        
        let version = u16::from_be_bytes([data[1], data[2]]);
        let version = TlsVersion::from_u16(version)
            .ok_or("Unknown TLS version")?;
        
        let length = u16::from_be_bytes([data[3], data[4]]);
        
        if data.len() < 5 + length as usize {
            return Err("TLS record incomplete");
        }
        
        let record_data = data[5..5 + length as usize].to_vec();
        
        Ok(Self {
            content_type,
            version,
            length,
            data: record_data,
        })
    }
    
    /// TLS 레코드를 바이트 배열로 직렬화
    pub fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        let mut result = alloc::vec::Vec::new();
        result.push(self.content_type.to_u8());
        
        let version_bytes = self.version.to_u16().to_be_bytes();
        result.extend_from_slice(&version_bytes);
        
        let length_bytes = self.length.to_be_bytes();
        result.extend_from_slice(&length_bytes);
        
        result.extend_from_slice(&self.data);
        
        result
    }
}

/// TLS 레코드 타입 (컨텍스트별)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsRecordType {
    /// Change Cipher Spec 레코드
    ChangeCipherSpec,
    /// Alert 레코드
    Alert,
    /// Handshake 레코드
    Handshake,
    /// Application Data 레코드
    ApplicationData,
}

