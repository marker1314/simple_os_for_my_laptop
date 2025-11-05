//! CPU 사용률 계산
//!
//! CPU 사용률을 측정하고 추적합니다.

use spin::Mutex;

/// CPU 사용률 추적기
pub struct CpuUsageTracker {
    /// 활성 시간 (ms) - 유휴가 아닌 시간
    active_time_ms: u64,
    /// 총 시간 (ms)
    total_time_ms: u64,
    /// 마지막 업데이트 시간
    last_update_ms: u64,
    /// 마지막 C-State (0 = 활성, >0 = 유휴)
    last_c_state: u8,
}

impl CpuUsageTracker {
    pub fn new() -> Self {
        let now_ms = crate::drivers::timer::get_milliseconds();
        Self {
            active_time_ms: 0,
            total_time_ms: 0,
            last_update_ms: now_ms,
            last_c_state: 0, // 초기에는 활성 상태로 가정
        }
    }
    
    /// CPU 사용률 업데이트
    /// 
    /// C-State 변화를 기반으로 사용률을 계산합니다.
    /// C0 = 활성, C1+ = 유휴
    pub fn update(&mut self, current_c_state: u8) {
        let now_ms = crate::drivers::timer::get_milliseconds();
        let elapsed_ms = now_ms.saturating_sub(self.last_update_ms);
        
        if elapsed_ms == 0 {
            return; // 시간 변화 없음
        }
        
        self.total_time_ms += elapsed_ms;
        
        // C0는 활성, C1 이상은 유휴
        if self.last_c_state == 0 {
            // 이전에 활성 상태였음
            self.active_time_ms += elapsed_ms;
        }
        
        self.last_c_state = current_c_state;
        self.last_update_ms = now_ms;
    }
    
    /// 현재 CPU 사용률 가져오기 (0-100%)
    pub fn get_usage_percent(&self) -> u8 {
        if self.total_time_ms == 0 {
            return 0;
        }
        
        // 활성 시간 비율 계산
        let usage_percent = (self.active_time_ms * 100) / self.total_time_ms;
        usage_percent.min(100) as u8
    }
    
    /// 리셋 (새로운 측정 주기 시작)
    pub fn reset(&mut self) {
        self.active_time_ms = 0;
        self.total_time_ms = 0;
        let now_ms = crate::drivers::timer::get_milliseconds();
        self.last_update_ms = now_ms;
    }
}

static CPU_USAGE_TRACKER: Mutex<CpuUsageTracker> = Mutex::new(CpuUsageTracker {
    active_time_ms: 0,
    total_time_ms: 0,
    last_update_ms: 0,
    last_c_state: 0,
});

/// CPU 사용률 업데이트
/// 
/// C-State 변화를 감지하여 사용률을 계산합니다.
pub fn update_cpu_usage() {
    let current_c_state = crate::power::idle::get_current_c_state();
    CPU_USAGE_TRACKER.lock().update(current_c_state);
}

/// 현재 CPU 사용률 가져오기 (0-100%)
pub fn get_cpu_usage_percent() -> u8 {
    CPU_USAGE_TRACKER.lock().get_usage_percent()
}

/// CPU 사용률 추적기 리셋
pub fn reset_cpu_usage_tracker() {
    CPU_USAGE_TRACKER.lock().reset();
}

