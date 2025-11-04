//! TCP (Transmission Control Protocol) 모듈
//!
//! 이 모듈은 TCP 프로토콜을 구현합니다.
//! 현재는 기본 구조만 구현되어 있으며, 완전한 TCP 스택은 향후 구현 예정입니다.

use crate::net::ethernet::{PacketBuffer, NetworkError};
use crate::net::ip::Ipv4Address;

/// TCP 포트 번호
pub type TcpPort = u16;

/// TCP 플래그
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TcpFlags {
    /// SYN 플래그
    pub syn: bool,
    /// ACK 플래그
    pub ack: bool,
    /// FIN 플래그
    pub fin: bool,
    /// RST 플래그
    pub rst: bool,
    /// PSH 플래그
    pub psh: bool,
    /// URG 플래그
    pub urg: bool,
}

impl TcpFlags {
    /// 플래그를 바이트로 변환
    pub fn to_u8(&self) -> u8 {
        let mut flags = 0u8;
        if self.fin { flags |= 0x01; }
        if self.syn { flags |= 0x02; }
        if self.rst { flags |= 0x04; }
        if self.psh { flags |= 0x08; }
        if self.ack { flags |= 0x10; }
        if self.urg { flags |= 0x20; }
        flags
    }
    
    /// 바이트에서 플래그 생성
    pub fn from_u8(flags: u8) -> Self {
        Self {
            fin: (flags & 0x01) != 0,
            syn: (flags & 0x02) != 0,
            rst: (flags & 0x04) != 0,
            psh: (flags & 0x08) != 0,
            ack: (flags & 0x10) != 0,
            urg: (flags & 0x20) != 0,
        }
    }
}

/// TCP 헤더 구조
#[repr(C, packed)]
pub struct TcpHeader {
    /// 송신자 포트
    src_port: u16,
    /// 수신자 포트
    dst_port: u16,
    /// 시퀀스 번호
    sequence: u32,
    /// 확인 번호
    acknowledgment: u32,
    /// 데이터 오프셋 (4비트) + 예약 (3비트) + 플래그 (9비트)
    data_offset_flags: u16,
    /// 윈도우 크기
    window: u16,
    /// 체크섬
    checksum: u16,
    /// 긴급 포인터
    urgent_pointer: u16,
}

impl TcpHeader {
    /// TCP 헤더 기본 크기 (바이트, 옵션 제외)
    pub const BASE_SIZE: usize = 20;
    
    /// 송신자 포트 가져오기
    pub fn src_port(&self) -> TcpPort {
        u16::from_be_bytes([self.src_port as u8, (self.src_port >> 8) as u8])
    }
    
    /// 송신자 포트 설정
    pub fn set_src_port(&mut self, port: TcpPort) {
        self.src_port = u16::to_be(port);
    }
    
    /// 수신자 포트 가져오기
    pub fn dst_port(&self) -> TcpPort {
        u16::from_be_bytes([self.dst_port as u8, (self.dst_port >> 8) as u8])
    }
    
    /// 수신자 포트 설정
    pub fn set_dst_port(&mut self, port: TcpPort) {
        self.dst_port = u16::to_be(port);
    }
    
    /// 시퀀스 번호 가져오기
    pub fn sequence(&self) -> u32 {
        u32::from_be_bytes([
            self.sequence as u8,
            (self.sequence >> 8) as u8,
            (self.sequence >> 16) as u8,
            (self.sequence >> 24) as u8,
        ])
    }
    
    /// 시퀀스 번호 설정
    pub fn set_sequence(&mut self, seq: u32) {
        self.sequence = u32::to_be(seq);
    }
    
    /// 확인 번호 가져오기
    pub fn acknowledgment(&self) -> u32 {
        u32::from_be_bytes([
            self.acknowledgment as u8,
            (self.acknowledgment >> 8) as u8,
            (self.acknowledgment >> 16) as u8,
            (self.acknowledgment >> 24) as u8,
        ])
    }
    
    /// 확인 번호 설정
    pub fn set_acknowledgment(&mut self, ack: u32) {
        self.acknowledgment = u32::to_be(ack);
    }
    
    /// 데이터 오프셋 가져오기 (32비트 워드 단위)
    pub fn data_offset(&self) -> u8 {
        ((u16::from_be_bytes([self.data_offset_flags as u8, (self.data_offset_flags >> 8) as u8]) >> 12) & 0x0F) as u8
    }
    
    /// 헤더 길이 가져오기 (바이트 단위)
    pub fn header_length(&self) -> usize {
        (self.data_offset() * 4) as usize
    }
    
    /// 플래그 가져오기
    pub fn flags(&self) -> TcpFlags {
        let flags = (u16::from_be_bytes([self.data_offset_flags as u8, (self.data_offset_flags >> 8) as u8]) & 0x1FF) as u8;
        TcpFlags::from_u8(flags)
    }
    
    /// 플래그 설정
    pub fn set_flags(&mut self, flags: TcpFlags) {
        let data_offset = self.data_offset();
        self.data_offset_flags = u16::to_be(
            ((data_offset as u16) << 12) | (flags.to_u8() as u16)
        );
    }
    
    /// 윈도우 크기 가져오기
    pub fn window(&self) -> u16 {
        u16::from_be_bytes([self.window as u8, (self.window >> 8) as u8])
    }
    
    /// 윈도우 크기 설정
    pub fn set_window(&mut self, window: u16) {
        self.window = u16::to_be(window);
    }
    
    /// 패킷 버퍼에서 TCP 헤더 읽기
    pub fn from_packet(packet: &PacketBuffer) -> Option<&TcpHeader> {
        if packet.length < Self::BASE_SIZE {
            return None;
        }
        
        unsafe {
            Some(&*(packet.as_slice().as_ptr() as *const TcpHeader))
        }
    }
    
    /// 패킷 버퍼에서 TCP 헤더 읽기 (가변)
    pub fn from_packet_mut(packet: &mut PacketBuffer) -> Option<&mut TcpHeader> {
        if packet.length < Self::BASE_SIZE {
            return None;
        }
        
        unsafe {
            Some(&mut *(packet.as_mut_slice().as_mut_ptr() as *mut TcpHeader))
        }
    }
    
    /// 새 TCP 헤더 생성
    pub fn new(src_port: TcpPort, dst_port: TcpPort, flags: TcpFlags) -> Self {
        let header = Self {
            src_port: u16::to_be(src_port),
            dst_port: u16::to_be(dst_port),
            sequence: 0,
            acknowledgment: 0,
            data_offset_flags: u16::to_be((5 << 12) | (flags.to_u8() as u16)), // 데이터 오프셋 5 (20바이트)
            window: u16::to_be(65535),
            checksum: 0,
            urgent_pointer: 0,
        };
        
        header
    }
}

/// TCP 패킷 처리
///
/// 수신된 TCP 패킷을 처리합니다.
/// 현재는 기본 구조만 구현되어 있으며, 완전한 TCP 연결 관리는 향후 구현 예정입니다.
pub fn handle_tcp_packet(ip_src: Ipv4Address, packet: &PacketBuffer) -> Result<(), NetworkError> {
    let header = match TcpHeader::from_packet(packet) {
        Some(h) => h,
        None => {
            crate::log_warn!("Invalid TCP packet");
            return Err(NetworkError::InvalidPacket);
        }
    };
    
    let flags = header.flags();
    let header_len = header.header_length();
    let data = &packet.as_slice()[header_len..];
    
    crate::log_debug!(
        "TCP packet: {}:{} -> {}:{}, flags: {:?}, len: {}",
        ip_src,
        header.src_port(),
        "local", // TODO: 로컬 IP 주소 가져오기
        header.dst_port(),
        flags,
        data.len()
    );
    
    // TODO: TCP 연결 관리 구현
    // - 연결 상태 관리 (LISTEN, SYN_SENT, SYN_RECEIVED, ESTABLISHED, FIN_WAIT, etc.)
    // - 시퀀스 번호 및 확인 번호 관리
    // - 슬라이딩 윈도우
    // - 재전송 메커니즘
    // - 연결 종료 처리
    
    Ok(())
}

/// TCP 패킷 생성
pub fn create_tcp_packet(
    buffer: &mut PacketBuffer,
    src_port: TcpPort,
    dst_port: TcpPort,
    flags: TcpFlags,
    data: &[u8],
) -> Result<(), NetworkError> {
    if TcpHeader::BASE_SIZE + data.len() > buffer.data.len() {
        return Err(NetworkError::BufferFull);
    }
    
    let mut header = TcpHeader::new(src_port, dst_port, flags);
    header.set_window(65535);
    
    // 헤더 복사
    let header_bytes = unsafe {
        core::slice::from_raw_parts(
            &header as *const TcpHeader as *const u8,
            TcpHeader::BASE_SIZE
        )
    };
    buffer.data[..TcpHeader::BASE_SIZE].copy_from_slice(header_bytes);
    
    // 데이터 복사
    buffer.data[TcpHeader::BASE_SIZE..TcpHeader::BASE_SIZE + data.len()].copy_from_slice(data);
    buffer.length = TcpHeader::BASE_SIZE + data.len();
    
    Ok(())
}

