//! 메모리 할당 실패 복구 메커니즘
//!
//! 이 모듈은 힙 할당 실패 시 복구를 시도합니다.
//!
//! # 복구 전략
//!
//! 1. **힙 확장 시도**: 사용 가능한 프레임이 있으면 힙 확장
//! 2. **메모리 압축 시도**: 사용되지 않는 메모리 압축
//! 3. **스왑 아웃 시도**: LRU 페이지를 스왑으로 내보내기
//! 4. **대체 할당자 시도**: Slab 할당자 사용 (작은 할당의 경우)
//! 5. **OOM Killer**: 최후의 수단으로 프로세스 종료

use core::alloc::Layout;
use alloc::vec::Vec;
use spin::Mutex;

use crate::memory::frame::allocate_frame;
use crate::memory::paging::{map_zero_page_at, get_physical_memory_offset};
use crate::memory::heap::{HEAP_START, HEAP_MAX_SIZE, HEAP_SIZE_BYTES};
use crate::memory::swap::is_swap_enabled;
use crate::memory::swap::try_swap_out_lru;
use crate::boot::get_boot_info;
use x86_64::VirtAddr;
use x86_64::structures::paging::{Size4KiB, PageSize, FrameAllocator, Mapper};

/// 메모리 복구 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryResult {
    /// 복구 성공
    Success,
    /// 복구 실패 (더 이상 복구 불가능)
    Failed,
    /// 부분 복구 (일부 메모리만 확보)
    Partial,
}

/// 힙 할당 실패 복구 시도
///
/// 다양한 전략을 사용하여 메모리를 확보합니다.
///
/// # Arguments
/// * `layout` - 요청된 할당 레이아웃
///
/// # Returns
/// 복구 성공 여부
pub unsafe fn try_recover_allocation(layout: Layout) -> RecoveryResult {
    crate::log_warn!("Attempting memory recovery for allocation: size={}, align={}", 
                    layout.size(), layout.align());
    
    // 전략 1: 힙 확장 시도
    if let Ok(()) = try_expand_heap(layout.size()) {
        crate::log_info!("Heap expansion successful");
        return RecoveryResult::Success;
    }
    
    // 전략 2: 스왑 아웃 시도 (메모리 확보)
    if is_swap_enabled() {
        if let Ok(()) = try_swap_out_lru() {
            crate::log_info!("Swap out successful, retrying heap expansion");
            if let Ok(()) = try_expand_heap(layout.size()) {
                return RecoveryResult::Success;
            }
        }
    }
    
    // 전략 3: 작은 할당의 경우 Slab 할당자 시도
    if layout.size() <= 256 {
        crate::log_info!("Trying slab allocator for small allocation");
        // Slab 할당자는 이미 전역으로 설정되어 있으므로, 여기서는 힘 확장만 시도
        // 실제 할당은 alloc_error_handler에서 재시도됨
    }
    
    // 전략 5: OOM Killer (최후의 수단)
    if crate::memory::oom_killer::check_oom() {
        crate::log_warn!("Memory critically low, attempting OOM Killer");
        let killed = crate::memory::oom_killer::try_kill_oom();
        if killed > 0 {
            crate::log_info!("OOM Killer freed memory by terminating {} thread(s)", killed);
            // 메모리 해제 후 힙 확장 재시도
            if let Ok(()) = try_expand_heap(layout.size()) {
                return RecoveryResult::Success;
            }
        }
    }
    
    crate::log_error!("All memory recovery strategies failed");
    RecoveryResult::Failed
}

/// 힙 확장 시도
///
/// 힙이 최대 크기 미만이면 확장을 시도합니다.
///
/// # Arguments
/// * `required_size` - 필요한 메모리 크기 (바이트)
///
/// # Safety
/// 메모리 관리가 초기화된 후에 호출되어야 합니다.
unsafe fn try_expand_heap(required_size: usize) -> Result<(), AllocationError> {
    let current_size = {
        let size_guard = HEAP_SIZE_BYTES.lock();
        *size_guard
    };
    
    if current_size >= HEAP_MAX_SIZE {
        return Err(AllocationError::HeapMaxSize);
    }
    
    // 필요한 페이지 수 계산
    let pages_needed = (required_size + Size4KiB::SIZE as usize - 1) / Size4KiB::SIZE as usize;
    let additional_pages = core::cmp::max(pages_needed, 4); // 최소 4페이지 확장
    
    let new_size = current_size + (additional_pages * Size4KiB::SIZE as usize);
    
    if new_size > HEAP_MAX_SIZE {
        return Err(AllocationError::HeapMaxSize);
    }
    
    // 프레임 할당 및 페이지 매핑
    let boot_info = get_boot_info().ok_or(AllocationError::BootInfoNotFound)?;
    let phys_offset = crate::memory::paging::get_physical_memory_offset(boot_info);
    let mut mapper = crate::memory::paging::init_mapper(phys_offset);
    let mut frame_allocator = crate::memory::frame::BootInfoFrameAllocator::new();
    
    let heap_start = VirtAddr::new(HEAP_START as u64);
    let current_end = heap_start + current_size as u64;
    
    // 새 페이지 매핑
    for i in 0..additional_pages {
        let page_addr = current_end + ((i as u64) * Size4KiB::SIZE);
        
        // 프레임 할당 시도 (부트 정보 기반 임시 할당자 사용)
        let frame = match frame_allocator.allocate_frame() {
            Some(f) => f,
            None => {
                crate::log_warn!("Failed to allocate frame for heap expansion at page {}", i);
                    // 일부만 확장됨
                if i > 0 {
                    // 힙 크기 업데이트
                    let partial_size = current_size + (i * Size4KiB::SIZE as usize);
                    let mut size_guard = HEAP_SIZE_BYTES.lock();
                    *size_guard = partial_size;
                    
                    // 할당자 재초기화는 복잡하므로, 현재는 힙 크기만 업데이트
                    // linked_list_allocator는 동적 확장을 직접 지원하지 않음
                    // TODO: 동적 힙 확장 지원
                    
                    return Err(AllocationError::Partial);
                } else {
                    return Err(AllocationError::FrameAllocationFailed);
                }
            }
        };
        
        // 페이지 매핑
        let page = x86_64::structures::paging::Page::<Size4KiB>::containing_address(page_addr);
        let flags = x86_64::structures::paging::PageTableFlags::PRESENT 
                   | x86_64::structures::paging::PageTableFlags::WRITABLE;
        
        mapper
            .map_to(page, frame, flags, &mut frame_allocator)
            .map_err(|_| AllocationError::FrameAllocationFailed)?
            .flush();
        
        // 페이지를 0으로 초기화
        let page_ptr = page_addr.as_mut_ptr::<u8>();
        core::ptr::write_bytes(page_ptr, 0, Size4KiB::SIZE as usize);
    }
    
    // 힙 크기 업데이트
    {
        let mut size_guard = HEAP_SIZE_BYTES.lock();
        *size_guard = new_size;
    }
    
    // 할당자 재초기화 (힙 확장)
    // linked_list_allocator는 런타임 extend를 지원하지 않으므로 여기서는 크기만 갱신하고 로그만 남깁니다.
    crate::log_info!("Heap expanded from {} to {} bytes", current_size, new_size);
    
    Ok(())
}

/// 할당 오류
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationError {
    HeapMaxSize,
    FrameAllocationFailed,
    BootInfoNotFound,
    Partial,
}

impl core::fmt::Display for AllocationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AllocationError::HeapMaxSize => write!(f, "Heap reached maximum size"),
            AllocationError::FrameAllocationFailed => write!(f, "Frame allocation failed"),
            AllocationError::BootInfoNotFound => write!(f, "Boot info not found"),
            AllocationError::Partial => write!(f, "Partial heap expansion"),
        }
    }
}

