//! 성능 메트릭 수집 및 추적
//!
//! 시스템 성능 메트릭을 수집하고 추적합니다.

use spin::Mutex;
use alloc::vec::Vec;

/// 성능 메트릭
#[derive(Clone, Copy)]
pub struct PerformanceMetrics {
    /// 부팅 시간 (ms)
    pub boot_time_ms: u64,
    /// 총 CPU 사이클 (예상)
    pub cpu_cycles: u64,
    /// 컨텍스트 스위칭 횟수
    pub context_switches: u64,
    /// 인터럽트 발생 횟수
    pub interrupts: u64,
    /// 시스템 콜 호출 횟수
    pub syscalls: u64,
    /// 페이지 폴트 횟수
    pub page_faults: u64,
    /// 힙 할당 횟수
    pub heap_allocations: u64,
    /// 힙 해제 횟수
    pub heap_deallocations: u64,
    /// 현재 활성 스레드 수
    pub active_threads: u32,
    /// 메모리 사용량 (바이트)
    pub memory_used_bytes: u64,
    /// 메모리 피크 사용량 (바이트)
    pub memory_peak_bytes: u64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            boot_time_ms: 0,
            cpu_cycles: 0,
            context_switches: 0,
            interrupts: 0,
            syscalls: 0,
            page_faults: 0,
            heap_allocations: 0,
            heap_deallocations: 0,
            active_threads: 0,
            memory_used_bytes: 0,
            memory_peak_bytes: 0,
        }
    }
}

impl PerformanceMetrics {
    /// 메트릭 업데이트
    pub fn update(&mut self) {
        // 부팅 시간 업데이트
        self.boot_time_ms = crate::drivers::timer::get_milliseconds();
        
        // 메모리 사용량 업데이트
        if let Some((allocated, deallocated, in_use)) = crate::kernel::watchdog::get_memory_usage() {
            self.heap_allocations = allocated;
            self.heap_deallocations = deallocated;
            self.memory_used_bytes = in_use;
            if in_use > self.memory_peak_bytes {
                self.memory_peak_bytes = in_use;
            }
        }
        
        // 프레임 통계 업데이트
        if let Some((allocated, deallocated)) = crate::memory::frame::get_frame_stats() {
            // 프레임 할당도 메모리 사용량에 포함
            let frames_used = allocated.saturating_sub(deallocated);
            let frame_bytes = (frames_used as u64) * 4096; // 4KB per frame
            self.memory_used_bytes += frame_bytes;
        }
    }
    
    /// 메트릭 출력
    pub fn print_report(&self) {
        crate::log_info!("=== Performance Metrics ===");
        crate::log_info!("Boot time: {} ms", self.boot_time_ms);
        crate::log_info!("Context switches: {}", self.context_switches);
        crate::log_info!("Interrupts: {}", self.interrupts);
        crate::log_info!("Syscalls: {}", self.syscalls);
        crate::log_info!("Page faults: {}", self.page_faults);
        crate::log_info!("Heap allocations: {} (deallocated: {})", 
                        self.heap_allocations, self.heap_deallocations);
        crate::log_info!("Memory used: {} bytes (peak: {} bytes)", 
                        self.memory_used_bytes, self.memory_peak_bytes);
        crate::log_info!("Active threads: {}", self.active_threads);
        crate::log_info!("========================");
    }
    
    /// CSV 형식으로 내보내기
    pub fn export_csv(&self) {
        crate::serial_println!("metric,value");
        crate::serial_println!("boot_time_ms,{}", self.boot_time_ms);
        crate::serial_println!("context_switches,{}", self.context_switches);
        crate::serial_println!("interrupts,{}", self.interrupts);
        crate::serial_println!("syscalls,{}", self.syscalls);
        crate::serial_println!("page_faults,{}", self.page_faults);
        crate::serial_println!("heap_allocations,{}", self.heap_allocations);
        crate::serial_println!("heap_deallocations,{}", self.heap_deallocations);
        crate::serial_println!("memory_used_bytes,{}", self.memory_used_bytes);
        crate::serial_println!("memory_peak_bytes,{}", self.memory_peak_bytes);
        crate::serial_println!("active_threads,{}", self.active_threads);
    }
}

static METRICS: Mutex<PerformanceMetrics> = Mutex::new(PerformanceMetrics::default());

/// 메트릭 업데이트
pub fn update_metrics() {
    let mut metrics = METRICS.lock();
    metrics.update();
}

/// 컨텍스트 스위칭 기록
pub fn record_context_switch() {
    let mut metrics = METRICS.lock();
    metrics.context_switches += 1;
}

/// 인터럽트 기록
pub fn record_interrupt() {
    let mut metrics = METRICS.lock();
    metrics.interrupts += 1;
}

/// 시스템 콜 기록
pub fn record_syscall() {
    let mut metrics = METRICS.lock();
    metrics.syscalls += 1;
}

/// 페이지 폴트 기록
pub fn record_page_fault() {
    let mut metrics = METRICS.lock();
    metrics.page_faults += 1;
}

/// 활성 스레드 수 업데이트
pub fn update_active_threads(count: u32) {
    let mut metrics = METRICS.lock();
    metrics.active_threads = count;
}

/// 메트릭 가져오기
pub fn get_metrics() -> PerformanceMetrics {
    *METRICS.lock()
}

/// 메트릭 리포트 출력
pub fn print_report() {
    let metrics = METRICS.lock();
    metrics.print_report();
}

/// CSV 형식으로 메트릭 내보내기
pub fn export_csv() {
    let metrics = METRICS.lock();
    metrics.export_csv();
}


