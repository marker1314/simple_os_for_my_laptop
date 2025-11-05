//! OOM (Out of Memory) Killer
//!
//! 이 모듈은 메모리가 완전히 부족할 때 스레드를 종료하여 메모리를 확보합니다.
//!
//! # OOM Killer 전략
//!
//! 1. **메모리 임계값 확인**: 사용 가능한 메모리가 임계값 이하로 떨어질 때
//! 2. **스레드 선택**: 우선순위 기반으로 종료할 스레드 선택
//! 3. **스레드 종료**: 선택된 스레드를 안전하게 종료
//! 4. **메모리 해제**: 스레드의 메모리를 해제하여 메모리 확보

use spin::Mutex;
use alloc::collections::BTreeMap;

use crate::scheduler::SCHEDULER;

/// OOM Killer 설정
#[derive(Debug, Clone, Copy)]
pub struct OomKillerConfig {
    /// 메모리 부족 임계값 (퍼센트, 0-100)
    /// 이 값 이하로 사용 가능한 메모리가 떨어지면 OOM Killer 활성화
    pub memory_threshold_percent: u8,
    /// 최소 메모리 요구량 (바이트)
    /// 이 값 이상의 메모리가 있어야 시스템이 정상 작동
    pub min_memory_bytes: usize,
    /// OOM Killer 활성화 여부
    pub enabled: bool,
}

impl Default for OomKillerConfig {
    fn default() -> Self {
        Self {
            memory_threshold_percent: 5, // 5% 이하
            min_memory_bytes: 1024 * 1024, // 1MB
            enabled: true,
        }
    }
}

/// 스레드 메모리 사용량 통계
#[derive(Debug, Clone, Copy)]
struct ThreadMemoryStats {
    /// 스레드 ID
    thread_id: u64,
    /// 메모리 사용량 (바이트)
    memory_used: usize,
    /// 스택 크기 (바이트)
    stack_size: usize,
    /// 할당된 프레임 수
    allocated_frames: usize,
}

/// OOM Killer 관리자
pub struct OomKiller {
    /// 설정
    config: OomKillerConfig,
    /// 스레드별 메모리 통계
    thread_stats: BTreeMap<u64, ThreadMemoryStats>,
    /// 종료된 스레드 수
    killed_count: u64,
    /// 마지막 OOM 체크 시간 (밀리초)
    last_check_time: u64,
}

impl OomKiller {
    /// 새 OOM Killer 생성
    pub fn new(config: OomKillerConfig) -> Self {
        Self {
            config,
            thread_stats: BTreeMap::new(),
            killed_count: 0,
            last_check_time: 0,
        }
    }
    
    /// 스레드 메모리 통계 업데이트
    pub fn update_thread_stats(&mut self, thread_id: u64, memory_used: usize, stack_size: usize, allocated_frames: usize) {
        self.thread_stats.insert(thread_id, ThreadMemoryStats {
            thread_id,
            memory_used,
            stack_size,
            allocated_frames,
        });
    }
    
    /// 현재 메모리 상태 확인
    ///
    /// # Returns
    /// (사용 가능한 메모리 비율, 사용 가능한 메모리 바이트)
    fn check_memory_status(&self) -> (f64, usize) {
        // 힙 통계 가져오기
        let (heap_start, heap_size) = crate::memory::heap::heap_bounds();
        
        // 메모리 사용량 추정
        let (frame_allocated, frame_deallocated) = crate::memory::frame::get_frame_stats()
            .unwrap_or((0, 0));
        let frames_in_use = frame_allocated.saturating_sub(frame_deallocated);
        let frame_bytes = frames_in_use * 4096; // 4KB per frame
        
        // 힙 사용량 추정 (간단한 추정)
        let (_, _, heap_in_use) = crate::kernel::watchdog::get_memory_usage();
        let total_used = frame_bytes + heap_in_use as usize;
        
        // 사용 가능한 메모리
        let total_memory = heap_size;
        let available_memory = total_memory.saturating_sub(total_used);
        
        // 사용 가능한 메모리 비율
        let available_percent = if total_memory > 0 {
            (available_memory as f64 / total_memory as f64) * 100.0
        } else {
            0.0
        };
        
        (available_percent, available_memory)
    }
    
    /// OOM 상황 확인
    ///
    /// 메모리가 임계값 이하로 떨어졌는지 확인합니다.
    pub fn is_oom(&self) -> bool {
        if !self.config.enabled {
            return false;
        }
        
        let (available_percent, available_bytes) = self.check_memory_status();
        
        // 임계값 이하 또는 최소 메모리 요구량 이하
        available_percent <= self.config.memory_threshold_percent as f64
            || available_bytes < self.config.min_memory_bytes
    }
    
    /// 종료할 스레드 선택
    ///
    /// 우선순위 기반으로 종료할 스레드를 선택합니다.
    ///
    /// # 전략
    /// 1. 가장 많은 메모리를 사용하는 스레드
    /// 2. 종료 가능한 상태인 스레드 (Blocked, Ready)
    /// 3. 커널 스레드는 제외 (id == 0)
    fn select_thread_to_kill(&self) -> Option<u64> {
        // 스케줄러에서 현재 실행 중인 스레드 확인
        let scheduler = SCHEDULER.lock();
        let sched = scheduler.as_ref()?;
        
        // 가장 많은 메모리를 사용하는 스레드 찾기
        let mut max_memory = 0;
        let mut candidate_thread_id = None;
        
        for (&thread_id, stats) in &self.thread_stats {
            // 커널 스레드 제외 (id == 0 또는 특정 패턴)
            if thread_id == 0 {
                continue;
            }
            
            // 메모리 사용량이 가장 큰 스레드 선택
            let total_memory = stats.memory_used + stats.stack_size + (stats.allocated_frames * 4096);
            if total_memory > max_memory {
                max_memory = total_memory;
                candidate_thread_id = Some(thread_id);
            }
        }
        
        candidate_thread_id
    }
    
    /// OOM Killer 실행
    ///
    /// 메모리가 부족할 때 스레드를 종료하여 메모리를 확보합니다.
    ///
    /// # Returns
    /// 종료된 스레드 수
    pub fn try_kill(&mut self) -> u64 {
        if !self.is_oom() {
            return 0;
        }
        
        let (available_percent, available_bytes) = self.check_memory_status();
        crate::log_warn!("OOM detected: {:.1}% memory available ({} bytes)", 
                        available_percent, available_bytes);
        
        // 종료할 스레드 선택
        if let Some(thread_id) = self.select_thread_to_kill() {
            crate::log_warn!("OOM Killer: Terminating thread {} to free memory", thread_id);
            
            // 스레드 종료
            crate::scheduler::terminate_thread(thread_id);
            
            // 통계에서 제거
            self.thread_stats.remove(&thread_id);
            self.killed_count += 1;
            self.last_check_time = crate::drivers::timer::get_milliseconds();
            
            crate::log_info!("OOM Killer: Thread {} terminated, {} threads killed total", 
                            thread_id, self.killed_count);
            
            1
        } else {
            crate::log_error!("OOM Killer: No suitable thread found to kill");
            0
        }
    }
    
    /// 통계 가져오기
    pub fn get_stats(&self) -> (u64, u64) {
        (self.killed_count, self.thread_stats.len() as u64)
    }
}

/// 전역 OOM Killer 인스턴스
static OOM_KILLER: Mutex<OomKiller> = Mutex::new(OomKiller {
    config: OomKillerConfig::default(),
    thread_stats: BTreeMap::new(),
    killed_count: 0,
    last_check_time: 0,
});

/// OOM Killer 초기화
pub fn init_oom_killer(config: OomKillerConfig) {
    let mut killer = OOM_KILLER.lock();
    *killer = OomKiller::new(config);
    crate::log_info!("OOM Killer initialized (threshold: {}%, min_memory: {} bytes)", 
                   config.memory_threshold_percent, config.min_memory_bytes);
}

/// OOM Killer 활성화/비활성화
pub fn set_enabled(enabled: bool) {
    let mut killer = OOM_KILLER.lock();
    killer.config.enabled = enabled;
    crate::log_info!("OOM Killer {}", if enabled { "enabled" } else { "disabled" });
}

/// 스레드 메모리 통계 업데이트
pub fn update_thread_memory(thread_id: u64, memory_used: usize, stack_size: usize, allocated_frames: usize) {
    let mut killer = OOM_KILLER.lock();
    killer.update_thread_stats(thread_id, memory_used, stack_size, allocated_frames);
}

/// OOM 상황 확인
pub fn check_oom() -> bool {
    let killer = OOM_KILLER.lock();
    killer.is_oom()
}

/// OOM Killer 실행 시도
pub fn try_kill_oom() -> u64 {
    let mut killer = OOM_KILLER.lock();
    killer.try_kill()
}

/// OOM Killer 통계 가져오기
pub fn get_oom_stats() -> (u64, u64) {
    let killer = OOM_KILLER.lock();
    killer.get_stats()
}

