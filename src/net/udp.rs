//! UDP (User Datagram Protocol) 모듈
//!
//! 이 모듈은 UDP 프로토콜을 구현합니다.

use crate::net::ethernet::{PacketBuffer, NetworkError};
use crate::net::ip::{Ipv4Address, IpProtocol};
use spin::Mutex;
use alloc::collections::BTreeMap;

/// UDP 포트 번호
pub type UdpPort = u16;

/// UDP 소켓 핸들러 트레이트
pub trait UdpSocketHandler {
    /// UDP 패킷 수신 처리
    fn handle_packet(&mut self, src_ip: Ipv4Address, src_port: UdpPort, data: &[u8]);
}

/// UDP 소켓
struct UdpSocket {
    /// 로컬 포트
    local_port: UdpPort,
    /// 핸들러
    handler: Option<&'static mut dyn UdpSocketHandler>,
}

// Access to handlers is guarded by the global UDP_MANAGER mutex; mark Send/Sync.
unsafe impl Send for UdpSocket {}
unsafe impl Sync for UdpSocket {}

/// UDP 소켓 관리자
struct UdpSocketManager {
    /// 포트 -> 소켓 매핑
    sockets: BTreeMap<UdpPort, UdpSocket>,
}

unsafe impl Send for UdpSocketManager {}
unsafe impl Sync for UdpSocketManager {}

impl UdpSocketManager {
    /// 새 UDP 소켓 관리자 생성
    fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
        }
    }
    
    /// 소켓 바인드
    fn bind(&mut self, port: UdpPort) -> Result<(), NetworkError> {
        if self.sockets.contains_key(&port) {
            return Err(NetworkError::InvalidPacket);
        }
        
        self.sockets.insert(port, UdpSocket {
            local_port: port,
            handler: None,
        });
        
        Ok(())
    }
    
    /// 패킷 처리
    fn handle_packet(
        &mut self,
        src_ip: Ipv4Address,
        src_port: UdpPort,
        dst_port: UdpPort,
        data: &[u8],
    ) {
        if let Some(socket) = self.sockets.get_mut(&dst_port) {
            if let Some(ref mut handler) = socket.handler {
                handler.handle_packet(src_ip, src_port, data);
            }
        }
    }
}

/// 전역 UDP 소켓 관리자
static UDP_MANAGER: Mutex<Option<UdpSocketManager>> = Mutex::new(None);

/// UDP 헤더 구조
#[repr(C, packed)]
pub struct UdpHeader {
    /// 송신자 포트
    src_port: u16,
    /// 수신자 포트
    dst_port: u16,
    /// 길이 (헤더 + 데이터)
    length: u16,
    /// 체크섬
    checksum: u16,
}

impl UdpHeader {
    /// UDP 헤더 크기 (바이트)
    pub const SIZE: usize = 8;
    
    /// 송신자 포트 가져오기
    pub fn src_port(&self) -> UdpPort {
        u16::from_be_bytes([self.src_port as u8, (self.src_port >> 8) as u8])
    }
    
    /// 송신자 포트 설정
    pub fn set_src_port(&mut self, port: UdpPort) {
        self.src_port = u16::to_be(port);
    }
    
    /// 수신자 포트 가져오기
    pub fn dst_port(&self) -> UdpPort {
        u16::from_be_bytes([self.dst_port as u8, (self.dst_port >> 8) as u8])
    }
    
    /// 수신자 포트 설정
    pub fn set_dst_port(&mut self, port: UdpPort) {
        self.dst_port = u16::to_be(port);
    }
    
    /// 길이 가져오기
    pub fn length(&self) -> u16 {
        u16::from_be_bytes([self.length as u8, (self.length >> 8) as u8])
    }
    
    /// 길이 설정
    pub fn set_length(&mut self, length: u16) {
        self.length = u16::to_be(length);
    }
    
    /// 패킷 버퍼에서 UDP 헤더 읽기
    pub fn from_packet(packet: &PacketBuffer) -> Option<&UdpHeader> {
        if packet.length < Self::SIZE {
            return None;
        }
        
        unsafe {
            Some(&*(packet.as_slice().as_ptr() as *const UdpHeader))
        }
    }
    
    /// 패킷 버퍼에서 UDP 헤더 읽기 (가변)
    pub fn from_packet_mut(packet: &mut PacketBuffer) -> Option<&mut UdpHeader> {
        if packet.length < Self::SIZE {
            return None;
        }
        
        unsafe {
            Some(&mut *(packet.as_mut_slice().as_mut_ptr() as *mut UdpHeader))
        }
    }
    
    /// 새 UDP 헤더 생성
    pub fn new(src_port: UdpPort, dst_port: UdpPort, data_length: usize) -> Self {
        Self {
            src_port: u16::to_be(src_port),
            dst_port: u16::to_be(dst_port),
            length: u16::to_be((Self::SIZE + data_length) as u16),
            checksum: 0, // UDP 체크섬은 선택사항
        }
    }
}

/// UDP 패킷 생성
pub fn create_udp_packet(
    buffer: &mut PacketBuffer,
    src_port: UdpPort,
    dst_port: UdpPort,
    data: &[u8],
) -> Result<(), NetworkError> {
    if UdpHeader::SIZE + data.len() > buffer.data.len() {
        return Err(NetworkError::BufferFull);
    }
    
    let header = UdpHeader::new(src_port, dst_port, data.len());
    
    // 헤더 복사
    let header_bytes = unsafe {
        core::slice::from_raw_parts(
            &header as *const UdpHeader as *const u8,
            UdpHeader::SIZE
        )
    };
    buffer.data[..UdpHeader::SIZE].copy_from_slice(header_bytes);
    
    // 데이터 복사
    buffer.data[UdpHeader::SIZE..UdpHeader::SIZE + data.len()].copy_from_slice(data);
    buffer.length = UdpHeader::SIZE + data.len();
    
    Ok(())
}

/// UDP 패킷 처리
///
/// 수신된 UDP 패킷을 처리합니다.
pub fn handle_udp_packet(ip_src: Ipv4Address, packet: &PacketBuffer) -> Result<(), NetworkError> {
    let header = match UdpHeader::from_packet(packet) {
        Some(h) => h,
        None => {
            crate::log_warn!("Invalid UDP packet");
            return Err(NetworkError::InvalidPacket);
        }
    };
    
    let data = &packet.as_slice()[UdpHeader::SIZE..];
    
    // 소켓 관리자에 전달
    let mut manager = UDP_MANAGER.lock();
    if manager.is_none() { *manager = Some(UdpSocketManager::new()); }
    manager.as_mut().unwrap().handle_packet(ip_src, header.src_port(), header.dst_port(), data);
    
    Ok(())
}

/// UDP 소켓 바인드
pub fn bind(port: UdpPort) -> Result<(), NetworkError> {
    let mut manager = UDP_MANAGER.lock();
    if manager.is_none() { *manager = Some(UdpSocketManager::new()); }
    manager.as_mut().unwrap().bind(port)
}

/// UDP 패킷 송신
pub fn send_udp_packet(
    dst_ip: Ipv4Address,
    src_port: UdpPort,
    dst_port: UdpPort,
    data: &[u8],
) -> Result<(), NetworkError> {
    let mut udp_buffer = PacketBuffer::new();
    create_udp_packet(&mut udp_buffer, src_port, dst_port, data)?;
    
    // IP 패킷으로 전송
    crate::net::ip::send_ip_packet(dst_ip, IpProtocol::Udp, udp_buffer.as_slice())
}

