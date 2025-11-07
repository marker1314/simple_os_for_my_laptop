//! 프레임 캐싱 메커니즘
//!
//! 이 모듈은 메모리 단편화를 최소화하기 위해 프레임 캐싱을 제공합니다.
//!
//! # 프레임 캐싱
//!
//! 1. **해제된 프레임 캐싱**: 해제된 프레임을 즉시 재사용 가능하게 유지
//! 2. **연속 프레임 할당**: 연속된 프레임 할당을 우선시
//! 3. **단편화 최소화**: 작은 프레임 블록을 병합

use x86_64::structures::paging::{PhysFrame, Size4KiB, FrameAllocator};
use spin::Mutex;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

// use crate::memory::frame::allocate_frame; // 순환 참조 방지
use crate::memory::frame::BootInfoFrameAllocator;

/// 프레임 캐시 엔트리
#[derive(Debug, Clone, Copy)]
struct CachedFrame {
    frame: PhysFrame<Size4KiB>,
    /// 캐시 시간 (타이머 틱)
    cache_time: u64,
}

/// 프레임 캐시
///
/// 해제된 프레임을 캐싱하여 재사용합니다.
pub struct FrameCache {
    /// 캐시된 프레임 목록
    cached_frames: Vec<CachedFrame>,
    /// 최대 캐시 크기
    max_cache_size: usize,
    /// 캐시 적중 횟수
    hits: u64,
    /// 캐시 미스 횟수
    misses: u64,
}

impl FrameCache {
    /// 새 프레임 캐시 생성
    pub fn new(max_size: usize) -> Self {
        Self {
            cached_frames: Vec::new(),
            max_cache_size: max_size,
            hits: 0,
            misses: 0,
        }
    }
    
    /// 캐시에서 프레임 가져오기
    ///
    /// 캐시된 프레임이 있으면 반환하고, 없으면 None을 반환합니다.
    pub fn get_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        if let Some(cached) = self.cached_frames.pop() {
            self.hits += 1;
            Some(cached.frame)
        } else {
            self.misses += 1;
            None
        }
    }
    
    /// 프레임을 캐시에 추가
    ///
    /// 캐시가 가득 차면 가장 오래된 프레임을 제거합니다.
    pub fn cache_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        let now = crate::drivers::timer::get_milliseconds();
        
        // 캐시 크기 제한 확인
        if self.cached_frames.len() >= self.max_cache_size {
            // 가장 오래된 프레임 제거 (FIFO)
            self.cached_frames.remove(0);
        }
        
        self.cached_frames.push(CachedFrame {
            frame,
            cache_time: now,
        });
    }
    
    /// 오래된 캐시 엔트리 정리
    ///
    /// 지정된 시간보다 오래된 캐시 엔트리를 제거합니다.
    pub fn cleanup_old_cache(&mut self, max_age_ms: u64) {
        let now = crate::drivers::timer::get_milliseconds();
        self.cached_frames.retain(|cached| {
            now.saturating_sub(cached.cache_time) < max_age_ms
        });
    }
    
    /// 캐시 통계
    pub fn stats(&self) -> (u64, u64, usize) {
        (self.hits, self.misses, self.cached_frames.len())
    }
    
    /// 캐시 비우기
    pub fn clear(&mut self) {
        self.cached_frames.clear();
    }
}

/// 전역 프레임 캐시
static FRAME_CACHE: Mutex<FrameCache> = Mutex::new(FrameCache {
    cached_frames: Vec::new(),
    max_cache_size: 64, // 최대 64개 프레임 캐싱 (256KB)
    hits: 0,
    misses: 0,
});

/// 프레임 캐시 초기화
pub fn init_cache(max_size: usize) {
    let mut cache = FRAME_CACHE.lock();
    cache.max_cache_size = max_size;
}

/// 캐시에서 프레임 할당 시도
///
/// 캐시에 프레임이 있으면 반환하고, 없으면 일반 할당자를 사용합니다.
pub fn allocate_frame_cached() -> Option<PhysFrame<Size4KiB>> {
    let mut cache = FRAME_CACHE.lock();
    
    // 캐시에서 시도
    if let Some(frame) = cache.get_frame() {
        return Some(frame);
    }
    
    // 캐시 미스 - 일반 할당자 사용
    drop(cache);
    
    // 프레임 할당자 직접 생성 (전역 접근 회피)
    let mut allocator = BootInfoFrameAllocator::new();
    allocator.allocate_frame()
}

/// 프레임을 캐시에 추가
pub fn cache_frame(frame: PhysFrame<Size4KiB>) {
    let mut cache = FRAME_CACHE.lock();
    cache.cache_frame(frame);
}

/// 오래된 캐시 정리
pub fn cleanup_cache(max_age_ms: u64) {
    let mut cache = FRAME_CACHE.lock();
    cache.cleanup_old_cache(max_age_ms);
}

/// 캐시 통계 가져오기
pub fn get_cache_stats() -> (u64, u64, usize) {
    let cache = FRAME_CACHE.lock();
    cache.stats()
}

