//! 이더넷 프레임 처리 모듈
//!
//! 이 모듈은 이더넷 프레임의 생성 및 파싱을 담당합니다.

use crate::net::ethernet::{MacAddress, PacketBuffer, NetworkError};
use crate::net::ip;
use crate::net::arp;

/// 이더넷 타입 (EtherType)
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtherType {
    /// IPv4 (0x0800)
    Ipv4 = 0x0800,
    /// ARP (0x0806)
    Arp = 0x0806,
    /// 알 수 없는 타입
    Unknown(u16),
}

impl From<u16> for EtherType {
    fn from(value: u16) -> Self {
        match value {
            0x0800 => EtherType::Ipv4,
            0x0806 => EtherType::Arp,
            n => EtherType::Unknown(n),
        }
    }
}

impl From<EtherType> for u16 {
    fn from(ether_type: EtherType) -> Self {
        match ether_type {
            EtherType::Ipv4 => 0x0800,
            EtherType::Arp => 0x0806,
            EtherType::Unknown(n) => n,
        }
    }
}

/// 이더넷 헤더 구조
#[repr(C, packed)]
pub struct EthernetHeader {
    /// 수신자 MAC 주소
    dst_mac: [u8; 6],
    /// 송신자 MAC 주소
    src_mac: [u8; 6],
    /// 이더넷 타입
    ether_type: u16,
}

impl EthernetHeader {
    /// 이더넷 헤더 크기 (바이트)
    pub const SIZE: usize = 14;
    
    /// 수신자 MAC 주소 가져오기
    pub fn dst_mac(&self) -> MacAddress {
        MacAddress(self.dst_mac)
    }
    
    /// 수신자 MAC 주소 설정
    pub fn set_dst_mac(&mut self, mac: MacAddress) {
        self.dst_mac = mac.0;
    }
    
    /// 송신자 MAC 주소 가져오기
    pub fn src_mac(&self) -> MacAddress {
        MacAddress(self.src_mac)
    }
    
    /// 송신자 MAC 주소 설정
    pub fn set_src_mac(&mut self, mac: MacAddress) {
        self.src_mac = mac.0;
    }
    
    /// 이더넷 타입 가져오기
    pub fn ether_type(&self) -> EtherType {
        let ether_type = u16::from_be_bytes([self.ether_type as u8, (self.ether_type >> 8) as u8]);
        EtherType::from(ether_type)
    }
    
    /// 이더넷 타입 설정
    pub fn set_ether_type(&mut self, ether_type: EtherType) {
        self.ether_type = u16::to_be(u16::from(ether_type));
    }
    
    /// 패킷 버퍼에서 이더넷 헤더 읽기
    pub fn from_packet(packet: &PacketBuffer) -> Option<&EthernetHeader> {
        if packet.length < Self::SIZE {
            return None;
        }
        
        unsafe {
            Some(&*(packet.as_slice().as_ptr() as *const EthernetHeader))
        }
    }
    
    /// 패킷 버퍼에서 이더넷 헤더 읽기 (가변)
    pub fn from_packet_mut(packet: &mut PacketBuffer) -> Option<&mut EthernetHeader> {
        if packet.length < Self::SIZE {
            return None;
        }
        
        unsafe {
            Some(&mut *(packet.as_mut_slice().as_mut_ptr() as *mut EthernetHeader))
        }
    }
    
    /// 새 이더넷 헤더 생성
    pub fn new(src_mac: MacAddress, dst_mac: MacAddress, ether_type: EtherType) -> Self {
        Self {
            dst_mac: dst_mac.0,
            src_mac: src_mac.0,
            ether_type: u16::to_be(u16::from(ether_type)),
        }
    }
}

/// 이더넷 프레임 생성
pub fn create_ethernet_frame(
    buffer: &mut PacketBuffer,
    src_mac: MacAddress,
    dst_mac: MacAddress,
    ether_type: EtherType,
    payload: &[u8],
) -> Result<(), NetworkError> {
    if EthernetHeader::SIZE + payload.len() > buffer.data.len() {
        return Err(NetworkError::BufferFull);
    }
    
    let header = EthernetHeader::new(src_mac, dst_mac, ether_type);
    
    // 헤더 복사
    let header_bytes = unsafe {
        core::slice::from_raw_parts(
            &header as *const EthernetHeader as *const u8,
            EthernetHeader::SIZE
        )
    };
    buffer.data[..EthernetHeader::SIZE].copy_from_slice(header_bytes);
    
    // 페이로드 복사
    buffer.data[EthernetHeader::SIZE..EthernetHeader::SIZE + payload.len()].copy_from_slice(payload);
    buffer.length = EthernetHeader::SIZE + payload.len();
    
    Ok(())
}

/// 이더넷 프레임 처리
///
/// 수신된 이더넷 프레임을 처리하고 상위 프로토콜로 전달합니다.
pub fn handle_ethernet_frame(packet: &PacketBuffer) -> Result<(), NetworkError> {
    let header = match EthernetHeader::from_packet(packet) {
        Some(h) => h,
        None => {
            crate::log_warn!("Invalid Ethernet frame");
            return Err(NetworkError::InvalidPacket);
        }
    };
    
    // 로컬 MAC 주소 확인
    let local_mac = match crate::net::get_mac_address() {
        Ok(mac) => mac,
        Err(_) => {
            crate::log_warn!("Failed to get local MAC address");
            return Err(NetworkError::NotInitialized);
        }
    };
    
    // 패킷이 우리에게 오는 것인지 확인
    let dst_mac = header.dst_mac();
    let is_multicast = dst_mac.0[0] & 0x01 != 0;
    if dst_mac != local_mac 
        && dst_mac != MacAddress::BROADCAST
        && !is_multicast {
        // 다른 호스트로 전달할 프레임
        return Ok(());
    }
    
    // 페이로드 추출
    let payload = &packet.as_slice()[EthernetHeader::SIZE..];
    
    // 이더넷 타입별 처리
    match header.ether_type() {
        EtherType::Ipv4 => {
            // IP 패킷 처리
            let ip_buffer = match PacketBuffer::from_slice(payload) {
                Some(buf) => buf,
                None => return Err(NetworkError::InvalidPacket),
            };
            ip::handle_ip_packet(&ip_buffer)
        }
        EtherType::Arp => {
            // ARP 패킷 처리
            let arp_buffer = match PacketBuffer::from_slice(payload) {
                Some(buf) => buf,
                None => return Err(NetworkError::InvalidPacket),
            };
            arp::handle_arp_packet(&arp_buffer)
        }
        EtherType::Unknown(ether_type) => {
            crate::log_debug!("Unknown EtherType: 0x{:04X}", ether_type);
            Ok(())
        }
    }
}

/// IP 패킷을 이더넷 프레임으로 캡슐화하여 전송
pub fn send_ip_over_ethernet(
    dst_ip: crate::net::ip::Ipv4Address,
    ip_packet: &PacketBuffer,
) -> Result<(), NetworkError> {
    // ARP를 통해 MAC 주소 해석
    let dst_mac = match arp::resolve_ip(dst_ip) {
        Some(mac) => mac,
        None => {
            crate::log_warn!("Failed to resolve IP address: {}", dst_ip);
            return Err(NetworkError::InvalidPacket);
        }
    };
    
    // 로컬 MAC 주소 가져오기
    let src_mac = match crate::net::get_mac_address() {
        Ok(mac) => mac,
        Err(_) => {
            return Err(NetworkError::NotInitialized);
        }
    };
    
    // 이더넷 프레임 생성
    let mut ethernet_buffer = PacketBuffer::new();
    create_ethernet_frame(
        &mut ethernet_buffer,
        src_mac,
        dst_mac,
        EtherType::Ipv4,
        ip_packet.as_slice(),
    )?;
    
    // 네트워크 드라이버로 전송
    crate::net::send_packet(&ethernet_buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn ether_type_roundtrip() {
        assert_eq!(u16::from(EtherType::Ipv4), 0x0800);
        assert_eq!(u16::from(EtherType::Arp), 0x0806);
        assert_eq!(EtherType::from(0x0800), EtherType::Ipv4);
    }

    #[test_case]
    fn header_basic() {
        let src = MacAddress([1,2,3,4,5,6]);
        let dst = MacAddress([6,5,4,3,2,1]);
        let mut h = EthernetHeader::new(src, dst, EtherType::Ipv4);
        assert_eq!(h.src_mac(), src);
        assert_eq!(h.dst_mac(), dst);
        assert_eq!(h.ether_type(), EtherType::Ipv4);
    }
}

