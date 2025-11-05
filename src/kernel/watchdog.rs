//! Kernel Watchdog
//!
//! Soft lockup detector 및 메모리 leak 추적

use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use spin::Mutex;

/// Watchdog 타임아웃 (초)
const WATCHDOG_TIMEOUT_SEC: u64 = 60; // 60초

/// 마지막 heartbeat 시간 (타이머 틱)
static LAST_HEARTBEAT: AtomicU64 = AtomicU64::new(0);

/// Watchdog 활성화 여부
static WATCHDOG_ENABLED: AtomicBool = AtomicBool::new(true);

/// Watchdog 초기화
pub fn init() {
    LAST_HEARTBEAT.store(crate::drivers::timer::get_milliseconds(), Ordering::Release);
    WATCHDOG_ENABLED.store(true, Ordering::Release);
    crate::log_info!("Watchdog initialized (timeout: {}s)", WATCHDOG_TIMEOUT_SEC);
}

/// Heartbeat 업데이트 (scheduler tick에서 호출)
pub fn heartbeat() {
    let now = crate::drivers::timer::get_milliseconds();
    LAST_HEARTBEAT.store(now, Ordering::Release);
}

/// Watchdog 체크 (타이머 인터럽트에서 호출)
pub fn check() {
    if !WATCHDOG_ENABLED.load(Ordering::Acquire) {
        return;
    }
    
    let now = crate::drivers::timer::get_milliseconds();
    let last = LAST_HEARTBEAT.load(Ordering::Acquire);
    
    if now.saturating_sub(last) > WATCHDOG_TIMEOUT_SEC * 1000 {
        // Soft lockup 감지
        crate::log_error!("SOFT LOCKUP DETECTED: scheduler not responding for {}s",
                         (now - last) / 1000);
        
        // Panic 또는 복구 시도
        #[cfg(not(feature = "watchdog_panic"))]
        {
            // 복구 시도: heartbeat 강제 업데이트
            heartbeat();
        }
        
        #[cfg(feature = "watchdog_panic")]
        {
            panic!("Kernel soft lockup detected");
        }
    }
}

/// Watchdog 활성화/비활성화
pub fn set_enabled(enabled: bool) {
    WATCHDOG_ENABLED.store(enabled, Ordering::Release);
}

/// Enable watchdog with a custom timeout (seconds)
pub fn enable_with_timeout(seconds: u64) {
    LAST_HEARTBEAT.store(crate::drivers::timer::get_milliseconds(), Ordering::Release);
    WATCHDOG_ENABLED.store(true, Ordering::Release);
    // Note: static timeout constant not changed; this is a placeholder for future dynamic timeout
    crate::log_info!("Watchdog enabled (requested timeout: {}s)", seconds);
}

/// 메모리 할당 추적 (메모리 leak 감지)
static MEMORY_ALLOCATED: AtomicU64 = AtomicU64::new(0);
static MEMORY_FREED: AtomicU64 = AtomicU64::new(0);
static MEMORY_PEAK: AtomicU64 = AtomicU64::new(0);

/// 메모리 할당 기록
pub fn record_allocation(bytes: usize) {
    let allocated = MEMORY_ALLOCATED.fetch_add(bytes as u64, Ordering::Relaxed);
    let current = allocated + bytes as u64;
    let freed = MEMORY_FREED.load(Ordering::Relaxed);
    let in_use = current - freed;
    
    let mut peak = MEMORY_PEAK.load(Ordering::Relaxed);
    while in_use > peak {
        match MEMORY_PEAK.compare_exchange(peak, in_use, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(new_peak) => peak = new_peak,
        }
    }
}

/// 메모리 해제 기록
pub fn record_deallocation(bytes: usize) {
    MEMORY_FREED.fetch_add(bytes as u64, Ordering::Relaxed);
}

/// 메모리 사용량 가져오기
pub fn get_memory_usage() -> (u64, u64, u64) {
    let allocated = MEMORY_ALLOCATED.load(Ordering::Relaxed);
    let freed = MEMORY_FREED.load(Ordering::Relaxed);
    let in_use = allocated.saturating_sub(freed);
    let peak = MEMORY_PEAK.load(Ordering::Relaxed);
    
    (allocated, freed, in_use)
}

/// 메모리 leak 검사 (주기적으로 호출)
pub fn check_memory_leak() -> Option<f64> {
    let (allocated, freed, in_use) = get_memory_usage();
    
    if allocated == 0 {
        return None;
    }
    
    // 메모리 사용률 계산
    let usage_rate = (in_use as f64) / (allocated as f64);
    
    // 90% 이상 사용 중이면 경고
    if usage_rate > 0.90 {
        crate::log_warn!("High memory usage: {}% ({}/{})", 
                        usage_rate * 100.0, in_use, allocated);
    }
    
    Some(usage_rate)
}

