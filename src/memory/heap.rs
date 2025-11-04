//! 힙 할당자 설정
//!
//! 이 모듈은 커널 힙 할당자를 초기화하고 전역 할당자로 설정합니다.

use linked_list_allocator::LockedHeap;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};
use bootloader_api::BootInfo;
use spin::Mutex;

use crate::memory::paging::{get_physical_memory_offset, init_mapper};
use crate::memory::frame::BootInfoFrameAllocator;

/// 힙 베이스 주소 및 초기/최대 크기
pub const HEAP_START: usize = 0x_4444_4444_0000;
const HEAP_INITIAL_SIZE: usize = 100 * 1024; // 100 KB
const HEAP_MAX_SIZE: usize = 2 * 1024 * 1024; // 2 MB cap
static HEAP_SIZE_BYTES: Mutex<usize> = Mutex::new(HEAP_INITIAL_SIZE);

/// 전역 힙 할당자
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// 힙 할당자 초기화
///
/// 힙 영역을 물리 메모리에 매핑하고 할당자를 초기화합니다.
///
/// # Safety
/// - `boot_info`는 유효한 BootInfo여야 합니다
/// - 이 함수는 한 번만 호출되어야 합니다
/// - 페이지 테이블이 유효해야 합니다
pub unsafe fn init_heap(
    boot_info: &BootInfo,
) -> Result<(), MapToError<Size4KiB>> {
    let phys_mem_offset = get_physical_memory_offset(boot_info);
    let mut mapper = init_mapper(phys_mem_offset);
    let mut frame_allocator = BootInfoFrameAllocator::new();

    // 시스템 메모리 상황에 따라 힙 크기를 동적으로 설정 (최대 2MB)
    let total_regions = boot_info.memory_regions.len();
    let dynamic_target = if total_regions > 0 { 512 * 1024 } else { HEAP_INITIAL_SIZE };
    {
        let mut size_guard = HEAP_SIZE_BYTES.lock();
        *size_guard = core::cmp::min(HEAP_MAX_SIZE, core::cmp::max(HEAP_INITIAL_SIZE, dynamic_target));
    }

    // 힙 영역을 페이지로 변환
    let heap_start = VirtAddr::new(HEAP_START as u64);
    let heap_end = {
        let size_guard = HEAP_SIZE_BYTES.lock();
        heap_start + (*size_guard as u64) - 1u64
    };
    let heap_start_page = Page::containing_address(heap_start);
    let heap_end_page = Page::containing_address(heap_end);

    // 힙 영역의 모든 페이지를 매핑
    for page in Page::range_inclusive(heap_start_page, heap_end_page) {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, &mut frame_allocator)?.flush();
        }
    }

    // 할당자 초기화
    unsafe {
        let size_guard = HEAP_SIZE_BYTES.lock();
        ALLOCATOR.lock().init(HEAP_START as *mut u8, *size_guard);
    }

    Ok(())
}

/// 힙 할당 오류 핸들러
///
/// 힙 할당이 실패했을 때 호출됩니다.
#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

/// 현재 힙의 시작 주소와 크기를 반환 (바이트)
pub fn heap_bounds() -> (usize, usize) {
    let size_guard = HEAP_SIZE_BYTES.lock();
    (HEAP_START, *size_guard)
}

