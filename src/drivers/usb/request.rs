//! USB Request Block (URB) 및 USB 요청 처리
//!
//! 이 모듈은 USB 표준 요청 및 USB Request Block을 처리합니다.

use crate::drivers::usb::descriptor::DescriptorType;

/// USB 요청 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbRequestType {
    /// 표준 요청
    Standard = 0x00,
    /// 클래스 요청
    Class = 0x20,
    /// 벤더 요청
    Vendor = 0x40,
    /// 예약됨
    Reserved = 0x60,
}

/// USB 요청 수신자
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbRequestRecipient {
    /// 디바이스
    Device = 0x00,
    /// 인터페이스
    Interface = 0x01,
    /// 엔드포인트
    Endpoint = 0x02,
    /// 기타
    Other = 0x03,
}

/// USB 요청 방향
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbRequestDirection {
    /// 호스트 → 디바이스
    HostToDevice = 0x00,
    /// 디바이스 → 호스트
    DeviceToHost = 0x80,
}

/// USB 표준 요청 코드
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbStandardRequest {
    GetStatus = 0x00,
    ClearFeature = 0x01,
    SetFeature = 0x03,
    SetAddress = 0x05,
    GetDescriptor = 0x06,
    SetDescriptor = 0x07,
    GetConfiguration = 0x08,
    SetConfiguration = 0x09,
    GetInterface = 0x0A,
    SetInterface = 0x0B,
    SynchFrame = 0x0C,
}

/// USB 제어 요청 구조
#[repr(C, packed)]
pub struct UsbControlRequest {
    /// 요청 타입 (bmRequestType)
    pub request_type: u8,
    /// 요청 코드 (bRequest)
    pub request: u8,
    /// 값 (wValue)
    pub value: u16,
    /// 인덱스 (wIndex)
    pub index: u16,
    /// 길이 (wLength)
    pub length: u16,
}

impl UsbControlRequest {
    /// 표준 Get Descriptor 요청 생성
    pub fn new_get_descriptor(
        descriptor_type: DescriptorType,
        descriptor_index: u8,
        language_id: u16,
        length: u16,
    ) -> Self {
        Self {
            request_type: 0x80, // Device to Host, Standard, Device
            request: UsbStandardRequest::GetDescriptor as u8,
            value: ((descriptor_type as u16) << 8) | (descriptor_index as u16),
            index: language_id,
            length,
        }
    }
    
    /// Set Address 요청 생성
    pub fn new_set_address(address: u8) -> Self {
        Self {
            request_type: 0x00, // Host to Device, Standard, Device
            request: UsbStandardRequest::SetAddress as u8,
            value: address as u16,
            index: 0,
            length: 0,
        }
    }
    
    /// Set Configuration 요청 생성
    pub fn new_set_configuration(configuration_value: u8) -> Self {
        Self {
            request_type: 0x00, // Host to Device, Standard, Device
            request: UsbStandardRequest::SetConfiguration as u8,
            value: configuration_value as u16,
            index: 0,
            length: 0,
        }
    }
}

