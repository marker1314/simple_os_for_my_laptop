//! 오디오 드라이버 모듈
//!
//! 이 모듈은 오디오 하드웨어를 관리합니다.
//!
//! # 지원 계획
//!
//! 1. **HDA (High Definition Audio)**: Intel HD Audio 스펙 구현
//! 2. **기본 오디오 출력**: PCM 샘플링 지원
//! 3. **오디오 입력**: 마이크 입력 (선택적)
//!
//! # 구현 상태
//!
//! - [x] 기본 구조 및 모듈
//! - [ ] HDA 컨트롤러 감지
//! - [ ] 코덱 초기화
//! - [ ] PCM 출력
//! - [ ] 오디오 입력

pub mod hda;
pub mod hda_codec;
pub mod pcm;

pub use hda::HdaController;
pub use pcm::{PcmStream, PcmFormat, PcmError};

use crate::drivers::pci::PciDevice;

/// 오디오 드라이버 초기화
///
/// # Safety
/// PCI 버스가 초기화된 후에 호출되어야 합니다.
pub unsafe fn init() -> Result<(), AudioError> {
    crate::log_info!("Initializing audio subsystem...");
    
    // PCI를 통해 HDA 컨트롤러 찾기
    if let Some(pci_device) = find_hda_controller() {
        crate::log_info!("Found HDA controller: Vendor=0x{:04X}, Device=0x{:04X}",
                        pci_device.vendor_id, pci_device.device_id);
        
        // HDA 컨트롤러 초기화
        match HdaController::new(pci_device) {
            Ok(mut controller) => {
                // 드라이버 재시도 메커니즘 적용
                use crate::kernel::error_recovery::{driver_retry, RetryConfig};
                
                let retry_config = RetryConfig {
                    max_retries: 3,
                    retry_delay_ms: 50,
                    exponential_backoff: true,
                };
                
                match driver_retry(|| unsafe { controller.init() }, retry_config) {
                    Ok(()) => {
                        crate::log_info!("HDA controller initialized successfully");
                        // 전역 컨트롤러 저장 (향후 구현)
                        Ok(())
                    }
                    Err(e) => {
                        crate::log_warn!("Failed to initialize HDA controller after retries: {:?}", e);
                        Err(AudioError::InitFailed)
                    }
                }
            }
            Err(e) => {
                crate::log_warn!("Failed to create HDA controller: {:?}", e);
                Err(AudioError::InitFailed)
            }
        }
    } else {
        crate::log_warn!("No HDA controller found");
        Err(AudioError::DeviceNotFound)
    }
}

/// PCI를 통한 HDA 컨트롤러 감지
///
/// # Safety
/// PCI 버스가 초기화된 후에 호출되어야 합니다.
unsafe fn find_hda_controller() -> Option<PciDevice> {
    use crate::drivers::pci;
    
    // PCI 클래스 코드: Multimedia (0x04)
    // 서브클래스: Audio (0x03)
    // 프로그래밍 인터페이스: HDA (0x01)
    let mut found: Option<PciDevice> = None;
    
    pci::scan_pci_bus(|device| {
        if device.class_code == 0x04 && device.subclass == 0x03 && device.prog_if == 0x01 {
            crate::log_info!("Found HDA controller: Vendor=0x{:04X}, Device=0x{:04X}",
                            device.vendor_id, device.device_id);
            found = Some(*device);
            true // 스캔 중단
        } else {
            false // 계속 스캔
        }
    });
    
    found
}

/// 오디오 에러
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioError {
    DeviceNotFound,
    InitFailed,
    NotInitialized,
    InvalidFormat,
    BufferTooSmall,
    IoError,
}

impl core::fmt::Display for AudioError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AudioError::DeviceNotFound => write!(f, "Audio device not found"),
            AudioError::InitFailed => write!(f, "Audio initialization failed"),
            AudioError::NotInitialized => write!(f, "Audio subsystem not initialized"),
            AudioError::InvalidFormat => write!(f, "Invalid audio format"),
            AudioError::BufferTooSmall => write!(f, "Audio buffer too small"),
            AudioError::IoError => write!(f, "Audio I/O error"),
        }
    }
}

