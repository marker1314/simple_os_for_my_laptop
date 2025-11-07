//! IP (Internet Protocol) 모듈
//!
//! 이 모듈은 IPv4 프로토콜을 구현합니다.

use crate::net::ethernet::{PacketBuffer, NetworkError};
use core::fmt;
use spin::Mutex;

/// IPv4 주소 (4바이트)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv4Address(pub [u8; 4]);

impl Ipv4Address {
    /// 로컬호스트 주소 (127.0.0.1)
    pub const LOCALHOST: Ipv4Address = Ipv4Address([127, 0, 0, 1]);
    
    /// 브로드캐스트 주소 (255.255.255.255)
    pub const BROADCAST: Ipv4Address = Ipv4Address([255, 255, 255, 255]);
    
    /// 모든 호스트 멀티캐스트 주소 (224.0.0.1)
    pub const ALL_HOSTS: Ipv4Address = Ipv4Address([224, 0, 0, 1]);
    
    /// 빈 주소 (0.0.0.0)
    pub const UNSPECIFIED: Ipv4Address = Ipv4Address([0, 0, 0, 0]);
    
    /// 주소가 브로드캐스트인지 확인
    pub fn is_broadcast(&self) -> bool {
        *self == Self::BROADCAST
    }
    
    /// 주소가 멀티캐스트인지 확인 (224.0.0.0 ~ 239.255.255.255)
    pub fn is_multicast(&self) -> bool {
        self.0[0] >= 224 && self.0[0] <= 239
    }
    
    /// 주소가 로컬호스트인지 확인
    pub fn is_localhost(&self) -> bool {
        *self == Self::LOCALHOST
    }
}

/// Global (temporary) assigned IPv4 address storage
static ASSIGNED_IPV4: Mutex<Ipv4Address> = Mutex::new(Ipv4Address::UNSPECIFIED);

/// Apply assigned IPv4 address (e.g., from DHCP)
pub fn apply_assigned_ipv4(addr: Ipv4Address) {
    *ASSIGNED_IPV4.lock() = addr;
    crate::log_info!("IP: assigned IPv4 {}", addr);
}

/// Get current assigned IPv4 address or UNSPECIFIED
pub fn current_ipv4() -> Ipv4Address {
    *ASSIGNED_IPV4.lock()
}

impl fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

/// IP 프로토콜 번호
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpProtocol {
    /// ICMP (1)
    Icmp = 1,
    /// TCP (6)
    Tcp = 6,
    /// UDP (17)
    Udp = 17,
    /// 알 수 없는 프로토콜
    Unknown(u8),
}

impl From<u8> for IpProtocol {
    fn from(value: u8) -> Self {
        match value {
            1 => IpProtocol::Icmp,
            6 => IpProtocol::Tcp,
            17 => IpProtocol::Udp,
            n => IpProtocol::Unknown(n),
        }
    }
}

impl From<IpProtocol> for u8 {
    fn from(protocol: IpProtocol) -> Self {
        match protocol {
            IpProtocol::Icmp => 1,
            IpProtocol::Tcp => 6,
            IpProtocol::Udp => 17,
            IpProtocol::Unknown(n) => n,
        }
    }
}

/// IP 헤더 구조
#[repr(C, packed)]
pub struct IpHeader {
    /// 버전 (4비트) + 헤더 길이 (4비트, 32비트 워드 단위)
    version_ihl: u8,
    /// 서비스 타입 (Type of Service)
    tos: u8,
    /// 전체 길이 (바이트 단위)
    total_length: u16,
    /// 식별자
    identification: u16,
    /// 플래그 (3비트) + Fragment Offset (13비트)
    flags_fragment_offset: u16,
    /// TTL (Time To Live)
    ttl: u8,
    /// 프로토콜
    protocol: u8,
    /// 헤더 체크섬
    checksum: u16,
    /// 송신자 IP 주소
    src_addr: [u8; 4],
    /// 수신자 IP 주소
    dst_addr: [u8; 4],
}

impl IpHeader {
    /// IP 헤더 크기 (바이트)
    pub const SIZE: usize = 20;
    
    /// 버전 가져오기
    pub fn version(&self) -> u8 {
        (self.version_ihl >> 4) & 0x0F
    }
    
    /// 헤더 길이 가져오기 (32비트 워드 단위)
    pub fn ihl(&self) -> u8 {
        self.version_ihl & 0x0F
    }
    
    /// 헤더 길이 가져오기 (바이트 단위)
    pub fn header_length(&self) -> usize {
        (self.ihl() * 4) as usize
    }
    
    /// 전체 길이 가져오기
    pub fn total_length(&self) -> u16 {
        u16::from_be_bytes([self.total_length as u8, (self.total_length >> 8) as u8])
    }
    
    /// 전체 길이 설정
    pub fn set_total_length(&mut self, length: u16) {
        self.total_length = u16::to_be(length);
    }
    
    /// 프로토콜 가져오기
    pub fn protocol(&self) -> IpProtocol {
        IpProtocol::from(self.protocol)
    }
    
    /// 프로토콜 설정
    pub fn set_protocol(&mut self, protocol: IpProtocol) {
        self.protocol = u8::from(protocol);
    }
    
    /// TTL 가져오기
    pub fn ttl(&self) -> u8 {
        self.ttl
    }
    
    /// TTL 설정
    pub fn set_ttl(&mut self, ttl: u8) {
        self.ttl = ttl;
    }
    
    /// 송신자 주소 가져오기
    pub fn src_addr(&self) -> Ipv4Address {
        Ipv4Address(self.src_addr)
    }
    
    /// 송신자 주소 설정
    pub fn set_src_addr(&mut self, addr: Ipv4Address) {
        self.src_addr = addr.0;
    }
    
    /// 수신자 주소 가져오기
    pub fn dst_addr(&self) -> Ipv4Address {
        Ipv4Address(self.dst_addr)
    }
    
    /// 수신자 주소 설정
    pub fn set_dst_addr(&mut self, addr: Ipv4Address) {
        self.dst_addr = addr.0;
    }
    
    /// 헤더 체크섬 계산
    pub fn calculate_checksum(&self) -> u16 {
        let mut sum: u32 = 0;
        let header_bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u16,
                Self::SIZE / 2
            )
        };
        
        for &word in header_bytes {
            sum += u16::from_be(word) as u32;
        }
        
        // 체크섬 필드는 0으로 간주
        sum -= self.checksum as u32;
        
        // 캐리 비트 처리
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        !sum as u16
    }
    
    /// 체크섬 검증
    pub fn verify_checksum(&self) -> bool {
        self.calculate_checksum() == 0
    }
    
    /// 체크섬 설정
    pub fn set_checksum(&mut self) {
        self.checksum = 0;
        self.checksum = self.calculate_checksum();
    }
    
    /// 패킷 버퍼에서 IP 헤더 읽기
    pub fn from_packet(packet: &PacketBuffer) -> Option<&IpHeader> {
        if packet.length < Self::SIZE {
            return None;
        }
        
        unsafe {
            Some(&*(packet.as_slice().as_ptr() as *const IpHeader))
        }
    }
    
    /// 패킷 버퍼에서 IP 헤더 읽기 (가변)
    pub fn from_packet_mut(packet: &mut PacketBuffer) -> Option<&mut IpHeader> {
        if packet.length < Self::SIZE {
            return None;
        }
        
        unsafe {
            Some(&mut *(packet.as_mut_slice().as_mut_ptr() as *mut IpHeader))
        }
    }
    
    /// 새 IP 헤더 생성
    pub fn new(
        src: Ipv4Address,
        dst: Ipv4Address,
        protocol: IpProtocol,
        payload_length: usize,
    ) -> Self {
        let total_length = (Self::SIZE + payload_length) as u16;
        
        let mut header = Self {
            version_ihl: (4 << 4) | (Self::SIZE as u8 / 4), // IPv4, 헤더 길이
            tos: 0,
            total_length: u16::to_be(total_length),
            identification: 0,
            flags_fragment_offset: 0,
            ttl: 64,
            protocol: u8::from(protocol),
            checksum: 0,
            src_addr: src.0,
            dst_addr: dst.0,
        };
        
        header.set_checksum();
        header
    }
}

/// IP 패킷 구조
pub struct IpPacket<'a> {
    /// IP 헤더
    pub header: &'a IpHeader,
    /// 페이로드 데이터
    pub payload: &'a [u8],
}

impl<'a> IpPacket<'a> {
    /// 패킷 버퍼에서 IP 패킷 파싱
    pub fn from_buffer(buffer: &'a PacketBuffer) -> Option<Self> {
        let header = IpHeader::from_packet(buffer)?;
        
        if header.version() != 4 {
            return None;
        }
        
        let header_len = header.header_length();
        if buffer.length < header_len {
            return None;
        }
        
        let total_len = header.total_length() as usize;
        if buffer.length < total_len {
            return None;
        }
        
        let payload = &buffer.as_slice()[header_len..total_len];
        
        Some(Self {
            header,
            payload,
        })
    }
}

/// IP 체크섬 계산 헬퍼 함수
pub fn calculate_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    
    // 16비트 워드 단위로 처리
    let chunks = data.chunks_exact(2);
    let remainder = chunks.remainder();
    
    for chunk in chunks {
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

/// IP 패킷 생성
pub fn create_ip_packet(
    buffer: &mut PacketBuffer,
    src: Ipv4Address,
    dst: Ipv4Address,
    protocol: IpProtocol,
    payload: &[u8],
) -> Result<(), NetworkError> {
    if payload.len() + IpHeader::SIZE > buffer.data.len() {
        return Err(NetworkError::BufferFull);
    }
    
    // IP 헤더 생성
    let header = IpHeader::new(src, dst, protocol, payload.len());
    
    // 헤더 복사
    let header_bytes = unsafe {
        core::slice::from_raw_parts(
            &header as *const IpHeader as *const u8,
            IpHeader::SIZE
        )
    };
    buffer.data[..IpHeader::SIZE].copy_from_slice(header_bytes);
    
    // 페이로드 복사
    buffer.data[IpHeader::SIZE..IpHeader::SIZE + payload.len()].copy_from_slice(payload);
    
    buffer.length = IpHeader::SIZE + payload.len();
    
    Ok(())
}

/// IP 패킷 처리
///
/// 수신된 IP 패킷을 처리하고, 상위 프로토콜로 전달합니다.
pub fn handle_ip_packet(packet: &PacketBuffer) -> Result<(), NetworkError> {
    let ip_packet = match IpPacket::from_buffer(packet) {
        Some(p) => p,
        None => {
            crate::log_warn!("Invalid IP packet");
            return Err(NetworkError::InvalidPacket);
        }
    };
    
    // 체크섬 검증
    if !ip_packet.header.verify_checksum() {
        crate::log_warn!("IP packet checksum mismatch");
        return Err(NetworkError::InvalidPacket);
    }
    
    // TTL 확인
    if ip_packet.header.ttl() == 0 {
        crate::log_warn!("IP packet TTL expired");
        return Err(NetworkError::InvalidPacket);
    }
    
    // 로컬 IP 주소 확인 (DHCP/수동 적용값 사용, 기본 UNSPECIFIED)
    let local_ip = current_ipv4();
    
    // 패킷이 우리에게 오는 것인지 확인
    if ip_packet.header.dst_addr() != local_ip 
        && !ip_packet.header.dst_addr().is_broadcast()
        && !ip_packet.header.dst_addr().is_multicast() {
        // 다른 호스트로 전달할 패킷 (라우팅 필요)
        crate::log_debug!("IP packet not for us, dropping");
        return Ok(());
    }
    
    // 프로토콜별 처리
    let payload_buffer = match PacketBuffer::from_slice(ip_packet.payload) {
        Some(buf) => buf,
        None => {
            crate::log_warn!("Invalid IP packet payload");
            return Err(NetworkError::InvalidPacket);
        }
    };
    match ip_packet.header.protocol() {
        IpProtocol::Icmp => {
            crate::log_debug!("Received ICMP packet");
            if let Err(e) = crate::net::icmp::handle_icmp_packet(ip_packet.header.src_addr(), &payload_buffer) {
                crate::log_warn!("ICMP packet handling failed: {:?}", e);
            }
        }
        IpProtocol::Tcp => {
            crate::log_debug!("Received TCP packet");
            if let Err(e) = crate::net::tcp::handle_tcp_packet(ip_packet.header.src_addr(), &payload_buffer) {
                crate::log_warn!("TCP packet handling failed: {:?}", e);
            }
        }
        IpProtocol::Udp => {
            crate::log_debug!("Received UDP packet");
            if let Err(e) = crate::net::udp::handle_udp_packet(ip_packet.header.src_addr(), &payload_buffer) {
                crate::log_warn!("UDP packet handling failed: {:?}", e);
            }
        }
        IpProtocol::Unknown(proto) => {
            crate::log_warn!("Unknown IP protocol: {}", proto);
        }
    }
    
    Ok(())
}

/// IP 패킷 송신
///
/// IP 패킷을 생성하고 이더넷으로 전송합니다.
pub fn send_ip_packet(
    dst_ip: Ipv4Address,
    protocol: IpProtocol,
    payload: &[u8],
) -> Result<(), NetworkError> {
    // 로컬 IP 주소 (DHCP/수동 적용값 사용)
    let src_ip = current_ipv4();
    
    // IP 패킷 생성
    let mut ip_buffer = PacketBuffer::new();
    create_ip_packet(&mut ip_buffer, src_ip, dst_ip, protocol, payload)?;
    
    // 이더넷 프레임으로 캡슐화하여 전송
    crate::net::ethernet_frame::send_ip_over_ethernet(dst_ip, &ip_buffer)
}

