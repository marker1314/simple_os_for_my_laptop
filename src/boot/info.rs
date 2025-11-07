//! 부트로더 정보 구조체
//!
//! 이 모듈은 부트로더에서 전달된 정보를 파싱하고 저장합니다.

use bootloader_api::BootInfo;
use spin::Mutex;

/// 부트 정보 전역 변수 (안전한 캡슐화)
///
/// 부트로더가 전달한 정보를 저장합니다.
/// 초기화 후에는 읽기 전용으로 사용됩니다.
static BOOT_INFO: Mutex<Option<&'static BootInfo>> = Mutex::new(None);
// Store framebuffer address as usize to satisfy Sync in static
static FRAMEBUFFER_ADDR: Mutex<Option<usize>> = Mutex::new(None);

/// 부트 정보 초기화
///
/// `_start` 함수에서 부트로더로부터 받은 정보를 저장합니다.
///
/// # Safety
/// 이 함수는 한 번만 호출되어야 하며, 부트로더가 전달한 유효한 BootInfo를
/// 가리켜야 합니다.
pub unsafe fn init(boot_info: &'static BootInfo) {
    // Store read-only view for general queries
    let mut guard = BOOT_INFO.lock();
    *guard = Some(boot_info);
}

pub unsafe fn capture_framebuffer(boot_info: &mut BootInfo) {
    if let bootloader_api::info::Optional::Some(fb) = &mut boot_info.framebuffer {
        let fb_ptr = fb as *mut _ as usize;
        *FRAMEBUFFER_ADDR.lock() = Some(fb_ptr);
    }
}

/// 부트 정보 가져오기
///
/// `BootInfo` 가져오기 (옵션)
pub fn get() -> Option<&'static BootInfo> {
    let guard = BOOT_INFO.lock();
    *guard
}

/// 메모리 맵 엔트리 수
pub fn memory_map_len() -> usize {
    let guard = BOOT_INFO.lock();
    guard
        .map(|info| info.memory_regions.len())
        .unwrap_or(0)
}

/// ACPI RSDP (Root System Description Pointer) 주소
pub fn acpi_rsdp_addr() -> Option<u64> {
    let guard = BOOT_INFO.lock();
    guard.and_then(|info| {
        match info.rsdp_addr {
            bootloader_api::info::Optional::Some(addr) => Some(addr),
            bootloader_api::info::Optional::None => None,
        }
    })
}

/// 프레임버퍼 가져오기
///
/// # Safety
/// `init`이 먼저 호출되어야 합니다.
pub unsafe fn get_framebuffer() -> Option<&'static mut bootloader_api::info::FrameBuffer> {
    if let Some(addr) = *FRAMEBUFFER_ADDR.lock() {
        let ptr = addr as *mut bootloader_api::info::FrameBuffer;
        Some(&mut *ptr)
    } else {
        None
    }
}

