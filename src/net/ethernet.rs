//! 이더넷 드라이버 인터페이스
//!
//! 이 모듈은 이더넷 네트워크 카드의 기본 드라이버 인터페이스를 정의합니다.

use crate::drivers::pci::PciDevice;

/// MAC 주소 (6바이트)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    /// 브로드캐스트 MAC 주소
    pub const BROADCAST: MacAddress = MacAddress([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
}

impl core::fmt::Display for MacAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for (i, byte) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ":")?;
            }
            write!(f, "{:02X}", byte)?;
        }
        Ok(())
    }
}

/// 네트워크 패킷 버퍼
///
/// 이더넷 프레임을 저장하는 버퍼입니다.
#[derive(Debug, Clone)]
pub struct PacketBuffer {
    /// 패킷 데이터
    pub data: [u8; 1518], // 최대 이더넷 프레임 크기
    /// 실제 데이터 길이
    pub length: usize,
}

impl PacketBuffer {
    /// 빈 패킷 버퍼 생성
    pub fn new() -> Self {
        Self {
            data: [0; 1518],
            length: 0,
        }
    }
    
    /// 데이터로부터 패킷 버퍼 생성
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() > 1518 {
            return None;
        }
        
        let mut buffer = Self::new();
        buffer.data[..data.len()].copy_from_slice(data);
        buffer.length = data.len();
        Some(buffer)
    }
    
    /// 패킷 데이터 슬라이스 가져오기
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.length]
    }
    
    /// 패킷 데이터 슬라이스 가져오기 (가변)
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data[..self.length]
    }
}

/// 네트워크 드라이버 오류
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkError {
    /// 디바이스를 찾을 수 없음
    DeviceNotFound,
    /// 초기화 실패
    InitializationFailed,
    /// 버퍼 부족
    BufferFull,
    /// 잘못된 패킷
    InvalidPacket,
    /// 드라이버가 초기화되지 않음
    NotInitialized,
    /// 하드웨어 오류
    HardwareError,
}

/// 이더넷 드라이버 트레이트
///
/// 모든 이더넷 드라이버는 이 트레이트를 구현해야 합니다.
pub trait EthernetDriver {
    /// 드라이버 이름
    fn name(&self) -> &str;
    
    /// 드라이버 초기화
    ///
    /// # Safety
    /// PCI 디바이스가 유효하고, 메모리 관리가 초기화된 후에 호출되어야 합니다.
    unsafe fn init(&mut self, pci_device: &PciDevice) -> Result<(), NetworkError>;
    
    /// MAC 주소 가져오기
    fn get_mac_address(&self) -> Result<MacAddress, NetworkError>;
    
    /// 패킷 송신
    ///
    /// 패킷을 네트워크로 전송합니다.
    fn send_packet(&mut self, packet: &PacketBuffer) -> Result<(), NetworkError>;
    
    /// 패킷 수신
    ///
    /// 수신된 패킷이 있으면 반환합니다. 없으면 None을 반환합니다.
    fn receive_packet(&mut self) -> Option<PacketBuffer>;
    
    /// 인터럽트 핸들러
    ///
    /// 네트워크 인터럽트가 발생했을 때 호출됩니다.
    fn handle_interrupt(&mut self);
    
    /// 드라이버가 초기화되었는지 확인
    fn is_initialized(&self) -> bool;
}

