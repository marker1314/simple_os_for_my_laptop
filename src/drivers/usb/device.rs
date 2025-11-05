//! USB 디바이스 관리
//!
//! 이 모듈은 USB 디바이스의 상태 및 정보를 관리합니다.

use crate::drivers::usb::descriptor::{DeviceDescriptor, ConfigurationDescriptor, InterfaceDescriptor, EndpointDescriptor};
use crate::drivers::usb::error::UsbError;
use crate::drivers::usb::UsbClassCode;

/// USB 디바이스 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbDeviceState {
    /// 초기 상태 (기본 주소 0)
    Default,
    /// 주소 할당됨
    Address,
    /// 구성 설정됨
    Configured,
    /// 중단됨
    Suspended,
}

/// USB 디바이스
pub struct UsbDevice {
    /// 디바이스 주소
    address: u8,
    /// 디바이스 디스크립터
    device_descriptor: Option<DeviceDescriptor>,
    /// 구성 디스크립터
    configuration_descriptor: Option<ConfigurationDescriptor>,
    /// 인터페이스 디스크립터
    interface_descriptors: [Option<InterfaceDescriptor>; 4],
    /// 엔드포인트 디스크립터
    endpoint_descriptors: [Option<EndpointDescriptor>; 8],
    /// 디바이스 상태
    state: UsbDeviceState,
    /// 디바이스 클래스
    class_code: UsbClassCode,
    /// 최대 패킷 크기 (Endpoint 0)
    max_packet_size: u8,
}

impl UsbDevice {
    /// 새 USB 디바이스 생성
    pub fn new(address: u8) -> Self {
        Self {
            address,
            device_descriptor: None,
            configuration_descriptor: None,
            interface_descriptors: [None, None, None, None],
            endpoint_descriptors: [None, None, None, None, None, None, None, None],
            state: UsbDeviceState::Default,
            class_code: UsbClassCode::Unknown,
            max_packet_size: 8, // 기본값
        }
    }
    
    /// 디바이스 주소 가져오기
    pub fn address(&self) -> u8 {
        self.address
    }
    
    /// 디바이스 상태 가져오기
    pub fn state(&self) -> UsbDeviceState {
        self.state
    }
    
    /// 디바이스 상태 설정
    pub fn set_state(&mut self, state: UsbDeviceState) {
        self.state = state;
    }
    
    /// 디바이스 클래스 가져오기
    pub fn class_code(&self) -> UsbClassCode {
        self.class_code
    }
    
    /// 디바이스 디스크립터 설정
    pub fn set_device_descriptor(&mut self, descriptor: DeviceDescriptor) {
        self.device_descriptor = Some(descriptor);
        self.max_packet_size = descriptor.max_packet_size;
        
        // 디바이스 클래스 코드 설정
        if descriptor.device_class != 0 {
            self.class_code = UsbClassCode::from(descriptor.device_class);
        }
    }
    
    /// 디바이스 디스크립터 가져오기
    pub fn device_descriptor(&self) -> Option<&DeviceDescriptor> {
        self.device_descriptor.as_ref()
    }
    
    /// 최대 패킷 크기 가져오기
    pub fn max_packet_size(&self) -> u8 {
        self.max_packet_size
    }
}

