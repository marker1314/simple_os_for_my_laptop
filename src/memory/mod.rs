//! 메모리 관리 모듈
//!
//! 이 모듈은 물리 메모리 및 가상 메모리 관리를 담당합니다.

pub mod map;
pub mod frame;
pub mod paging;
pub mod heap;
pub mod slab;
pub mod guard;
pub mod swap;
pub mod aslr;
pub mod recovery;
pub mod frame_cache;
pub mod compression;
pub mod fragmentation;
pub mod oom_killer;
pub mod stack_canary;
pub mod leak_detector;

pub use map::{init as init_memory_map, get as get_memory_map, MemoryMap, MemoryType, ParsedMemoryRegion};
pub use frame::{init as init_frame_allocator, allocate_frame};
pub use paging::{init_mapper, get_physical_memory_offset, print_page_table_info, set_physical_memory_offset};
pub use heap::{init_heap, HEAP_START};
pub use slab::SLAB;

use bootloader_api::BootInfo;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::Size4KiB;

/// 메모리 관리 시스템 초기화
///
/// 다음 순서로 초기화합니다:
/// 1. 메모리 맵 파싱
/// 2. 프레임 할당자 초기화
/// 3. 힙 할당자 초기화
///
/// # Safety
/// - `boot_info`는 유효한 BootInfo여야 합니다
/// - 이 함수는 한 번만 호출되어야 합니다
pub unsafe fn init(boot_info: &'static BootInfo) -> Result<(), MapToError<Size4KiB>> {
    // 1. 메모리 맵 파싱
    map::init(&boot_info.memory_regions);
    crate::log_info!("Memory map parsed: {} regions", boot_info.memory_regions.len());
    
    // 사용 가능한 메모리 크기 출력
    let memory_map = map::get();
    let usable_memory = memory_map.total_usable_memory();
    crate::log_info!("Total usable memory: {} KB", usable_memory / 1024);
    
    // 2. 프레임 할당자 초기화
    frame::init();
    crate::log_info!("Frame allocator initialized");
    
    // 3. 힙 할당자 초기화
    // Also cache physical memory offset for later dynamic mappings
    let phys_off = paging::get_physical_memory_offset(boot_info);
    paging::set_physical_memory_offset(phys_off);
    heap::init_heap(boot_info)?;
    crate::log_info!("Heap allocator initialized at {:p}", HEAP_START as *const u8);
    
    // 4. 페이지 테이블 정보 출력 (디버깅)
    paging::print_page_table_info();
    
    // 5. 메모리 누수 감지기 초기화
    leak_detector::init();
    
    Ok(())
}
