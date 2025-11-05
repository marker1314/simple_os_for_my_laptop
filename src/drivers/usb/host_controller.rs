//! USB 호스트 컨트롤러 드라이버
//!
//! 이 모듈은 USB 호스트 컨트롤러를 감지하고 관리합니다.
//! 지원하는 컨트롤러:
//! - xHCI (USB 3.0)
//! - EHCI (USB 2.0)
//! - OHCI/UHCI (USB 1.1)

use crate::drivers::pci::PciDevice;
use crate::drivers::usb::error::UsbError;

/// USB 호스트 컨트롤러 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbHostControllerType {
    /// xHCI (eXtensible Host Controller Interface) - USB 3.0
    Xhci,
    /// EHCI (Enhanced Host Controller Interface) - USB 2.0
    Ehci,
    /// OHCI (Open Host Controller Interface) - USB 1.1
    Ohci,
    /// UHCI (Universal Host Controller Interface) - USB 1.1
    Uhci,
}

/// USB 호스트 컨트롤러 인터페이스
pub trait UsbHostController {
    /// 호스트 컨트롤러 초기화
    fn init(&mut self) -> Result<(), UsbError>;
    
    /// 호스트 컨트롤러 리셋
    fn reset(&mut self) -> Result<(), UsbError>;
    
    /// 호스트 컨트롤러 타입
    fn controller_type(&self) -> UsbHostControllerType;
    
    /// 컨트롤러가 동작 중인지 확인
    fn is_running(&self) -> bool;
}

/// PCI를 통한 USB 호스트 컨트롤러 감지
///
/// # Safety
/// PCI 버스가 초기화된 후에 호출되어야 합니다.
pub unsafe fn find_usb_host_controller() -> Option<(PciDevice, UsbHostControllerType)> {
    use crate::drivers::pci;
    
    // PCI 클래스 코드: Serial Bus Controller (0x0C)
    // 서브클래스: USB (0x03)
    let mut found: Option<(PciDevice, UsbHostControllerType)> = None;
    
    pci::scan_pci_bus(|device| {
        // USB 호스트 컨트롤러 클래스 확인
        if device.class_code == 0x0C && device.subclass == 0x03 {
            // 프로그래밍 인터페이스로 호스트 컨트롤러 타입 결정
            let controller_type = match device.prog_if {
                0x30 => UsbHostControllerType::Xhci, // xHCI
                0x20 => UsbHostControllerType::Ehci,  // EHCI
                0x10 => UsbHostControllerType::Ohci,  // OHCI
                0x00 => UsbHostControllerType::Uhci,  // UHCI
                _ => return false, // 알 수 없는 타입
            };
            
            crate::log_info!(
                "Found USB host controller: {:?} (Vendor=0x{:04X}, Device=0x{:04X})",
                controller_type,
                device.vendor_id,
                device.device_id
            );
            
            found = Some((*device, controller_type));
            true // 스캔 중단
        } else {
            false // 계속 스캔
        }
    });
    
    found
}

/// PCI 디바이스에서 USB 호스트 컨트롤러 타입 확인
pub fn get_usb_controller_type(pci_device: &PciDevice) -> Option<UsbHostControllerType> {
    if pci_device.class_code == 0x0C && pci_device.subclass == 0x03 {
        match pci_device.prog_if {
            0x30 => Some(UsbHostControllerType::Xhci),
            0x20 => Some(UsbHostControllerType::Ehci),
            0x10 => Some(UsbHostControllerType::Ohci),
            0x00 => Some(UsbHostControllerType::Uhci),
            _ => None,
        }
    } else {
        None
    }
}

/// USB 호스트 컨트롤러 기본 구현
/// 
/// 실제 구현은 각 컨트롤러 타입별로 별도 모듈로 분리할 예정
pub struct GenericUsbHostController {
    pci_device: PciDevice,
    controller_type: UsbHostControllerType,
    base_address: u64,
    initialized: bool,
}

impl GenericUsbHostController {
    /// 새 USB 호스트 컨트롤러 생성
    pub fn new(pci_device: PciDevice, controller_type: UsbHostControllerType) -> Self {
        Self {
            pci_device,
            controller_type,
            base_address: 0,
            initialized: false,
        }
    }
    
    /// PCI 디바이스에서 베이스 주소 읽기
    pub unsafe fn read_base_address(&mut self) -> Result<u64, UsbError> {
        // BAR0 읽기
        let bar0 = self.pci_device.bar0;
        
        // MMIO 또는 IO 공간 확인
        if (bar0 & 0x01) == 0 {
            // MMIO 공간
            self.base_address = (bar0 & !0xF) as u64;
            crate::log_info!("USB controller MMIO base: 0x{:016X}", self.base_address);
        } else {
            // IO 공간
            self.base_address = (bar0 & !0x3) as u64;
            crate::log_info!("USB controller IO base: 0x{:04X}", self.base_address as u16);
        }
        
        Ok(self.base_address)
    }
}

impl UsbHostController for GenericUsbHostController {
    fn init(&mut self) -> Result<(), UsbError> {
        unsafe {
            // PCI 버스 마스터 활성화
            let command = self.pci_device.read_config_register(0x04);
            self.pci_device.write_config_register(0x04, command | 0x05); // Bus Master + Memory Space
            
            // 베이스 주소 읽기
            self.read_base_address()?;
            
            // TODO: 실제 호스트 컨트롤러 초기화
            // 각 컨트롤러 타입별로 구현 필요
            crate::log_warn!("USB host controller initialization not yet fully implemented");
            
            self.initialized = true;
            Ok(())
        }
    }
    
    fn reset(&mut self) -> Result<(), UsbError> {
        // TODO: 호스트 컨트롤러 리셋 구현
        crate::log_warn!("USB host controller reset not yet implemented");
        Ok(())
    }
    
    fn controller_type(&self) -> UsbHostControllerType {
        self.controller_type
    }
    
    fn is_running(&self) -> bool {
        self.initialized
    }
}

