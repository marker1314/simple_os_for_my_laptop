//! 전력 관리 모듈
//!
//! 이 모듈은 CPU 전력 관리 및 ACPI 파싱을 담당합니다.
//!
//! ## 기능
//! - ACPI 테이블 파싱 (RSDP, RSDT/XSDT, FADT 등)
//! - CPU 클럭 스케일링 (P-State 제어)
//! - CPU 유휴 상태 관리 (C-State 제어)
//! - 전력 정책 관리

pub mod manager;
pub mod acpi;
pub mod scaling;
pub mod policy;
pub mod idle;
pub mod device;
pub mod device_policy;
pub mod stats;
pub mod rapl;
pub mod temps;
pub mod cpu_usage;
pub mod device_hooks;

pub use manager::PowerManager;
pub use policy::{PowerPolicy, PowerMode};
pub use idle::{IdleStateManager, CState};
pub use scaling::ScalingGovernor;

use spin::Mutex;

/// 전역 전력 관리자 인스턴스
static POWER_MANAGER: Mutex<Option<PowerManager>> = Mutex::new(None);

/// 전력 관리자 초기화
///
/// 부트 정보에서 ACPI RSDP 주소를 찾아 전력 관리자를 초기화합니다.
///
/// # Safety
/// 이 함수는 한 번만 호출되어야 하며, 메모리 관리가 초기화된 후에 호출되어야 합니다.
pub unsafe fn init() -> Result<(), PowerError> {
    let mut manager = PowerManager::new()?;
    manager.init()?;
    
    let mut global_manager = POWER_MANAGER.lock();
    *global_manager = Some(manager);
    
    Ok(())
}

/// 전력 관리자 가져오기
///
/// 초기화되지 않은 경우 None을 반환합니다.
pub fn get_manager() -> Option<&'static Mutex<Option<PowerManager>>> {
    Some(&POWER_MANAGER)
}

/// Enter S3 sleep (best-effort stub)
pub fn suspend_s3() -> Result<(), PowerError> {
    let mut pm = POWER_MANAGER.lock();
    if let Some(ref mut m) = *pm {
        m.suspend_s3()
    } else {
        Err(PowerError::NotInitialized)
    }
}

/// Resume from sleep (stub)
pub fn resume() -> Result<(), PowerError> {
    let mut pm = POWER_MANAGER.lock();
    if let Some(ref mut m) = *pm {
        m.resume()
    } else {
        Err(PowerError::NotInitialized)
    }
}

/// 런타임 전력 모드 변경 (편의 래퍼)
pub fn set_mode(mode: PowerMode) -> Result<(), PowerError> {
    let mut pm = POWER_MANAGER.lock();
    if let Some(ref mut m) = *pm {
        m.set_policy(mode)
    } else {
        Err(PowerError::NotInitialized)
    }
}

/// 전력 관리 오류
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerError {
    /// ACPI RSDP를 찾을 수 없음
    RsdpNotFound,
    /// ACPI 테이블 파싱 실패
    AcpiParseError,
    /// MSR 접근 실패
    MsrAccessError,
    /// 전력 관리자가 초기화되지 않음
    NotInitialized,
    /// 지원하지 않는 기능
    Unsupported,
}
