//! 메모리 단편화 모니터링 및 최소화
//!
//! 이 모듈은 메모리 단편화를 추적하고 최소화하는 메커니즘을 제공합니다.
//!
//! # 단편화 관리
//!
//! 1. **단편화 추적**: 힙의 단편화 정도 측정
//! 2. **통계 수집**: 단편화 통계 및 히스토리
//! 3. **압축 힌트**: 단편화가 심할 때 압축 또는 재구성 제안

use spin::Mutex;
use alloc::vec::Vec;

use crate::memory::heap::heap_bounds;

/// 메모리 단편화 통계
#[derive(Debug, Clone, Copy)]
pub struct FragmentationStats {
    /// 현재 단편화 비율 (0.0 - 1.0)
    pub fragmentation_ratio: f64,
    /// 사용 가능한 연속 메모리 블록 수
    pub free_blocks: usize,
    /// 가장 큰 연속 메모리 블록 크기 (바이트)
    pub largest_free_block: usize,
    /// 총 사용 가능한 메모리 (바이트)
    pub total_free: usize,
    /// 총 사용 중인 메모리 (바이트)
    pub total_used: usize,
}

/// 단편화 관리자
pub struct FragmentationManager {
    /// 단편화 통계 히스토리
    history: Vec<FragmentationStats>,
    /// 최대 히스토리 크기
    max_history: usize,
    /// 단편화 임계값 (이 값을 넘으면 경고)
    warning_threshold: f64,
    /// 단편화 위험 임계값 (이 값을 넘으면 압축 제안)
    critical_threshold: f64,
}

impl FragmentationManager {
    /// 새 단편화 관리자 생성
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::new(),
            max_history,
            warning_threshold: 0.5,  // 50% 단편화 시 경고
            critical_threshold: 0.75, // 75% 단편화 시 위험
        }
    }
    
    /// 단편화 통계 계산
    ///
    /// 현재 힙의 단편화 정도를 측정합니다.
    pub fn calculate_fragmentation(&self) -> FragmentationStats {
        let (heap_start, heap_size) = heap_bounds();
        
        // 간단한 단편화 계산:
        // 실제로는 힙 할당자의 내부 상태를 확인해야 하지만,
        // linked_list_allocator는 내부 상태를 직접 노출하지 않으므로
        // 프레임 통계를 사용하여 추정
        
        // 프레임 할당 통계 사용
        let (frame_allocated, frame_deallocated) = crate::memory::frame::get_frame_stats()
            .unwrap_or((0, 0));
        let frames_in_use = frame_allocated.saturating_sub(frame_deallocated);
        
        // 메모리 사용량 추정
        let total_used = (frames_in_use * 4096) as usize;
        let total_free = heap_size.saturating_sub(total_used);
        
        // 단편화 비율 추정 (실제로는 더 정확한 계산 필요)
        // 프레임 캐시 통계 사용
        let (cache_hits, cache_misses, cached_frames) = crate::memory::frame_cache::get_cache_stats();
        let total_cache_requests = cache_hits + cache_misses;
        
        // 캐시 미스율이 높으면 단편화가 심한 것으로 추정
        let fragmentation_ratio = if total_cache_requests > 0 {
            (cache_misses as f64) / (total_cache_requests as f64)
        } else {
            0.0
        };
        
        // 가장 큰 연속 블록 크기 추정 (캐시된 프레임 수 기반)
        let largest_free_block = if cached_frames > 0 {
            cached_frames * 4096
        } else {
            total_free
        };
        
        FragmentationStats {
            fragmentation_ratio,
            free_blocks: cached_frames,
            largest_free_block,
            total_free,
            total_used,
        }
    }
    
    /// 단편화 통계 업데이트
    pub fn update_stats(&mut self) {
        let stats = self.calculate_fragmentation();
        
        // 히스토리에 추가
        self.history.push(stats);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
        
        // 경고 체크
        if stats.fragmentation_ratio > self.critical_threshold {
            crate::log_warn!("Critical fragmentation detected: {:.1}% (largest free block: {} bytes)", 
                            stats.fragmentation_ratio * 100.0, stats.largest_free_block);
        } else if stats.fragmentation_ratio > self.warning_threshold {
            crate::log_warn!("High fragmentation detected: {:.1}% (largest free block: {} bytes)", 
                            stats.fragmentation_ratio * 100.0, stats.largest_free_block);
        }
    }
    
    /// 단편화 통계 가져오기
    pub fn get_stats(&self) -> Option<FragmentationStats> {
        self.history.last().copied()
    }
    
    /// 단편화 히스토리 가져오기
    pub fn get_history(&self) -> &[FragmentationStats] {
        &self.history
    }
    
    /// 단편화가 심한지 확인
    pub fn is_fragmented(&self) -> bool {
        if let Some(stats) = self.history.last() {
            stats.fragmentation_ratio > self.warning_threshold
        } else {
            false
        }
    }
    
    /// 단편화가 위험한지 확인
    pub fn is_critical(&self) -> bool {
        if let Some(stats) = self.history.last() {
            stats.fragmentation_ratio > self.critical_threshold
        } else {
            false
        }
    }
}

/// 전역 단편화 관리자
static FRAGMENTATION_MANAGER: Mutex<FragmentationManager> = Mutex::new(FragmentationManager {
    history: Vec::new(),
    max_history: 100,
    warning_threshold: 0.5,
    critical_threshold: 0.75,
});

/// 단편화 관리자 초기화
pub fn init_fragmentation_monitoring(max_history: usize) {
    let mut manager = FRAGMENTATION_MANAGER.lock();
    *manager = FragmentationManager::new(max_history);
    crate::log_info!("Fragmentation monitoring initialized");
}

/// 단편화 통계 업데이트
pub fn update_fragmentation_stats() {
    let mut manager = FRAGMENTATION_MANAGER.lock();
    manager.update_stats();
}

/// 단편화 통계 가져오기
pub fn get_fragmentation_stats() -> Option<FragmentationStats> {
    let manager = FRAGMENTATION_MANAGER.lock();
    manager.get_stats()
}

/// 단편화가 심한지 확인
pub fn is_fragmented() -> bool {
    let manager = FRAGMENTATION_MANAGER.lock();
    manager.is_fragmented()
}

/// 단편화가 위험한지 확인
pub fn is_fragmentation_critical() -> bool {
    let manager = FRAGMENTATION_MANAGER.lock();
    manager.is_critical()
}

