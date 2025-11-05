//! USB (Universal Serial Bus) 드라이버 모듈
//!
//! 이 모듈은 USB 호스트 컨트롤러 및 USB 디바이스 관리를 담당합니다.
//!
//! # USB 지원 계획
//!
//! ## Phase 1: 기본 인프라 (현재)
//! - USB 호스트 컨트롤러 감지 (xHCI, EHCI, OHCI)
//! - USB 핵심 프로토콜 (USB Request Block, Endpoint)
//! - 기본 디바이스 열거 (Enumeration)
//!
//! ## Phase 2: USB 클래스 지원
//! - USB HID (Human Interface Device) - 키보드, 마우스
//! - USB Mass Storage - 저장장치
//! - USB 네트워크 어댑터 (선택적)
//!
//! ## Phase 3: 고급 기능
//! - USB 3.0 지원 (xHCI)
//! - 동적 디바이스 연결/분리 (Hotplug)
//! - 전력 관리

pub mod host_controller;
pub mod core;
pub mod error;
pub mod device;
pub mod descriptor;
pub mod request;

pub use host_controller::{UsbHostController, UsbHostControllerType};
pub use error::UsbError;
pub use device::UsbDevice;
pub use core::{UsbManager, enumerate_devices};

/// USB 클래스 코드
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbClassCode {
    /// Human Interface Device (키보드, 마우스 등)
    Hid = 0x03,
    /// Mass Storage (USB 저장장치)
    MassStorage = 0x08,
    /// Hub (USB 허브)
    Hub = 0x09,
    /// Video (웹캠 등)
    Video = 0x0E,
    /// Audio (오디오 장치)
    Audio = 0x01,
    /// Unknown/Other
    Unknown = 0xFF,
}

impl From<u8> for UsbClassCode {
    fn from(code: u8) -> Self {
        match code {
            0x03 => UsbClassCode::Hid,
            0x08 => UsbClassCode::MassStorage,
            0x09 => UsbClassCode::Hub,
            0x0E => UsbClassCode::Video,
            0x01 => UsbClassCode::Audio,
            _ => UsbClassCode::Unknown,
        }
    }
}

/// USB 전역 관리자 초기화
///
/// # Safety
/// 메모리 관리 및 PCI 버스가 초기화된 후에 호출되어야 합니다.
pub unsafe fn init() -> Result<(), UsbError> {
    crate::log_info!("Initializing USB subsystem...");
    
    // USB 매니저 초기화
    UsbManager::init()?;
    
    crate::log_info!("USB subsystem initialized");
    Ok(())
}

