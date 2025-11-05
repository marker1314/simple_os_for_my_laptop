//! 물리 메모리 프레임 할당자
//!
//! 이 모듈은 4KB 페이지 단위로 물리 메모리를 할당하고 해제합니다.

use x86_64::PhysAddr;
use x86_64::structures::paging::{FrameAllocator, PhysFrame, PageSize, Size4KiB};
use spin::Mutex;
use alloc::vec::Vec;

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
    /// 해제된 프레임을 재사용하기 위한 자유 리스트
    free_list: Vec<PhysFrame<Size4KiB>>,
    /// 할당된 프레임 추적 (디버그 모드에서만 활성)
    #[cfg(debug_assertions)]
    allocated_frames: Vec<PhysFrame<Size4KiB>>,
    /// 할당된 프레임 수
    allocated_count: usize,
    /// 해제된 프레임 수
    deallocated_count: usize,
}

impl BootInfoFrameAllocator {
    /// 새로운 프레임 할당자 생성
    pub fn new() -> Self {
        Self {
            current_region_index: 0,
            current_frame_offset: 0,
            free_list: Vec::new(),
            #[cfg(debug_assertions)]
            allocated_frames: Vec::new(),
            allocated_count: 0,
            deallocated_count: 0,
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

    /// 해제된 프레임을 자유 리스트에 추가
    pub fn deallocate(&mut self, frame: PhysFrame<Size4KiB>) {
        #[cfg(debug_assertions)]
        {
            // 디버그 모드: 할당 목록에서 제거 확인
            if let Some(pos) = self.allocated_frames.iter().position(|&f| f.start_address() == frame.start_address()) {
                self.allocated_frames.remove(pos);
            } else {
                crate::log_warn!("Double-free or invalid frame deallocation: {:?}", frame.start_address());
            }
        }
        
        self.deallocated_count += 1;
        self.free_list.push(frame);
    }
    
    /// 메모리 누수 검사 (디버그 모드)
    #[cfg(debug_assertions)]
    pub fn check_leaks(&self) -> usize {
        self.allocated_frames.len()
    }
    
    /// 할당/해제 통계
    pub fn get_stats(&self) -> (usize, usize) {
        (self.allocated_count, self.deallocated_count)
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = if let Some(frame) = self.free_list.pop() {
            // 자유 리스트에서 재사용
            frame
        } else {
            // 새 프레임 할당
            self.find_next_frame()?
        };
        
        #[cfg(debug_assertions)]
        {
            self.allocated_frames.push(frame);
        }
        
        self.allocated_count += 1;
        Some(frame)
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
    // 프레임 캐시에서 먼저 시도
    crate::memory::frame_cache::allocate_frame_cached()
}

/// 프레임 해제 (전역)
pub fn deallocate_frame(frame: PhysFrame<Size4KiB>) {
    // 프레임 캐시에 먼저 추가 시도
    crate::memory::frame_cache::cache_frame(frame);
    
    // 기존 할당자에도 기록 (통계용)
    let mut allocator = FRAME_ALLOCATOR.lock();
    if let Some(ref mut alloc) = *allocator {
        alloc.deallocate(frame);
    }
}

/// 메모리 누수 검사 (디버그 모드)
#[cfg(debug_assertions)]
pub fn check_memory_leaks() -> Option<usize> {
    let allocator = FRAME_ALLOCATOR.lock();
    allocator.as_ref().map(|alloc| alloc.check_leaks())
}

/// 프레임 할당 통계 가져오기
pub fn get_frame_stats() -> Option<(usize, usize)> {
    let allocator = FRAME_ALLOCATOR.lock();
    allocator.as_ref().map(|alloc| alloc.get_stats())
}

/// 주소를 위로 정렬 (4KB 경계)
fn align_up(addr: u64, align: u64) -> u64 {
    (addr + align - 1) & !(align - 1)
}

/// 주소를 아래로 정렬 (4KB 경계)
fn align_down(addr: u64, align: u64) -> u64 {
    addr & !(align - 1)
}

