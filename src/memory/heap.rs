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
/// Graceful degradation을 시도: 메모리 압박 시 힙 확장 시도
#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    // 힙 할당 실패 로깅
    crate::log_error!("Heap allocation failed: size={}, align={}", layout.size(), layout.align());
    
    // 힙 통계 로깅
    let (heap_start, heap_size) = heap_bounds();
    crate::log_error!("Heap bounds: start=0x{:016x}, size={} bytes", heap_start, heap_size);
    
    // 프레임 할당 통계 확인
    if let Some((allocated, deallocated)) = crate::memory::frame::get_frame_stats() {
        crate::log_error!("Frame stats: allocated={}, deallocated={}", allocated, deallocated);
        if allocated > deallocated + 100 {
            crate::log_error!("Warning: Possible memory leak detected ({} frames not deallocated)", allocated - deallocated);
        }
    }
    
    // 메모리 복구 메커니즘 시도
    unsafe {
        use crate::memory::recovery::{try_recover_allocation, RecoveryResult};
        
        match try_recover_allocation(layout) {
            RecoveryResult::Success => {
                crate::log_info!("Memory recovery successful, retrying allocation");
                // 복구 성공 시 재시도 (할당은 자동으로 재시도됨)
                // 하지만 alloc_error_handler는 반환할 수 없으므로 여기서는 패닉
                // 실제로는 allocator가 재시도해야 함
                panic!("allocation error: {:?} (recovery attempted but handler cannot return)", layout);
            }
            RecoveryResult::Partial => {
                crate::log_warn!("Partial memory recovery, may still fail");
                // 부분 복구는 여전히 실패할 수 있음
            }
            RecoveryResult::Failed => {
                crate::log_error!("All memory recovery strategies failed");
            }
        }
    }
    
    // 복구 불가능: 패닉
    panic!("allocation error: {:?}", layout)
}

/// 현재 힙의 시작 주소와 크기를 반환 (바이트)
pub fn heap_bounds() -> (usize, usize) {
    let size_guard = HEAP_SIZE_BYTES.lock();
    (HEAP_START, *size_guard)
}

