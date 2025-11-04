//! 부트로더 정보 구조체
//!
//! 이 모듈은 부트로더에서 전달된 정보를 파싱하고 저장합니다.

use bootloader_api::BootInfo;

/// 부트 정보 전역 변수
///
/// 부트로더가 전달한 정보를 저장합니다.
/// 초기화 후에는 읽기 전용으로 사용됩니다.
static mut BOOT_INFO: Option<&'static BootInfo> = None;

/// 부트 정보 초기화
///
/// `_start` 함수에서 부트로더로부터 받은 정보를 저장합니다.
///
/// # Safety
/// 이 함수는 한 번만 호출되어야 하며, 부트로더가 전달한 유효한 BootInfo를
/// 가리켜야 합니다.
pub unsafe fn init(boot_info: &'static BootInfo) {
    BOOT_INFO = Some(boot_info);
}

/// 부트 정보 가져오기
///
/// # Safety
/// `init`이 먼저 호출되어야 합니다.
pub unsafe fn get() -> &'static BootInfo {
    BOOT_INFO.expect("BootInfo not initialized")
}

/// 메모리 맵 엔트리 수
pub fn memory_map_len() -> usize {
    unsafe {
        BOOT_INFO
            .map(|info| info.memory_regions.len())
            .unwrap_or(0)
    }
}

/// ACPI RSDP (Root System Description Pointer) 주소
pub fn acpi_rsdp_addr() -> Option<u64> {
    unsafe {
        BOOT_INFO.and_then(|info| {
            // bootloader 크레이트의 BootInfo에서 RSDP 주소 추출
            // 실제 구현은 bootloader 버전에 따라 다를 수 있음
            None // TODO: RSDP 주소 파싱 구현
        })
    }
}

