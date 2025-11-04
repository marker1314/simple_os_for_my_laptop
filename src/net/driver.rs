//! 네트워크 드라이버 관리
//!
//! 이 모듈은 네트워크 드라이버를 관리하고 초기화합니다.

use crate::drivers::pci;
use crate::net::ethernet::{EthernetDriver, NetworkError, MacAddress, PacketBuffer};
use spin::Mutex;

/// 네트워크 드라이버 매니저
///
/// 네트워크 드라이버를 관리하고 패킷을 라우팅합니다.
pub struct NetworkDriverManager {
    /// 현재 사용 중인 이더넷 드라이버
    driver: Option<Box<dyn EthernetDriver + Send>>,
    /// 초기화 여부
    initialized: bool,
}

impl NetworkDriverManager {
    /// 새 네트워크 드라이버 매니저 생성
    pub fn new() -> Self {
        Self {
            driver: None,
            initialized: false,
        }
    }
    
    /// 네트워크 드라이버 초기화
    ///
    /// PCI 버스를 스캔하여 네트워크 디바이스를 찾고 초기화합니다.
    ///
    /// # Safety
    /// 메모리 관리가 초기화된 후에 호출되어야 합니다.
    pub unsafe fn init(&mut self) -> Result<(), NetworkError> {
        // PCI 버스에서 네트워크 디바이스 찾기
        let pci_device = match pci::find_network_device() {
            Some(device) => device,
            None => {
                crate::log_warn!("No network device found on PCI bus");
                return Err(NetworkError::DeviceNotFound);
            }
        };
        
        crate::log_info!("Found network device: Vendor=0x{:04X}, Device=0x{:04X}", 
                         pci_device.vendor_id, pci_device.device_id);
        
        // TODO: 벤더 ID와 디바이스 ID에 따라 적절한 드라이버 선택
        // 현재는 기본 이더넷 드라이버만 지원
        
        // 일단 드라이버 초기화는 나중에 구현
        // 실제 드라이버 구현은 특정 하드웨어(예: RTL8139, e1000)에 따라 달라집니다
        
        self.initialized = true;
        Ok(())
    }
    
    /// 패킷 송신
    pub fn send_packet(&mut self, packet: &PacketBuffer) -> Result<(), NetworkError> {
        if let Some(ref mut driver) = self.driver {
            driver.send_packet(packet)
        } else {
            Err(NetworkError::NotInitialized)
        }
    }
    
    /// 패킷 수신
    pub fn receive_packet(&mut self) -> Option<PacketBuffer> {
        if let Some(ref mut driver) = self.driver {
            driver.receive_packet()
        } else {
            None
        }
    }
    
    /// MAC 주소 가져오기
    pub fn get_mac_address(&self) -> Result<MacAddress, NetworkError> {
        if let Some(ref driver) = self.driver {
            driver.get_mac_address()
        } else {
            Err(NetworkError::NotInitialized)
        }
    }
    
    /// 초기화 여부 확인
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

/// 전역 네트워크 드라이버 매니저
static NETWORK_MANAGER: Mutex<NetworkDriverManager> = Mutex::new(NetworkDriverManager::new());

/// 네트워크 드라이버 초기화
///
/// # Safety
/// 메모리 관리가 초기화된 후에 호출되어야 합니다.
pub unsafe fn init() -> Result<(), NetworkError> {
    let mut manager = NETWORK_MANAGER.lock();
    manager.init()
}

/// 패킷 송신
pub fn send_packet(packet: &PacketBuffer) -> Result<(), NetworkError> {
    let mut manager = NETWORK_MANAGER.lock();
    manager.send_packet(packet)
}

/// 패킷 수신
pub fn receive_packet() -> Option<PacketBuffer> {
    let mut manager = NETWORK_MANAGER.lock();
    manager.receive_packet()
}

/// MAC 주소 가져오기
pub fn get_mac_address() -> Result<MacAddress, NetworkError> {
    let manager = NETWORK_MANAGER.lock();
    manager.get_mac_address()
}

