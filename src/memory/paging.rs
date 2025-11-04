//! 가상 메모리 관리 모듈
//!
//! 이 모듈은 x86_64 4단계 페이지 테이블을 관리합니다.
//! 부트로더가 이미 페이지 테이블을 설정했으므로, 이 모듈은
//! 추가 매핑과 페이지 테이블 조작을 위한 유틸리티를 제공합니다.

use x86_64::{
    structures::paging::OffsetPageTable,
    VirtAddr,
};
use bootloader_api::BootInfo;

/// 부트로더가 설정한 페이지 테이블에 접근하기 위한 매퍼 생성
///
/// # Safety
/// - `physical_memory_offset`는 부트로더가 설정한 물리 메모리 오프셋이어야 합니다
/// - 페이지 테이블이 유효해야 합니다
pub unsafe fn init_mapper(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    use x86_64::registers::control::Cr3;

    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// 부트로더가 설정한 레벨 4 페이지 테이블에 접근
///
/// # Safety
/// `physical_memory_offset`는 부트로더가 설정한 물리 메모리 오프셋이어야 합니다.
unsafe fn active_level_4_table(
    physical_memory_offset: VirtAddr,
) -> &'static mut x86_64::structures::paging::PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut x86_64::structures::paging::PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

/// 부트로더 BootInfo에서 물리 메모리 오프셋 가져오기
///
/// bootloader_api 0.11.12는 물리 메모리를 가상 주소 공간에 매핑합니다.
pub fn get_physical_memory_offset(boot_info: &BootInfo) -> VirtAddr {
    // bootloader_api 0.11.12는 BootInfo에 physical_memory_offset 필드가 있습니다
    boot_info.physical_memory_offset.into()
}

/// 페이지 테이블 정보 출력 (디버깅용)
pub fn print_page_table_info() {
    use x86_64::registers::control::Cr3;
    
    let (level_4_table_frame, flags) = Cr3::read();
    crate::log_info!(
        "Page table root: {:?}, flags: {:?}",
        level_4_table_frame.start_address(),
        flags
    );
}

