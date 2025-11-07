//! Guard pages for stack protection
//!
//! 스택 오버플로우/언더플로우 방지를 위한 guard page 구현

use x86_64::structures::paging::{Page, PageTableFlags, Size4KiB, PageSize};
use x86_64::VirtAddr;
use crate::memory::paging::{init_mapper, get_physical_memory_offset};
use crate::memory::frame::BootInfoFrameAllocator;
use bootloader_api::BootInfo;

/// Guard page 설정
/// 
/// # Arguments
/// * `stack_start` - 스택 시작 주소 (낮은 주소)
/// * `stack_end` - 스택 끝 주소 (높은 주소)
/// 
/// # Safety
/// 스택이 유효한 가상 주소 범위여야 함
pub unsafe fn setup_stack_guard_pages(stack_start: VirtAddr, stack_end: VirtAddr) -> Result<(), &'static str> {
    use crate::memory::paging::create_guard_page;
    
    // 스택 아래에 guard page 설정 (오버플로우 방지)
    let guard_page_below = stack_start - Size4KiB::SIZE;
    if let Err(e) = create_guard_page(guard_page_below) {
        crate::log_warn!("Failed to create guard page below stack: {:?}", e);
    }
    
    // 스택 위에 guard page 설정 (언더플로우 방지)
    let guard_page_above = stack_end;
    if let Err(e) = create_guard_page(guard_page_above) {
        crate::log_warn!("Failed to create guard page above stack: {:?}", e);
    }
    
    crate::log_debug!("Stack guard pages set up: below={:?}, above={:?}",
                     guard_page_below,
                     guard_page_above);
    
    Ok(())
}

/// 스택에 Guard page 자동 생성
/// 
/// 스레드 생성 시 자동으로 호출되어 스택 주변에 guard page를 생성합니다.
pub unsafe fn auto_create_stack_guard(stack_start: u64, stack_size: usize) -> Result<(), &'static str> {
    let stack_start_va = VirtAddr::new(stack_start);
    let stack_end_va = VirtAddr::new(stack_start + stack_size as u64);
    setup_stack_guard_pages(stack_start_va, stack_end_va)
}

/// 스택 크기 확인 (간단한 검사)
pub fn check_stack_usage(stack_start: VirtAddr, stack_pointer: VirtAddr) -> Result<usize, &'static str> {
    if stack_pointer < stack_start {
        return Err("Stack underflow detected");
    }
    
    let used = (stack_pointer.as_u64() - stack_start.as_u64()) as usize;
    Ok(used)
}

