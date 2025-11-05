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
    /// 감지된 HID 장치 (주소와 프로퍼티)
    hid_devices: Vec<(u8, crate::drivers::usb::hid::HidDevice)>,
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
            hid_devices: Vec::new(),
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
        use crate::drivers::usb::descriptor::{DeviceDescriptor, ConfigurationDescriptor, DescriptorType, InterfaceDescriptor, EndpointDescriptor};
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
        
        // 5. 구성 디스크립터 읽기 (먼저 헤더 9바이트로 total_length 파악)
        let get_cfg_hdr = UsbControlRequest::new_get_descriptor(
            DescriptorType::Configuration,
            0,
            0,
            core::mem::size_of::<ConfigurationDescriptor>() as u16,
        );
        let mut cfg_hdr_buf = [0u8; 9];
        unsafe {
            controller.send_control_request(&get_cfg_hdr, cfg_hdr_buf.as_mut_ptr(), cfg_hdr_buf.len() as u16)?;
        }
        let cfg_hdr = unsafe { core::ptr::read(cfg_hdr_buf.as_ptr() as *const ConfigurationDescriptor) };
        let total_len = cfg_hdr.total_length as usize;
        // 전체 구성 디스크립터 블록 읽기
        let mut cfg_buf_vec: alloc::vec::Vec<u8> = alloc::vec![0u8; total_len];
        let get_cfg_full = UsbControlRequest::new_get_descriptor(DescriptorType::Configuration, 0, 0, total_len as u16);
        unsafe {
            controller.send_control_request(&get_cfg_full, cfg_buf_vec.as_mut_ptr(), total_len as u16)?;
        }
        // 간단 파서: Interface/Endpoint 디스크립터들만 추출
        let mut idx = 0usize;
        let mut interfaces: alloc::vec::Vec<(InterfaceDescriptor, alloc::vec::Vec<EndpointDescriptor>)> = alloc::vec::Vec::new();
        while idx + 2 <= total_len {
            let len = cfg_buf_vec[idx] as usize;
            if len == 0 || idx + len > total_len { break; }
            let dtype = cfg_buf_vec[idx + 1];
            match dtype {
                0x04 => { // Interface
                    if len >= core::mem::size_of::<InterfaceDescriptor>() { 
                        let intf = unsafe { core::ptr::read(cfg_buf_vec[idx..].as_ptr() as *const InterfaceDescriptor) };
                        interfaces.push((intf, alloc::vec::Vec::new()));
                    }
                }
                0x05 => { // Endpoint
                    if len >= core::mem::size_of::<EndpointDescriptor>() {
                        if let Some((_, ref mut eps)) = interfaces.last_mut() {
                            let ep = unsafe { core::ptr::read(cfg_buf_vec[idx..].as_ptr() as *const EndpointDescriptor) };
                            eps.push(ep);
                        }
                    }
                }
                _ => {}
            }
            idx += len;
        }
        
        // 6. 구성 설정 (기본 구성 1 사용)
        let set_config_request = UsbControlRequest::new_set_configuration(1);
        unsafe {
            controller.send_control_request(&set_config_request, core::ptr::null_mut(), 0)?;
        }
        
        // 7. USB 디바이스 객체 생성
        let mut device = UsbDevice::new(new_address);
        device.set_device_descriptor(device_descriptor);
        device.set_state(crate::drivers::usb::device::UsbDeviceState::Configured);

        // HID 인터페이스 감지 (키보드/마우스) 및 저장
        for (intf, eps) in interfaces.iter() {
            if let Some(hid) = crate::drivers::usb::hid::HidDevice::from_interface(intf, eps) {
                crate::log_info!(
                    "Detected USB HID {:?} (INT IN ep=0x{:02X}, interval={}ms)",
                    hid.kind,
                    hid.interrupt_in.map(|e| e.address).unwrap_or(0),
                    hid.interrupt_in.map(|e| e.interval_ms).unwrap_or(0)
                );
                self.hid_devices.push((new_address, hid));
            }
        }
        
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
    hid_devices: Vec::new(),
    next_address: 1,
    initialized: false,
});

/// Poll HID devices for input and feed into GUI mouse when available
pub fn poll_hid() {
    use crate::drivers::usb::hid::HidDeviceKind;
    use crate::drivers::usb::hid::build_get_report_request;
    use crate::drivers::usb::hid::report::MouseReport;
    let mut mgr = MANAGER.lock();
    if !mgr.initialized || mgr.host_controllers.is_empty() { return; }
    // 단일 컨트롤러 가정
    let controller: *mut GenericUsbHostController = &mut mgr.host_controllers[0];
    for (_addr, hid) in mgr.hid_devices.iter() {
        match hid.kind {
            HidDeviceKind::Mouse => unsafe {
                let mut buf = [0u8; core::mem::size_of::<MouseReport>()];
                // 우선 Interrupt IN 시도
                if (*controller).recv_interrupt_in(hid.interrupt_in.map(|e| e.address).unwrap_or(0), buf.as_mut_ptr(), buf.len() as u16).is_err() {
                    let req = build_get_report_request(hid.interface_number, 0x01, 0, buf.len() as u16);
                    let _ = (*controller).send_control_request(&req, buf.as_mut_ptr(), buf.len() as u16);
                }
                let rep = MouseReport { buttons: buf[0], dx: buf[1] as i8, dy: buf[2] as i8, wheel: buf[3] as i8 };
                // PS/2 드라이버 좌표계와 일치하도록 Y축 반전 적용
                let (mut x, mut y) = crate::drivers::mouse::get_position();
                x = x.saturating_add(rep.dx as isize);
                y = y.saturating_sub(rep.dy as isize);
                // 화면 경계 보정
                let (sw, sh) = if let Some(info) = crate::drivers::framebuffer::info() {
                    (info.width as isize, info.height as isize)
                } else { (800, 600) };
                if x < 0 { x = 0; } else if x >= sw { x = sw - 1; }
                if y < 0 { y = 0; } else if y >= sh { y = sh - 1; }

                // 이동 이벤트
                crate::drivers::mouse::inject_event(crate::drivers::mouse::MouseEvent::Move(x, y));

                // 버튼 이벤트 (현재 버튼 상태와 비교)
                let (mut left, mut right, mut middle) = crate::drivers::mouse::get_buttons();
                let new_left = (rep.buttons & 0x01) != 0;
                let new_right = (rep.buttons & 0x02) != 0;
                let new_middle = (rep.buttons & 0x04) != 0;
                if new_left != left {
                    let ev = if new_left { crate::drivers::mouse::MouseEvent::LeftButtonDown(x,y) } else { crate::drivers::mouse::MouseEvent::LeftButtonUp(x,y) };
                    crate::drivers::mouse::inject_event(ev);
                    left = new_left;
                }
                if new_right != right {
                    let ev = if new_right { crate::drivers::mouse::MouseEvent::RightButtonDown(x,y) } else { crate::drivers::mouse::MouseEvent::RightButtonUp(x,y) };
                    crate::drivers::mouse::inject_event(ev);
                    right = new_right;
                }
                if new_middle != middle {
                    let ev = if new_middle { crate::drivers::mouse::MouseEvent::MiddleButtonDown(x,y) } else { crate::drivers::mouse::MouseEvent::MiddleButtonUp(x,y) };
                    crate::drivers::mouse::inject_event(ev);
                    middle = new_middle;
                }
            },
            _ => {}
        }
    }
}

