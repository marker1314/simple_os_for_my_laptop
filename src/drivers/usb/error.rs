//! USB 에러 타입

use crate::log_error;

/// USB 관련 에러
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbError {
    /// 디바이스를 찾을 수 없음
    DeviceNotFound,
    /// 초기화되지 않음
    NotInitialized,
    /// 지원하지 않는 디바이스
    UnsupportedDevice,
    /// 타임아웃
    Timeout,
    /// 잘못된 파라미터
    InvalidParameter,
    /// 메모리 부족
    OutOfMemory,
    /// 호스트 컨트롤러 초기화 실패
    HostControllerInitFailed,
    /// USB 요청 실패
    RequestFailed,
    /// 디바이스 열거 실패
    EnumerationFailed,
}

impl core::fmt::Display for UsbError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UsbError::DeviceNotFound => write!(f, "USB device not found"),
            UsbError::NotInitialized => write!(f, "USB subsystem not initialized"),
            UsbError::UnsupportedDevice => write!(f, "Unsupported USB device"),
            UsbError::Timeout => write!(f, "USB operation timeout"),
            UsbError::InvalidParameter => write!(f, "Invalid USB parameter"),
            UsbError::OutOfMemory => write!(f, "USB out of memory"),
            UsbError::HostControllerInitFailed => write!(f, "USB host controller initialization failed"),
            UsbError::RequestFailed => write!(f, "USB request failed"),
            UsbError::EnumerationFailed => write!(f, "USB device enumeration failed"),
        }
    }
}

