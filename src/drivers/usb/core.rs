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
            
            // 드라이버 재시도 메커니즘 적용
            use crate::kernel::error_recovery::{driver_retry, RetryConfig};
            
            let retry_config = RetryConfig {
                max_retries: 3,
                retry_delay_ms: 50,
                exponential_backoff: true,
            };
            
            match driver_retry(|| controller.init(), retry_config) {
                Ok(()) => {
                    manager.host_controllers.push(controller);
                    crate::log_info!("USB host controller initialized successfully");
                }
                Err(e) => {
                    crate::log_warn!("Failed to initialize USB host controller after retries: {}", e);
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
        
        crate::log_info!("Starting USB device enumeration...");
        
        // 각 호스트 컨트롤러에 대해 디바이스 열거
        for controller in &mut self.host_controllers {
            // 실제 포트 수 가져오기
            let port_count = controller.port_count();
            if port_count == 0 {
                crate::log_warn!("Controller has no ports, skipping");
                continue;
            }
            
            for port in 1..=port_count {
                // 포트 연결 상태 확인
                if let Ok(connected) = self.check_port_connection(controller, port) {
                    if connected {
                        crate::log_info!("USB device detected on port {}", port);
                        
                        // 디바이스 초기화 및 열거
                        if let Ok(device) = self.enumerate_device_on_port(&mut *controller, port) {
                            self.devices.push(device);
                            crate::log_info!("USB device enumerated successfully (address: {})", 
                                           self.devices.len());
                        } else {
                            crate::log_warn!("Failed to enumerate device on port {}", port);
                        }
                    }
                }
            }
        }
        
        crate::log_info!("USB enumeration complete: {} device(s) found", self.devices.len());
        Ok(())
    }
    
    /// 포트 연결 상태 확인
    /// 
    /// # Safety
    /// 호스트 컨트롤러가 초기화되어 있어야 합니다.
    unsafe fn check_port_connection(&self, controller: &GenericUsbHostController, port: u8) -> Result<bool, UsbError> {
        controller.check_port_connection(port)
    }
    
    /// 특정 포트의 디바이스 열거
    /// 
    /// # Safety
    /// 호스트 컨트롤러가 초기화되어 있어야 합니다.
    unsafe fn enumerate_device_on_port(&mut self, controller: &mut GenericUsbHostController, port: u8) -> Result<UsbDevice, UsbError> {
        use crate::drivers::usb::descriptor::DeviceDescriptor;
        use crate::drivers::usb::request::UsbControlRequest;
        
        // 1. 포트 리셋 (디바이스 초기화)
        controller.reset_port(port)?;
        
        // 2. 기본 주소(0)로 디바이스 디스크립터 읽기
        let device_address = 0u8; // 열거 전에는 기본 주소 사용
        
        // Get Descriptor 요청 생성 (Device Descriptor)
        let request = UsbControlRequest::new_get_descriptor(
            crate::drivers::usb::descriptor::DescriptorType::Device,
            0, // 인덱스
            0, // 언어 ID
            18, // Device Descriptor 길이
        );
        
        // 디바이스 디스크립터 읽기
        let mut descriptor_buf = [0u8; 18];
        unsafe {
            controller.send_control_request(&request, descriptor_buf.as_mut_ptr(), 18)?;
        }
        
        // 디스크립터 파싱
        let device_descriptor = unsafe {
            core::ptr::read(descriptor_buf.as_ptr() as *const DeviceDescriptor)
        };
        
        // 디스크립터 검증
        if device_descriptor.length != 18 || device_descriptor.descriptor_type != 0x01 {
            return Err(UsbError::InvalidDescriptor);
        }
        
        // 최대 패킷 크기 확인
        let max_packet_size = device_descriptor.max_packet_size;
        crate::log_info!("USB device: VID=0x{:04X}, PID=0x{:04X}, MaxPacketSize={}", 
                        device_descriptor.vendor_id,
                        device_descriptor.product_id,
                        max_packet_size);
        
        // 3. 주소 할당
        let new_address = self.next_address;
        if new_address > 127 {
            return Err(UsbError::DeviceLimitReached);
        }
        self.next_address += 1;
        
        // Set Address 요청
        let set_addr_request = UsbControlRequest::new_set_address(new_address);
        unsafe {
            controller.send_control_request(&set_addr_request, core::ptr::null_mut(), 0)?;
        }
        
        // 주소 설정 후 지연 (디바이스가 주소를 적용하는 시간)
        let start_ms = crate::drivers::timer::get_milliseconds();
        while crate::drivers::timer::get_milliseconds() - start_ms < 10 {
            core::hint::spin_loop();
        }
        
        // 4. 새 주소로 디바이스 디스크립터 다시 읽기 (검증)
        let verify_request = UsbControlRequest::new_get_descriptor(
            crate::drivers::usb::descriptor::DescriptorType::Device,
            0,
            0,
            18,
        );
        unsafe {
            controller.send_control_request(&verify_request, descriptor_buf.as_mut_ptr(), 18)?;
        }
        
        // 5. 구성 디스크립터 읽기
        // TODO: Get Configuration Descriptor (구현 필요)
        
        // 6. 구성 설정 (기본 구성 1 사용)
        let set_config_request = UsbControlRequest::new_set_configuration(1);
        unsafe {
            controller.send_control_request(&set_config_request, core::ptr::null_mut(), 0)?;
        }
        
        // 7. USB 디바이스 객체 생성
        let mut device = UsbDevice::new(new_address);
        device.set_device_descriptor(device_descriptor);
        device.set_state(crate::drivers::usb::device::UsbDeviceState::Configured);
        
        crate::log_info!("USB device enumerated: address={}, class={:?}", 
                        new_address,
                        device.class_code());
        
        Ok(device)
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

