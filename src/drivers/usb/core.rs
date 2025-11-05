//! USB 코어 시스템
//!
//! 이 모듈은 USB 시스템의 중앙 관리자입니다.

use crate::drivers::usb::error::UsbError;
use crate::drivers::usb::host_controller::{UsbHostController, UsbHostControllerType, GenericUsbHostController, find_usb_host_controller};
use crate::drivers::usb::device::UsbDevice;
use spin::Mutex;
use alloc::vec::Vec;

/// USB 매니저
pub struct UsbManager {
    /// 호스트 컨트롤러 목록
    host_controllers: Vec<GenericUsbHostController>,
    /// 연결된 USB 디바이스 목록
    devices: Vec<UsbDevice>,
    /// 다음 디바이스 주소 (1-127)
    next_address: u8,
    /// 초기화 여부
    initialized: bool,
}

impl UsbManager {
    /// 새 USB 매니저 생성
    fn new() -> Self {
        Self {
            host_controllers: Vec::new(),
            devices: Vec::new(),
            next_address: 1,
            initialized: false,
        }
    }
    
    /// USB 매니저 초기화
    ///
    /// # Safety
    /// PCI 버스 및 메모리 관리가 초기화된 후에 호출되어야 합니다.
    pub unsafe fn init() -> Result<(), UsbError> {
        let mut manager = MANAGER.lock();
        
        if manager.initialized {
            return Ok(()); // 이미 초기화됨
        }
        
        crate::log_info!("Scanning for USB host controllers...");
        
        // USB 호스트 컨트롤러 찾기
        if let Some((pci_device, controller_type)) = find_usb_host_controller() {
            crate::log_info!("Found USB host controller: {:?}", controller_type);
            
            let mut controller = GenericUsbHostController::new(pci_device, controller_type);
            
            match controller.init() {
                Ok(()) => {
                    manager.host_controllers.push(controller);
                    crate::log_info!("USB host controller initialized successfully");
                }
                Err(e) => {
                    crate::log_warn!("Failed to initialize USB host controller: {}", e);
                    return Err(e);
                }
            }
        } else {
            crate::log_warn!("No USB host controller found");
            return Err(UsbError::DeviceNotFound);
        }
        
        manager.initialized = true;
        crate::log_info!("USB manager initialized with {} controller(s)", manager.host_controllers.len());
        
        Ok(())
    }
    
    /// USB 디바이스 열거 (Enumeration) - 공개 인터페이스
    pub unsafe fn enumerate_devices() -> Result<(), UsbError> {
        let mut manager = MANAGER.lock();
        manager.enumerate_devices()
    }
    
    /// USB 디바이스 열거 (Enumeration) - 내부 구현
    ///
    /// 연결된 USB 디바이스를 발견하고 초기화합니다.
    ///
    /// # Safety
    /// USB 매니저가 초기화된 후에 호출되어야 합니다.
    pub unsafe fn enumerate_devices(&mut self) -> Result<(), UsbError> {
        if !self.initialized {
            return Err(UsbError::NotInitialized);
        }
        
        // TODO: 실제 디바이스 열거 구현
        // 1. 호스트 컨트롤러를 통해 루트 허브 확인
        // 2. 연결된 디바이스 감지
        // 3. 디바이스 디스크립터 읽기
        // 4. 주소 할당
        // 5. 구성 설정
        
        crate::log_info!("USB device enumeration not yet fully implemented");
        Ok(())
    }
    
    /// 연결된 디바이스 수 가져오기
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
    
    /// 초기화 여부 확인
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

/// 전역 USB 매니저
static MANAGER: Mutex<UsbManager> = Mutex::new(UsbManager {
    host_controllers: Vec::new(),
    devices: Vec::new(),
    next_address: 1,
    initialized: false,
});

