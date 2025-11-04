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
            // bootloader_api 0.11.12의 BootInfo에서 RSDP 주소 추출
            // BootInfo 구조체의 rsdp_addr 필드 확인
            match info.rsdp_addr {
                bootloader_api::info::Optional::Some(addr) => Some(addr),
                bootloader_api::info::Optional::None => None,
            }
        })
    }
}

/// 프레임버퍼 가져오기
///
/// # Safety
/// `init`이 먼저 호출되어야 합니다.
pub unsafe fn get_framebuffer() -> Option<&'static mut bootloader_api::info::FrameBuffer> {
    BOOT_INFO.and_then(|info| {
        match &info.framebuffer {
            bootloader_api::info::Optional::Some(fb) => {
                // FrameBuffer는 &'static Option<FrameBuffer>이므로 unsafe 변환 필요
                let fb_ptr = fb as *const _ as *mut bootloader_api::info::FrameBuffer;
                Some(&mut *fb_ptr)
            }
            bootloader_api::info::Optional::None => None,
        }
    })
}

