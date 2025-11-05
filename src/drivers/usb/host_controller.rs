//! USB 호스트 컨트롤러 드라이버
//!
//! 이 모듈은 USB 호스트 컨트롤러를 감지하고 관리합니다.
//! 지원하는 컨트롤러:
//! - xHCI (USB 3.0)
//! - EHCI (USB 2.0)
//! - OHCI/UHCI (USB 1.1)

use crate::drivers::pci::PciDevice;
use crate::drivers::usb::error::UsbError;
use crate::drivers::usb::xhci::XhciController;
use crate::drivers::usb::ehci::EhciController;

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

/// USB 호스트 컨트롤러 래퍼
/// 
/// 실제 컨트롤러 타입에 따라 xHCI 또는 EHCI를 사용합니다.
pub enum GenericUsbHostController {
    Xhci(XhciController),
    Ehci(EhciController),
    Generic {
        pci_device: PciDevice,
        controller_type: UsbHostControllerType,
        base_address: u64,
        initialized: bool,
    },
}

impl GenericUsbHostController {
    /// 새 USB 호스트 컨트롤러 생성
    pub fn new(pci_device: PciDevice, controller_type: UsbHostControllerType) -> Self {
        match controller_type {
            UsbHostControllerType::Xhci => {
                GenericUsbHostController::Xhci(XhciController::new(pci_device))
            }
            UsbHostControllerType::Ehci => {
                GenericUsbHostController::Ehci(EhciController::new(pci_device))
            }
            _ => {
                // OHCI/UHCI는 아직 구현되지 않음
                GenericUsbHostController::Generic {
                    pci_device,
                    controller_type,
                    base_address: 0,
                    initialized: false,
                }
            }
        }
    }
    
    /// 포트 연결 상태 확인
    pub unsafe fn check_port_connection(&self, port: u8) -> Result<bool, UsbError> {
        match self {
            GenericUsbHostController::Xhci(ctrl) => ctrl.check_port_connection(port),
            GenericUsbHostController::Ehci(ctrl) => ctrl.check_port_connection(port),
            GenericUsbHostController::Generic { initialized, .. } => {
                if !*initialized {
                    return Err(UsbError::NotInitialized);
                }
                Err(UsbError::NotImplemented)
            }
        }
    }
    
    /// 포트 리셋
    pub unsafe fn reset_port(&self, port: u8) -> Result<(), UsbError> {
        match self {
            GenericUsbHostController::Xhci(ctrl) => ctrl.reset_port(port),
            GenericUsbHostController::Ehci(ctrl) => ctrl.reset_port(port),
            GenericUsbHostController::Generic { initialized, .. } => {
                if !*initialized {
                    return Err(UsbError::NotInitialized);
                }
                Err(UsbError::NotImplemented)
            }
        }
    }
    
    /// 포트 수 가져오기
    pub fn port_count(&self) -> u8 {
        match self {
            GenericUsbHostController::Xhci(ctrl) => ctrl.port_count(),
            GenericUsbHostController::Ehci(ctrl) => ctrl.port_count(),
            GenericUsbHostController::Generic { .. } => 0,
        }
    }
    
    /// USB 제어 요청 전송
    ///
    /// # Safety
    /// 컨트롤러가 초기화되어 있어야 합니다.
    pub unsafe fn send_control_request(
        &mut self,
        request: &crate::drivers::usb::request::UsbControlRequest,
        data_buffer: *mut u8,
        data_length: u16,
    ) -> Result<(), UsbError> {
        match self {
            GenericUsbHostController::Xhci(ctrl) => {
                ctrl.send_control_request(request, data_buffer, data_length)
            }
            GenericUsbHostController::Ehci(ctrl) => {
                // EHCI는 아직 구현되지 않음
                Err(UsbError::NotImplemented)
            }
            GenericUsbHostController::Generic { initialized, .. } => {
                if !*initialized {
                    return Err(UsbError::NotInitialized);
                }
                Err(UsbError::NotImplemented)
            }
        }
    }
}

impl UsbHostController for GenericUsbHostController {
    fn init(&mut self) -> Result<(), UsbError> {
        match self {
            GenericUsbHostController::Xhci(ctrl) => ctrl.init(),
            GenericUsbHostController::Ehci(ctrl) => ctrl.init(),
            GenericUsbHostController::Generic { pci_device, controller_type, initialized, base_address } => {
                unsafe {
                    // PCI 버스 마스터 활성화
                    let command = pci_device.read_config_register(0x04);
                    pci_device.write_config_register(0x04, command | 0x05);
                    
                    // 베이스 주소 읽기
                    let bar0 = pci_device.bar0;
                    if (bar0 & 0x01) == 0 {
                        *base_address = (bar0 & !0xF) as u64;
                    } else {
                        *base_address = (bar0 & !0x3) as u64;
                    }
                    
                    crate::log_warn!("USB host controller {:?} initialization not yet fully implemented", controller_type);
                    *initialized = true;
                    Ok(())
                }
            }
        }
    }
    
    fn reset(&mut self) -> Result<(), UsbError> {
        match self {
            GenericUsbHostController::Xhci(ctrl) => ctrl.reset(),
            GenericUsbHostController::Ehci(ctrl) => ctrl.reset(),
            GenericUsbHostController::Generic { .. } => {
                crate::log_warn!("USB host controller reset not yet implemented");
                Ok(())
            }
        }
    }
    
    fn controller_type(&self) -> UsbHostControllerType {
        match self {
            GenericUsbHostController::Xhci(ctrl) => ctrl.controller_type(),
            GenericUsbHostController::Ehci(ctrl) => ctrl.controller_type(),
            GenericUsbHostController::Generic { controller_type, .. } => *controller_type,
        }
    }
    
    fn is_running(&self) -> bool {
        match self {
            GenericUsbHostController::Xhci(ctrl) => ctrl.is_running(),
            GenericUsbHostController::Ehci(ctrl) => ctrl.is_running(),
            GenericUsbHostController::Generic { initialized, .. } => *initialized,
        }
    }
}

