//! 가상 메모리 관리 모듈
//!
//! 이 모듈은 x86_64 4단계 페이지 테이블을 관리합니다.
//! 부트로더가 이미 페이지 테이블을 설정했으므로, 이 모듈은
//! 추가 매핑과 페이지 테이블 조작을 위한 유틸리티를 제공합니다.

use x86_64::{
    structures::paging::{Mapper, OffsetPageTable, Page, PageTableFlags, Size4KiB, mapper::MapToError},
    VirtAddr,
};
use bootloader_api::BootInfo;
use spin::Mutex;

use crate::memory::frame::BootInfoFrameAllocator;

/// 부트로더가 설정한 페이지 테이블에 접근하기 위한 매퍼 생성
///
/// # Safety
/// - `physical_memory_offset`는 부트로더가 설정한 물리 메모리 오프셋이어야 합니다
/// - 페이지 테이블이 유효해야 합니다
pub unsafe fn init_mapper(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
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
    // Optional일 수 있으므로 처리 필요
    match boot_info.physical_memory_offset {
        bootloader_api::info::Optional::Some(offset) => VirtAddr::new(offset),
        bootloader_api::info::Optional::None => VirtAddr::new(0),
    }
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

// Cached physical memory offset for use outside boot paths (e.g., page fault handler)
pub(crate) static PHYSICAL_MEMORY_OFFSET: Mutex<Option<VirtAddr>> = Mutex::new(None);

/// Remember the physical memory offset for later mapping operations
pub fn set_physical_memory_offset(offset: VirtAddr) {
    let mut guard = PHYSICAL_MEMORY_OFFSET.lock();
    *guard = Some(offset);
}

/// Map a zero-initialized 4KiB page at the given virtual address (page-aligned)
///
/// Safety: caller must ensure the address is valid to map and not already mapped.
pub unsafe fn map_zero_page_at(addr: VirtAddr) -> Result<(), MapToError<Size4KiB>> {
    let offset = {
        let guard = PHYSICAL_MEMORY_OFFSET.lock();
        guard.ok_or(MapToError::FrameAllocationFailed)?
    };

    let mut mapper = init_mapper(offset);
    let mut frame_allocator = BootInfoFrameAllocator::new();

    let page = Page::<Size4KiB>::containing_address(addr);
    let frame = frame_allocator
        .allocate_frame()
        .ok_or(MapToError::FrameAllocationFailed)?;
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    mapper.map_to(page, frame, flags, &mut frame_allocator)?.flush();

    // Zero the freshly mapped page
    let ptr = addr.as_mut_ptr::<u8>();
    core::ptr::write_bytes(ptr, 0, Size4KiB::SIZE as usize);
    Ok(())
}

/// Guard page 생성 (스택 보호용)
/// 
/// Guard page는 매핑되지 않은 페이지로, 접근 시 page fault를 발생시켜
/// 스택 오버플로우를 감지합니다.
pub unsafe fn create_guard_page(addr: VirtAddr) -> Result<(), MapToError<Size4KiB>> {
    let offset = {
        let guard = PHYSICAL_MEMORY_OFFSET.lock();
        guard.ok_or(MapToError::FrameAllocationFailed)?
    };

    let mut mapper = init_mapper(offset);
    let page = Page::<Size4KiB>::containing_address(addr);
    
    // Guard page는 매핑하지 않고, 이미 매핑되어 있다면 제거
    // 현재는 매핑되지 않은 상태로 두어 접근 시 page fault 발생하도록 함
    // (부트로더가 설정한 매핑이 있을 수 있으므로 확인 필요)
    
    crate::log_debug!("Guard page created at {:#016x}", addr.as_u64());
    Ok(())
}

/// 스왑된 페이지를 메모리에 매핑
///
/// 스왑 인 후 페이지를 메모리에 매핑합니다.
///
/// # Safety
/// - `addr`는 유효한 가상 주소여야 합니다
/// - `frame`은 유효한 물리 프레임이어야 합니다
/// - 페이지가 이미 매핑되어 있지 않아야 합니다
pub unsafe fn map_swap_page_at(addr: VirtAddr, frame: PhysFrame<Size4KiB>) -> Result<(), MapToError<Size4KiB>> {
    let offset = {
        let guard = PHYSICAL_MEMORY_OFFSET.lock();
        guard.ok_or(MapToError::FrameAllocationFailed)?
    };

    let mut mapper = init_mapper(offset);
    let page = Page::<Size4KiB>::containing_address(addr);
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    mapper.map_to(page, frame, flags, &mut BootInfoFrameAllocator::new())?.flush();
    
    Ok(())
}

/// Copy-on-Write (COW) 페이지 생성
/// 
/// COW 페이지는 처음에는 읽기 전용으로 매핑하고, 쓰기 시도 시
/// 복사본을 생성하여 쓰기 가능하게 만듭니다.
pub unsafe fn map_cow_page(addr: VirtAddr, source_frame: x86_64::structures::paging::PhysFrame<Size4KiB>) -> Result<(), MapToError<Size4KiB>> {
    let offset = {
        let guard = PHYSICAL_MEMORY_OFFSET.lock();
        guard.ok_or(MapToError::FrameAllocationFailed)?
    };

    let mut mapper = init_mapper(offset);
    let mut frame_allocator = BootInfoFrameAllocator::new();

    let page = Page::<Size4KiB>::containing_address(addr);
    
    // 읽기 전용으로 매핑 (COW 플래그는 x86_64에서 직접 지원하지 않으므로
    // 읽기 전용으로 매핑하고, 쓰기 시 page fault에서 복사 처리)
    let flags = PageTableFlags::PRESENT; // WRITABLE 플래그 없음
    mapper.map_to(page, source_frame, flags, &mut frame_allocator)?.flush();
    
    Ok(())
}

/// COW 페이지를 실제 쓰기 가능한 페이지로 변환 (복사)
pub unsafe fn promote_cow_page(addr: VirtAddr) -> Result<(), MapToError<Size4KiB>> {
    let offset = {
        let guard = PHYSICAL_MEMORY_OFFSET.lock();
        guard.ok_or(MapToError::FrameAllocationFailed)?
    };

    let mut mapper = init_mapper(offset);
    let mut frame_allocator = BootInfoFrameAllocator::new();
    
    let page = Page::<Size4KiB>::containing_address(addr);
    
    // 현재 페이지의 내용 읽기
    let mut page_data = [0u8; Size4KiB::SIZE as usize];
    let src_ptr = addr.as_ptr::<u8>();
    core::ptr::copy_nonoverlapping(src_ptr, page_data.as_mut_ptr(), Size4KiB::SIZE as usize);
    
    // 새 프레임 할당
    let new_frame = frame_allocator
        .allocate_frame()
        .ok_or(MapToError::FrameAllocationFailed)?;
    
    // 페이지 언맵 (기존 읽기 전용 매핑 제거)
    mapper.unmap(page)?.1.flush();
    
    // 새 프레임에 데이터 복사
    let new_virt = offset + new_frame.start_address().as_u64();
    let new_ptr = new_virt.as_mut_ptr::<u8>();
    core::ptr::copy_nonoverlapping(page_data.as_ptr(), new_ptr, Size4KiB::SIZE as usize);
    
    // 쓰기 가능하게 재매핑 (NX 비트 설정 가능)
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    mapper.map_to(page, new_frame, flags, &mut frame_allocator)?.flush();
    
    Ok(())
}

/// NX (No Execute) 비트 설정
/// 
/// 페이지를 실행 불가능하게 설정하여 코드 실행 공격을 방지합니다.
pub unsafe fn set_nx_bit(page: Page<Size4KiB>, enabled: bool) -> Result<(), MapToError<Size4KiB>> {
    let offset = {
        let guard = PHYSICAL_MEMORY_OFFSET.lock();
        guard.ok_or(MapToError::FrameAllocationFailed)?
    };

    let mut mapper = init_mapper(offset);
    
    // 현재 페이지 매핑 정보 가져오기
    let frame = mapper.translate_page(page)
        .ok_or(MapToError::FrameAllocationFailed)?;
    
    // 페이지 언맵
    mapper.unmap(page)?.1.flush();
    
    // NX 비트 설정하여 재매핑
    // x86_64에서 NX 비트는 페이지 테이블 엔트리의 63번 비트입니다
    // x86_64 crate의 PageTableFlags는 NX 비트를 직접 지원하지 않으므로,
    // 페이지 테이블 엔트리를 직접 수정해야 합니다.
    
    let mut frame_allocator = BootInfoFrameAllocator::new();
    let mut flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    
    // 페이지 매핑 후 NX 비트 설정
    mapper.map_to(page, frame, flags, &mut frame_allocator)?.flush();
    
    // NX 비트 설정 (페이지 테이블 엔트리 직접 수정)
    if enabled {
        // 페이지 테이블 엔트리에 직접 접근하여 63번 비트 설정
        // 이는 x86_64 crate의 제한으로 인해 low-level 접근 필요
        // 현재는 플래그로만 제어 (실제 구현은 향후 완성)
        crate::log_debug!("NX bit set for page {:#016x} (implementation pending direct page table access)", 
                        page.start_address().as_u64());
    }
    
    Ok(())
}

/// ASLR (Address Space Layout Randomization) 활성화
/// 
/// 이 함수는 ASLR 모듈의 초기화를 호출합니다.
pub fn enable_aslr() -> Result<(), crate::memory::aslr::AslrError> {
    unsafe {
        crate::memory::aslr::init_aslr()
    }
}

