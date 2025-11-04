//! 네트워크 드라이버 관리
//!
//! 이 모듈은 네트워크 드라이버를 관리하고 초기화합니다.

use crate::drivers::pci;
use crate::drivers::rtl8139::{Rtl8139Driver, is_rtl8139};
use crate::net::ethernet::{EthernetDriver, NetworkError, MacAddress, PacketBuffer};
use spin::Mutex;
use alloc::boxed::Box;

/// 네트워크 드라이버 매니저
///
/// 네트워크 드라이버를 관리하고 패킷을 라우팅합니다.
pub struct NetworkDriverManager {
    /// 현재 사용 중인 이더넷 드라이버
    driver: Option<Box<dyn EthernetDriver + Send>>,
    /// 초기화 여부
    initialized: bool,
    /// 네트워크 인터럽트 IRQ 번호
    irq: Option<u8>,
}

impl NetworkDriverManager {
    /// 새 네트워크 드라이버 매니저 생성
    pub fn new() -> Self {
        Self {
            driver: None,
            initialized: false,
            irq: None,
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
        
        // 벤더 ID와 디바이스 ID에 따라 적절한 드라이버 선택
        if is_rtl8139(&pci_device) {
            crate::log_info!("Initializing RTL8139 driver (IRQ: {})", pci_device.interrupt_line);
            let mut driver = Rtl8139Driver::new(pci_device);
            driver.init(&pci_device)?;
            self.driver = Some(Box::new(driver));
            self.initialized = true;
            
            // IRQ 번호 저장
            self.irq = Some(pci_device.interrupt_line);
            
            // 네트워크 인터럽트 핸들러 등록 및 활성화
            let interrupt_num = crate::interrupts::pic::PIC1_OFFSET + pci_device.interrupt_line;
            unsafe {
                crate::interrupts::idt::IDT[interrupt_num as usize]
                    .set_handler_fn(network_interrupt_handler);
                crate::interrupts::pic::set_mask(pci_device.interrupt_line, true);
            }
            crate::log_info!("Network interrupt handler registered (IRQ {}, interrupt {})",
                           pci_device.interrupt_line, interrupt_num);
            
            Ok(())
        } else {
            crate::log_warn!("Unsupported network device: Vendor=0x{:04X}, Device=0x{:04X}",
                           pci_device.vendor_id, pci_device.device_id);
            Err(NetworkError::DeviceNotFound)
        }
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

/// 네트워크 인터럽트 핸들러
///
/// 네트워크 인터럽트가 발생했을 때 호출됩니다.
pub extern "x86-interrupt" fn network_interrupt_handler(
    _stack_frame: x86_64::structures::idt::InterruptStackFrame
) {
    let mut manager = NETWORK_MANAGER.lock();
    
    if let Some(ref mut driver) = manager.driver {
        driver.handle_interrupt();
        
        // 수신된 패킷 처리
        while let Some(packet) = driver.receive_packet() {
            // 이더넷 프레임 처리로 전달
            if let Err(e) = crate::net::ethernet_frame::handle_ethernet_frame(&packet) {
                crate::log_warn!("Failed to handle Ethernet frame: {:?}", e);
            }
        }
    }
    
    // PIC에 인터럽트 종료 신호 전송
    if let Some(irq) = manager.irq {
        unsafe {
            crate::interrupts::pic::end_of_interrupt(irq);
        }
    }
}

