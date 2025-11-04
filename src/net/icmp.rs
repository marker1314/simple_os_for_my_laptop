//! ICMP (Internet Control Message Protocol) 모듈
//!
//! 이 모듈은 ICMP 프로토콜을 구현합니다.

use crate::net::ethernet::{PacketBuffer, NetworkError};
use crate::net::ip::{Ipv4Address, IpProtocol};

/// ICMP 타입
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcmpType {
    /// Echo Reply (0)
    EchoReply = 0,
    /// Echo Request (8)
    EchoRequest = 8,
    /// Destination Unreachable (3)
    DestinationUnreachable = 3,
    /// Time Exceeded (11)
    TimeExceeded = 11,
    /// 알 수 없는 타입
    Unknown(u8),
}

impl From<u8> for IcmpType {
    fn from(value: u8) -> Self {
        match value {
            0 => IcmpType::EchoReply,
            8 => IcmpType::EchoRequest,
            3 => IcmpType::DestinationUnreachable,
            11 => IcmpType::TimeExceeded,
            n => IcmpType::Unknown(n),
        }
    }
}

impl From<IcmpType> for u8 {
    fn from(icmp_type: IcmpType) -> Self {
        match icmp_type {
            IcmpType::EchoReply => 0,
            IcmpType::EchoRequest => 8,
            IcmpType::DestinationUnreachable => 3,
            IcmpType::TimeExceeded => 11,
            IcmpType::Unknown(n) => n,
        }
    }
}

/// ICMP 코드
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcmpCode {
    /// 기본 코드 (0)
    Default = 0,
    /// 알 수 없는 코드
    Unknown(u8),
}

impl From<u8> for IcmpCode {
    fn from(value: u8) -> Self {
        match value {
            0 => IcmpCode::Default,
            n => IcmpCode::Unknown(n),
        }
    }
}

impl From<IcmpCode> for u8 {
    fn from(code: IcmpCode) -> Self {
        match code {
            IcmpCode::Default => 0,
            IcmpCode::Unknown(n) => n,
        }
    }
}

/// ICMP 헤더 구조
#[repr(C, packed)]
pub struct IcmpHeader {
    /// 타입
    icmp_type: u8,
    /// 코드
    code: u8,
    /// 체크섬
    checksum: u16,
    /// 식별자 (Echo Request/Reply에서 사용)
    identifier: u16,
    /// 시퀀스 번호 (Echo Request/Reply에서 사용)
    sequence: u16,
}

impl IcmpHeader {
    /// ICMP 헤더 크기 (바이트)
    pub const SIZE: usize = 8;
    
    /// 타입 가져오기
    pub fn icmp_type(&self) -> IcmpType {
        IcmpType::from(self.icmp_type)
    }
    
    /// 타입 설정
    pub fn set_icmp_type(&mut self, icmp_type: IcmpType) {
        self.icmp_type = u8::from(icmp_type);
    }
    
    /// 코드 가져오기
    pub fn code(&self) -> IcmpCode {
        IcmpCode::from(self.code)
    }
    
    /// 코드 설정
    pub fn set_code(&mut self, code: IcmpCode) {
        self.code = u8::from(code);
    }
    
    /// 식별자 가져오기
    pub fn identifier(&self) -> u16 {
        u16::from_be_bytes([self.identifier as u8, (self.identifier >> 8) as u8])
    }
    
    /// 식별자 설정
    pub fn set_identifier(&mut self, id: u16) {
        self.identifier = u16::to_be(id);
    }
    
    /// 시퀀스 번호 가져오기
    pub fn sequence(&self) -> u16 {
        u16::from_be_bytes([self.sequence as u8, (self.sequence >> 8) as u8])
    }
    
    /// 시퀀스 번호 설정
    pub fn set_sequence(&mut self, seq: u16) {
        self.sequence = u16::to_be(seq);
    }
    
    /// 체크섬 계산
    pub fn calculate_checksum(&self, payload: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        
        // 헤더를 16비트 워드로 처리
        let header_words = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u16,
                Self::SIZE / 2
            )
        };
        
        for &word in header_words {
            sum += u16::from_be(word) as u32;
        }
        
        // 체크섬 필드는 0으로 간주
        sum -= self.checksum as u32;
        
        // 페이로드를 16비트 워드로 처리
        let payload_chunks = payload.chunks_exact(2);
        let remainder = payload_chunks.remainder();
        
        for chunk in payload_chunks {
            let word = u16::from_be_bytes([chunk[0], chunk[1]]);
            sum += word as u32;
        }
        
        // 홀수 바이트 처리
        if remainder.len() == 1 {
            sum += (remainder[0] as u32) << 8;
        }
        
        // 캐리 비트 처리
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        !sum as u16
    }
    
    /// 체크섬 검증
    pub fn verify_checksum(&self, payload: &[u8]) -> bool {
        self.calculate_checksum(payload) == 0
    }
    
    /// 체크섬 설정
    pub fn set_checksum(&mut self, payload: &[u8]) {
        self.checksum = 0;
        self.checksum = self.calculate_checksum(payload);
    }
    
    /// 패킷 버퍼에서 ICMP 헤더 읽기
    pub fn from_packet(packet: &PacketBuffer) -> Option<&IcmpHeader> {
        if packet.length < Self::SIZE {
            return None;
        }
        
        unsafe {
            Some(&*(packet.as_slice().as_ptr() as *const IcmpHeader))
        }
    }
    
    /// 패킷 버퍼에서 ICMP 헤더 읽기 (가변)
    pub fn from_packet_mut(packet: &mut PacketBuffer) -> Option<&mut IcmpHeader> {
        if packet.length < Self::SIZE {
            return None;
        }
        
        unsafe {
            Some(&mut *(packet.as_mut_slice().as_mut_ptr() as *mut IcmpHeader))
        }
    }
    
    /// 새 ICMP 헤더 생성
    pub fn new(icmp_type: IcmpType, code: IcmpCode, identifier: u16, sequence: u16) -> Self {
        Self {
            icmp_type: u8::from(icmp_type),
            code: u8::from(code),
            checksum: 0,
            identifier: u16::to_be(identifier),
            sequence: u16::to_be(sequence),
        }
    }
}

/// ICMP 패킷 생성
pub fn create_icmp_packet(
    buffer: &mut PacketBuffer,
    icmp_type: IcmpType,
    code: IcmpCode,
    identifier: u16,
    sequence: u16,
    payload: &[u8],
) -> Result<(), NetworkError> {
    if IcmpHeader::SIZE + payload.len() > buffer.data.len() {
        return Err(NetworkError::BufferFull);
    }
    
    let mut header = IcmpHeader::new(icmp_type, code, identifier, sequence);
    header.set_checksum(payload);
    
    // 헤더 복사
    let header_bytes = unsafe {
        core::slice::from_raw_parts(
            &header as *const IcmpHeader as *const u8,
            IcmpHeader::SIZE
        )
    };
    buffer.data[..IcmpHeader::SIZE].copy_from_slice(header_bytes);
    
    // 페이로드 복사
    buffer.data[IcmpHeader::SIZE..IcmpHeader::SIZE + payload.len()].copy_from_slice(payload);
    buffer.length = IcmpHeader::SIZE + payload.len();
    
    Ok(())
}

/// ICMP 패킷 처리
///
/// 수신된 ICMP 패킷을 처리합니다.
pub fn handle_icmp_packet(ip_src: Ipv4Address, packet: &PacketBuffer) -> Result<(), NetworkError> {
    let header = match IcmpHeader::from_packet(packet) {
        Some(h) => h,
        None => {
            crate::log_warn!("Invalid ICMP packet");
            return Err(NetworkError::InvalidPacket);
        }
    };
    
    let payload = &packet.as_slice()[IcmpHeader::SIZE..];
    
    // 체크섬 검증
    if !header.verify_checksum(payload) {
        crate::log_warn!("ICMP packet checksum mismatch");
        return Err(NetworkError::InvalidPacket);
    }
    
    match header.icmp_type() {
        IcmpType::EchoRequest => {
            // Ping 요청 처리
            crate::log_debug!("ICMP Echo Request received from {}", ip_src);
            
            // Echo Reply 생성
            let mut reply_buffer = PacketBuffer::new();
            create_icmp_packet(
                &mut reply_buffer,
                IcmpType::EchoReply,
                IcmpCode::Default,
                header.identifier(),
                header.sequence(),
                payload,
            )?;
            
            // IP 패킷으로 전송
            crate::net::ip::send_ip_packet(ip_src, IpProtocol::Icmp, reply_buffer.as_slice())?;
            
            crate::log_debug!("ICMP Echo Reply sent to {}", ip_src);
        }
        IcmpType::EchoReply => {
            // Ping 응답 처리
            crate::log_debug!("ICMP Echo Reply received from {}", ip_src);
            // TODO: ping 요청과 매칭하여 처리
        }
        _ => {
            crate::log_debug!("ICMP packet type {} not handled", u8::from(header.icmp_type()));
        }
    }
    
    Ok(())
}

/// Ping 전송
///
/// ICMP Echo Request를 전송합니다.
pub fn ping(dst_ip: Ipv4Address, identifier: u16, sequence: u16) -> Result<(), NetworkError> {
    let payload = b"Hello, Simple OS!";
    
    let mut icmp_buffer = PacketBuffer::new();
    create_icmp_packet(
        &mut icmp_buffer,
        IcmpType::EchoRequest,
        IcmpCode::Default,
        identifier,
        sequence,
        payload,
    )?;
    
    // IP 패킷으로 전송
    crate::net::ip::send_ip_packet(dst_ip, IpProtocol::Icmp, icmp_buffer.as_slice())
}

