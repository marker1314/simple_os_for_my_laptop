//! ARP (Address Resolution Protocol) 모듈
//!
//! 이 모듈은 IP 주소를 MAC 주소로 해석하는 ARP 프로토콜을 구현합니다.

use crate::net::ethernet::{MacAddress, PacketBuffer, NetworkError};
use crate::net::ip::Ipv4Address;
use spin::Mutex;
use alloc::collections::BTreeMap;

/// ARP 테이블 엔트리
#[derive(Debug, Clone)]
struct ArpEntry {
    /// MAC 주소
    mac: MacAddress,
    /// 타임스탬프 (밀리초)
    timestamp: u64,
}

/// ARP 테이블
///
/// IP 주소를 MAC 주소로 매핑하는 테이블입니다.
struct ArpTable {
    /// IP 주소 -> MAC 주소 매핑
    entries: BTreeMap<Ipv4Address, ArpEntry>,
    /// 엔트리 만료 시간 (밀리초, 기본 5분)
    timeout: u64,
}

impl ArpTable {
    /// 새 ARP 테이블 생성
    fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            timeout: 300_000, // 5분
        }
    }
    
    /// IP 주소 해석
    ///
    /// ARP 테이블에서 IP 주소에 해당하는 MAC 주소를 찾습니다.
    fn resolve(&mut self, ip: Ipv4Address) -> Option<MacAddress> {
        // 만료된 엔트리 제거
        self.cleanup();
        
        self.entries.get(&ip).map(|entry| entry.mac)
    }
    
    /// ARP 엔트리 추가/업데이트
    fn insert(&mut self, ip: Ipv4Address, mac: MacAddress) {
        let timestamp = crate::drivers::timer::get_milliseconds();
        self.entries.insert(ip, ArpEntry { mac, timestamp });
    }
    
    /// 만료된 엔트리 제거
    fn cleanup(&mut self) {
        let now = crate::drivers::timer::get_milliseconds();
        self.entries.retain(|_, entry| {
            now.wrapping_sub(entry.timestamp) < self.timeout
        });
    }
}

/// 전역 ARP 테이블
static ARP_TABLE: Mutex<Option<ArpTable>> = Mutex::new(None);

/// ARP 하드웨어 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArpHardwareType {
    /// 이더넷 (1)
    Ethernet = 1,
}

/// ARP 동작 코드
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArpOperation {
    /// ARP 요청 (1)
    Request = 1,
    /// ARP 응답 (2)
    Reply = 2,
}

/// ARP 헤더 구조
#[repr(C, packed)]
pub struct ArpHeader {
    /// 하드웨어 타입
    hardware_type: u16,
    /// 프로토콜 타입
    protocol_type: u16,
    /// 하드웨어 주소 길이
    hardware_length: u8,
    /// 프로토콜 주소 길이
    protocol_length: u8,
    /// 동작 코드
    operation: u16,
    /// 송신자 하드웨어 주소 (MAC)
    sender_hardware_addr: [u8; 6],
    /// 송신자 프로토콜 주소 (IP)
    sender_protocol_addr: [u8; 4],
    /// 수신자 하드웨어 주소 (MAC)
    target_hardware_addr: [u8; 6],
    /// 수신자 프로토콜 주소 (IP)
    target_protocol_addr: [u8; 4],
}

impl ArpHeader {
    /// ARP 헤더 크기 (바이트)
    pub const SIZE: usize = 28;
    
    /// 하드웨어 타입 가져오기
    pub fn hardware_type(&self) -> u16 {
        u16::from_be_bytes([self.hardware_type as u8, (self.hardware_type >> 8) as u8])
    }
    
    /// 프로토콜 타입 가져오기 (0x0800 = IPv4)
    pub fn protocol_type(&self) -> u16 {
        u16::from_be_bytes([self.protocol_type as u8, (self.protocol_type >> 8) as u8])
    }
    
    /// 동작 코드 가져오기
    pub fn operation(&self) -> ArpOperation {
        let op = u16::from_be_bytes([self.operation as u8, (self.operation >> 8) as u8]);
        match op {
            1 => ArpOperation::Request,
            2 => ArpOperation::Reply,
            _ => ArpOperation::Request, // 기본값
        }
    }
    
    /// 동작 코드 설정
    pub fn set_operation(&mut self, op: ArpOperation) {
        self.operation = u16::to_be(op as u16);
    }
    
    /// 송신자 MAC 주소 가져오기
    pub fn sender_mac(&self) -> MacAddress {
        MacAddress(self.sender_hardware_addr)
    }
    
    /// 송신자 MAC 주소 설정
    pub fn set_sender_mac(&mut self, mac: MacAddress) {
        self.sender_hardware_addr = mac.0;
    }
    
    /// 송신자 IP 주소 가져오기
    pub fn sender_ip(&self) -> Ipv4Address {
        Ipv4Address(self.sender_protocol_addr)
    }
    
    /// 송신자 IP 주소 설정
    pub fn set_sender_ip(&mut self, ip: Ipv4Address) {
        self.sender_protocol_addr = ip.0;
    }
    
    /// 수신자 MAC 주소 가져오기
    pub fn target_mac(&self) -> MacAddress {
        MacAddress(self.target_hardware_addr)
    }
    
    /// 수신자 MAC 주소 설정
    pub fn set_target_mac(&mut self, mac: MacAddress) {
        self.target_hardware_addr = mac.0;
    }
    
    /// 수신자 IP 주소 가져오기
    pub fn target_ip(&self) -> Ipv4Address {
        Ipv4Address(self.target_protocol_addr)
    }
    
    /// 수신자 IP 주소 설정
    pub fn set_target_ip(&mut self, ip: Ipv4Address) {
        self.target_protocol_addr = ip.0;
    }
    
    /// 패킷 버퍼에서 ARP 헤더 읽기
    pub fn from_packet(packet: &PacketBuffer) -> Option<&ArpHeader> {
        if packet.length < Self::SIZE {
            return None;
        }
        
        unsafe {
            Some(&*(packet.as_slice().as_ptr() as *const ArpHeader))
        }
    }
    
    /// 패킷 버퍼에서 ARP 헤더 읽기 (가변)
    pub fn from_packet_mut(packet: &mut PacketBuffer) -> Option<&mut ArpHeader> {
        if packet.length < Self::SIZE {
            return None;
        }
        
        unsafe {
            Some(&mut *(packet.as_mut_slice().as_mut_ptr() as *mut ArpHeader))
        }
    }
    
    /// 새 ARP 요청 헤더 생성
    pub fn new_request(
        sender_mac: MacAddress,
        sender_ip: Ipv4Address,
        target_ip: Ipv4Address,
    ) -> Self {
        let header = Self {
            hardware_type: u16::to_be(ArpHardwareType::Ethernet as u16),
            protocol_type: u16::to_be(0x0800), // IPv4
            hardware_length: 6,
            protocol_length: 4,
            operation: u16::to_be(ArpOperation::Request as u16),
            sender_hardware_addr: sender_mac.0,
            sender_protocol_addr: sender_ip.0,
            target_hardware_addr: [0; 6],
            target_protocol_addr: target_ip.0,
        };
        
        header
    }
    
    /// 새 ARP 응답 헤더 생성
    pub fn new_reply(
        sender_mac: MacAddress,
        sender_ip: Ipv4Address,
        target_mac: MacAddress,
        target_ip: Ipv4Address,
    ) -> Self {
        let header = Self {
            hardware_type: u16::to_be(ArpHardwareType::Ethernet as u16),
            protocol_type: u16::to_be(0x0800), // IPv4
            hardware_length: 6,
            protocol_length: 4,
            operation: u16::to_be(ArpOperation::Reply as u16),
            sender_hardware_addr: sender_mac.0,
            sender_protocol_addr: sender_ip.0,
            target_hardware_addr: target_mac.0,
            target_protocol_addr: target_ip.0,
        };
        
        header
    }
}

/// ARP 패킷 생성
pub fn create_arp_packet(
    buffer: &mut PacketBuffer,
    operation: ArpOperation,
    sender_mac: MacAddress,
    sender_ip: Ipv4Address,
    target_mac: MacAddress,
    target_ip: Ipv4Address,
) -> Result<(), NetworkError> {
    if ArpHeader::SIZE > buffer.data.len() {
        return Err(NetworkError::BufferFull);
    }
    
    let header = match operation {
        ArpOperation::Request => {
            ArpHeader::new_request(sender_mac, sender_ip, target_ip)
        }
        ArpOperation::Reply => {
            ArpHeader::new_reply(sender_mac, sender_ip, target_mac, target_ip)
        }
    };
    
    // 헤더 복사
    let header_bytes = unsafe {
        core::slice::from_raw_parts(
            &header as *const ArpHeader as *const u8,
            ArpHeader::SIZE
        )
    };
    buffer.data[..ArpHeader::SIZE].copy_from_slice(header_bytes);
    buffer.length = ArpHeader::SIZE;
    
    Ok(())
}

/// ARP 패킷 처리
///
/// 수신된 ARP 패킷을 처리합니다.
pub fn handle_arp_packet(packet: &PacketBuffer) -> Result<(), NetworkError> {
    let header = match ArpHeader::from_packet(packet) {
        Some(h) => h,
        None => {
            crate::log_warn!("Invalid ARP packet");
            return Err(NetworkError::InvalidPacket);
        }
    };
    
    // 이더넷/IPv4만 지원
    if header.hardware_type() != ArpHardwareType::Ethernet as u16 {
        crate::log_warn!("Unsupported ARP hardware type: {}", header.hardware_type());
        return Err(NetworkError::InvalidPacket);
    }
    
    if header.protocol_type() != 0x0800 {
        crate::log_warn!("Unsupported ARP protocol type: 0x{:04X}", header.protocol_type());
        return Err(NetworkError::InvalidPacket);
    }
    
    // ARP 테이블 업데이트
    let mut arp_table = ARP_TABLE.lock();
    if arp_table.is_none() { *arp_table = Some(ArpTable::new()); }
    arp_table.as_mut().unwrap().insert(header.sender_ip(), header.sender_mac());
    
    // 로컬 IP 주소 (임시로 하드코딩)
    // TODO: 네트워크 인터페이스 설정에서 가져오기
    let local_ip = Ipv4Address([192, 168, 1, 100]);
    let local_mac = match crate::net::get_mac_address() {
        Ok(mac) => mac,
        Err(_) => {
            crate::log_warn!("Failed to get local MAC address");
            return Err(NetworkError::NotInitialized);
        }
    };
    
    match header.operation() {
        ArpOperation::Request => {
            // ARP 요청 처리
            if header.target_ip() == local_ip {
                // 우리에게 온 요청이므로 응답 전송
                crate::log_debug!("ARP request for {} from {}", local_ip, header.sender_ip());
                
                let mut reply_buffer = PacketBuffer::new();
                create_arp_packet(
                    &mut reply_buffer,
                    ArpOperation::Reply,
                    local_mac,
                    local_ip,
                    header.sender_mac(),
                    header.sender_ip(),
                )?;
                
                // 이더넷 프레임으로 전송
                // TODO: 이더넷 프레임 생성 및 전송 구현
                crate::log_debug!("ARP reply sent");
            }
        }
        ArpOperation::Reply => {
            // ARP 응답 처리
            crate::log_debug!("ARP reply from {} ({})", header.sender_ip(), header.sender_mac());
        }
    }
    
    Ok(())
}

/// IP 주소 해석
///
/// ARP를 사용하여 IP 주소를 MAC 주소로 해석합니다.
/// ARP 테이블에 엔트리가 있으면 즉시 반환하고,
/// 없으면 ARP 요청을 전송합니다.
pub fn resolve_ip(ip: Ipv4Address) -> Option<MacAddress> {
    let mut arp_table = ARP_TABLE.lock();
    if arp_table.is_none() { *arp_table = Some(ArpTable::new()); }
    
    // 먼저 테이블에서 확인
    if let Some(mac) = arp_table.as_mut().unwrap().resolve(ip) {
        return Some(mac);
    }
    
    // 테이블에 없으면 ARP 요청 전송
    drop(arp_table);
    
    let local_mac = match crate::net::get_mac_address() {
        Ok(mac) => mac,
        Err(_) => {
            crate::log_warn!("Failed to get local MAC address");
            return None;
        }
    };
    
    let local_ip = Ipv4Address([192, 168, 1, 100]); // TODO: 네트워크 인터페이스에서 가져오기
    
    let mut request_buffer = PacketBuffer::new();
    if create_arp_packet(
        &mut request_buffer,
        ArpOperation::Request,
        local_mac,
        local_ip,
        MacAddress::BROADCAST,
        ip,
    ).is_err() {
        return None;
    }
    
    // 이더넷 브로드캐스트로 ARP 요청 전송
    // TODO: 이더넷 프레임 생성 및 전송 구현
    crate::log_debug!("ARP request sent for {}", ip);
    
    // ARP 응답 대기 (타임아웃 1초)
    // TODO: 비동기적으로 ARP 응답 대기 구현
    
    // 임시로 None 반환 (실제로는 응답 대기 필요)
    None
}

