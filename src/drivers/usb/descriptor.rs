//! USB 디스크립터 구조
//!
//! USB 디바이스는 다양한 디스크립터를 통해 자신의 정보를 제공합니다.
//! 이 모듈은 USB 디스크립터를 파싱하고 관리합니다.

/// USB 디스크립터 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorType {
    Device = 0x01,
    Configuration = 0x02,
    String = 0x03,
    Interface = 0x04,
    Endpoint = 0x05,
    DeviceQualifier = 0x06,
    OtherSpeedConfiguration = 0x07,
    InterfacePower = 0x08,
}

impl From<u8> for DescriptorType {
    fn from(value: u8) -> Self {
        match value {
            0x01 => DescriptorType::Device,
            0x02 => DescriptorType::Configuration,
            0x03 => DescriptorType::String,
            0x04 => DescriptorType::Interface,
            0x05 => DescriptorType::Endpoint,
            0x06 => DescriptorType::DeviceQualifier,
            0x07 => DescriptorType::OtherSpeedConfiguration,
            0x08 => DescriptorType::InterfacePower,
            _ => DescriptorType::Device, // 기본값
        }
    }
}

/// USB 디바이스 디스크립터
#[repr(C, packed)]
pub struct DeviceDescriptor {
    /// 디스크립터 길이 (18바이트)
    pub length: u8,
    /// 디스크립터 타입 (Device = 0x01)
    pub descriptor_type: u8,
    /// USB 버전 (BCD 형식, 예: 0x0200 = USB 2.0)
    pub usb_version: u16,
    /// 디바이스 클래스
    pub device_class: u8,
    /// 디바이스 서브클래스
    pub device_subclass: u8,
    /// 디바이스 프로토콜
    pub device_protocol: u8,
    /// 최대 패킷 크기 (Endpoint 0)
    pub max_packet_size: u8,
    /// 벤더 ID
    pub vendor_id: u16,
    /// 프로덕트 ID
    pub product_id: u16,
    /// 디바이스 버전 (BCD)
    pub device_version: u16,
    /// 제조사 문자열 인덱스
    pub manufacturer_string: u8,
    /// 제품 문자열 인덱스
    pub product_string: u8,
    /// 시리얼 번호 문자열 인덱스
    pub serial_string: u8,
    /// 구성 디스크립터 수
    pub num_configurations: u8,
}

/// USB 구성 디스크립터
#[repr(C, packed)]
pub struct ConfigurationDescriptor {
    /// 디스크립터 길이 (9바이트)
    pub length: u8,
    /// 디스크립터 타입 (Configuration = 0x02)
    pub descriptor_type: u8,
    /// 전체 구성 길이
    pub total_length: u16,
    /// 인터페이스 수
    pub num_interfaces: u8,
    /// 구성 값
    pub configuration_value: u8,
    /// 구성 문자열 인덱스
    pub configuration_string: u8,
    /// 속성 (Self-powered, Remote wakeup 등)
    pub attributes: u8,
    /// 최대 전력 (mA 단위, 2mA 단위)
    pub max_power: u8,
}

/// USB 인터페이스 디스크립터
#[repr(C, packed)]
pub struct InterfaceDescriptor {
    /// 디스크립터 길이 (9바이트)
    pub length: u8,
    /// 디스크립터 타입 (Interface = 0x04)
    pub descriptor_type: u8,
    /// 인터페이스 번호
    pub interface_number: u8,
    /// 대체 설정
    pub alternate_setting: u8,
    /// 엔드포인트 수
    pub num_endpoints: u8,
    /// 인터페이스 클래스
    pub interface_class: u8,
    /// 인터페이스 서브클래스
    pub interface_subclass: u8,
    /// 인터페이스 프로토콜
    pub interface_protocol: u8,
    /// 인터페이스 문자열 인덱스
    pub interface_string: u8,
}

/// USB 엔드포인트 디스크립터
#[repr(C, packed)]
pub struct EndpointDescriptor {
    /// 디스크립터 길이 (7바이트)
    pub length: u8,
    /// 디스크립터 타입 (Endpoint = 0x05)
    pub descriptor_type: u8,
    /// 엔드포인트 주소 (비트 7: 방향, 비트 3-0: 엔드포인트 번호)
    pub endpoint_address: u8,
    /// 속성 (비트 1-0: 전송 타입, 비트 3-2: 동기화 타입 등)
    pub attributes: u8,
    /// 최대 패킷 크기
    pub max_packet_size: u16,
    /// 폴링 간격 (ms 단위, 2^interval)
    pub interval: u8,
}

impl EndpointDescriptor {
    /// 엔드포인트 번호 추출
    pub fn endpoint_number(&self) -> u8 {
        self.endpoint_address & 0x0F
    }
    
    /// 엔드포인트 방향 (true = IN, false = OUT)
    pub fn is_in(&self) -> bool {
        (self.endpoint_address & 0x80) != 0
    }
    
    /// 전송 타입 추출
    pub fn transfer_type(&self) -> EndpointTransferType {
        match self.attributes & 0x03 {
            0 => EndpointTransferType::Control,
            1 => EndpointTransferType::Isochronous,
            2 => EndpointTransferType::Bulk,
            3 => EndpointTransferType::Interrupt,
            _ => EndpointTransferType::Control,
        }
    }
}

/// 엔드포인트 전송 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointTransferType {
    Control = 0,
    Isochronous = 1,
    Bulk = 2,
    Interrupt = 3,
}

