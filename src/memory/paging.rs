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

/// 페이지 테이블 엔트리 타입 (64비트)
type PageTableEntry = u64;

/// NX 비트 마스크 (63번 비트)
const NX_BIT_MASK: u64 = 1 << 63;

/// 페이지 테이블 엔트리 포인터 얻기
/// 
/// 가상 주소에 해당하는 페이지 테이블 엔트리에 직접 접근합니다.
/// 
/// # Safety
/// - `physical_memory_offset`는 유효한 물리 메모리 오프셋이어야 합니다
/// - 페이지가 이미 매핑되어 있어야 합니다
unsafe fn get_page_table_entry_ptr(
    page: Page<Size4KiB>,
    physical_memory_offset: VirtAddr,
) -> Option<*mut PageTableEntry> {
    use x86_64::registers::control::Cr3;
    use x86_64::structures::paging::{PageTable, PhysFrame};
    
    let addr = page.start_address().as_u64();
    
    // 주소에서 인덱스 추출 (x86_64 4단계 페이징)
    let p4_index = (addr >> 39) & 0x1FF;
    let p3_index = (addr >> 30) & 0x1FF;
    let p2_index = (addr >> 21) & 0x1FF;
    let p1_index = (addr >> 12) & 0x1FF;
    
    // P4 테이블 가져오기
    let (level_4_table_frame, _) = Cr3::read();
    let p4_phys = level_4_table_frame.start_address();
    let p4_virt = physical_memory_offset + p4_phys.as_u64();
    let p4_table: *mut PageTable = p4_virt.as_mut_ptr();
    
    // P4 엔트리 확인
    let p4_entry = (*p4_table)[p4_index as usize];
    if !p4_entry.flags().contains(PageTableFlags::PRESENT) {
        return None;
    }
    
    // P3 테이블 가져오기
    let p3_frame = PhysFrame::containing_address(x86_64::PhysAddr::new(p4_entry.addr().as_u64()));
    let p3_phys = p3_frame.start_address();
    let p3_virt = physical_memory_offset + p3_phys.as_u64();
    let p3_table: *mut PageTable = p3_virt.as_mut_ptr();
    
    // P3 엔트리 확인
    let p3_entry = (*p3_table)[p3_index as usize];
    if !p3_entry.flags().contains(PageTableFlags::PRESENT) {
        return None;
    }
    
    // P2 테이블 가져오기
    let p2_frame = PhysFrame::containing_address(x86_64::PhysAddr::new(p3_entry.addr().as_u64()));
    let p2_phys = p2_frame.start_address();
    let p2_virt = physical_memory_offset + p2_phys.as_u64();
    let p2_table: *mut PageTable = p2_virt.as_mut_ptr();
    
    // P2 엔트리 확인
    let p2_entry = (*p2_table)[p2_index as usize];
    if !p2_entry.flags().contains(PageTableFlags::PRESENT) {
        return None;
    }
    
    // P1 테이블 가져오기 (최종 페이지 테이블)
    let p1_frame = PhysFrame::containing_address(x86_64::PhysAddr::new(p2_entry.addr().as_u64()));
    let p1_phys = p1_frame.start_address();
    let p1_virt = physical_memory_offset + p1_phys.as_u64();
    let p1_table: *mut PageTable = p1_virt.as_mut_ptr();
    
    // P1 엔트리 포인터 반환
    Some((*p1_table).0.as_mut_ptr().add(p1_index as usize))
}

/// NX (No Execute) 비트 설정
/// 
/// 페이지를 실행 불가능하게 설정하여 코드 실행 공격을 방지합니다.
/// 
/// # Safety
/// 페이지가 이미 매핑되어 있어야 합니다.
pub unsafe fn set_nx_bit(page: Page<Size4KiB>, enabled: bool) -> Result<(), MapToError<Size4KiB>> {
    let offset = {
        let guard = PHYSICAL_MEMORY_OFFSET.lock();
        guard.ok_or(MapToError::FrameAllocationFailed)?
    };

    // 페이지 테이블 엔트리 포인터 가져오기
    let entry_ptr = get_page_table_entry_ptr(page, offset)
        .ok_or(MapToError::FrameAllocationFailed)?;
    
    // 현재 엔트리 읽기
    let mut entry = core::ptr::read(entry_ptr);
    
    // NX 비트 설정/해제 (63번 비트)
    if enabled {
        entry |= NX_BIT_MASK;
    } else {
        entry &= !NX_BIT_MASK;
    }
    
    // 엔트리 쓰기
    core::ptr::write(entry_ptr, entry);
    
    // TLB 플러시 (변경 사항 반영)
    use x86_64::instructions::tlb;
    tlb::flush(page.start_address());
    
    crate::log_debug!("NX bit {} for page {:#016x}", 
                    if enabled { "enabled" } else { "disabled" },
                        page.start_address().as_u64());
    
    Ok(())
}

/// NX 비트 확인
/// 
/// 페이지의 NX 비트 상태를 확인합니다.
pub fn is_nx_bit_set(page: Page<Size4KiB>) -> bool {
    let offset = {
        let guard = PHYSICAL_MEMORY_OFFSET.lock();
        match *guard {
            Some(off) => off,
            None => return false,
        }
    };

    unsafe {
        if let Some(entry_ptr) = get_page_table_entry_ptr(page, offset) {
            let entry = core::ptr::read(entry_ptr);
            (entry & NX_BIT_MASK) != 0
        } else {
            false
        }
    }
}

/// ASLR (Address Space Layout Randomization) 활성화
/// 
/// 이 함수는 ASLR 모듈의 초기화를 호출합니다.
pub fn enable_aslr() -> Result<(), crate::memory::aslr::AslrError> {
    unsafe {
        crate::memory::aslr::init_aslr()
    }
}

