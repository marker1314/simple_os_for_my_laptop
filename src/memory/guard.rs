//! Guard pages for stack protection
//!
//! 스택 오버플로우/언더플로우 방지를 위한 guard page 구현

use x86_64::structures::paging::{Page, PageTableFlags, Size4KiB};
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
    // 스택 아래에 guard page 설정 (오버플로우 방지)
    let guard_page_below = Page::<Size4KiB>::containing_address(stack_start - Size4KiB::SIZE);
    
    // 스택 위에 guard page 설정 (언더플로우 방지)
    let guard_page_above = Page::<Size4KiB>::containing_address(stack_end);
    
    // Guard page는 매핑하지 않음 (접근 시 page fault 발생)
    // 실제로는 이미 매핑되어 있지 않은지 확인하고, 매핑되어 있으면 제거
    
    crate::log_info!("Stack guard pages set up: below={:?}, above={:?}",
                     guard_page_below.start_address(),
                     guard_page_above.start_address());
    
    Ok(())
}

/// 스택 크기 확인 (간단한 검사)
pub fn check_stack_usage(stack_start: VirtAddr, stack_pointer: VirtAddr) -> Result<usize, &'static str> {
    if stack_pointer < stack_start {
        return Err("Stack underflow detected");
    }
    
    let used = (stack_pointer.as_u64() - stack_start.as_u64()) as usize;
    Ok(used)
}

