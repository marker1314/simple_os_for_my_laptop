//! 물리 메모리 프레임 할당자
//!
//! 이 모듈은 4KB 페이지 단위로 물리 메모리를 할당하고 해제합니다.

use x86_64::PhysAddr;
use x86_64::structures::paging::{FrameAllocator, PhysFrame, PageSize, Size4KiB};
use spin::Mutex;

use crate::memory::map::{get as get_memory_map, MemoryType};

/// 프레임 할당자 구현
///
/// 사용 가능한 메모리 영역을 추적하고 프레임을 할당합니다.
/// 간단한 구현으로, 메모리 맵을 직접 순회합니다.
pub struct BootInfoFrameAllocator {
    /// 현재 할당 중인 영역 인덱스 (전체 메모리 맵에서)
    current_region_index: usize,
    /// 현재 영역 내의 다음 프레임 오프셋 (프레임 단위)
    current_frame_offset: u64,
}

impl BootInfoFrameAllocator {
    /// 새로운 프레임 할당자 생성
    pub fn new() -> Self {
        Self {
            current_region_index: 0,
            current_frame_offset: 0,
        }
    }

    /// 다음 사용 가능한 프레임 찾기
    fn find_next_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        unsafe {
            let memory_map = get_memory_map();
            
            // 사용 가능한 영역만 수집
            let usable_regions: alloc::vec::Vec<_> = memory_map.usable_regions().collect();
            
            // 모든 사용 가능한 영역을 순회
            while self.current_region_index < usable_regions.len() {
                let region = usable_regions[self.current_region_index];
                let start = region.start.as_u64();
                let end = start + region.length;
                
                // 4KB 정렬
                let aligned_start = align_up(start, Size4KiB::SIZE);
                let aligned_end = align_down(end, Size4KiB::SIZE);
                
                if aligned_start < aligned_end {
                    let frame_start = aligned_start + (self.current_frame_offset * Size4KiB::SIZE);
                    
                    if frame_start + Size4KiB::SIZE <= aligned_end {
                        let frame = PhysFrame::containing_address(PhysAddr::new(frame_start));
                        self.current_frame_offset += 1;
                        return Some(frame);
                    }
                }
                
                // 현재 영역의 모든 프레임을 사용했으므로 다음 영역으로
                self.current_region_index += 1;
                self.current_frame_offset = 0;
            }
            
            None
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        self.find_next_frame()
    }
}

/// 전역 프레임 할당자
static FRAME_ALLOCATOR: Mutex<Option<BootInfoFrameAllocator>> = Mutex::new(None);

/// 프레임 할당자 초기화
pub fn init() {
    let mut allocator = FRAME_ALLOCATOR.lock();
    *allocator = Some(BootInfoFrameAllocator::new());
}

/// 프레임 할당
pub fn allocate_frame() -> Option<PhysFrame<Size4KiB>> {
    let mut allocator = FRAME_ALLOCATOR.lock();
    allocator.as_mut()?.allocate_frame()
}

/// 주소를 위로 정렬 (4KB 경계)
fn align_up(addr: u64, align: u64) -> u64 {
    (addr + align - 1) & !(align - 1)
}

/// 주소를 아래로 정렬 (4KB 경계)
fn align_down(addr: u64, align: u64) -> u64 {
    addr & !(align - 1)
}

