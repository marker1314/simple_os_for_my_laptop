//! 메모리 누수 추적 도구
//!
//! 이 모듈은 메모리 할당/해제를 추적하여 누수를 감지합니다.
//!
//! # 기능
//! - 할당 추적 (주소, 크기, 호출 스택)
//! - 해제 검증
//! - 누수 리포트 생성
//! - 주기적 검사

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use core::alloc::Layout;

/// 메모리 할당 정보
#[derive(Debug, Clone)]
struct AllocationInfo {
    /// 할당 주소
    address: usize,
    /// 할당 크기
    size: usize,
    /// 할당 시간 (밀리초)
    timestamp: u64,
    /// 할당 타입 (힙, 프레임 등)
    allocation_type: AllocationType,
}

/// 할당 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationType {
    Heap,
    Frame,
    Slab,
    Other,
}

/// 메모리 누수 감지기
pub struct LeakDetector {
    /// 활성 할당 추적
    active_allocations: BTreeMap<usize, AllocationInfo>,
    /// 누수 카운트
    leak_count: u64,
    /// 총 할당 수
    total_allocations: u64,
    /// 총 해제 수
    total_deallocations: u64,
    /// 활성화 여부
    enabled: bool,
}

impl LeakDetector {
    /// 새 누수 감지기 생성
    fn new() -> Self {
        Self {
            active_allocations: BTreeMap::new(),
            leak_count: 0,
            total_allocations: 0,
            total_deallocations: 0,
            enabled: true,
        }
    }
    
    /// 할당 기록
    pub fn record_allocation(&mut self, address: usize, size: usize, alloc_type: AllocationType) {
        if !self.enabled {
            return;
        }
        
        let timestamp = crate::drivers::timer::get_milliseconds();
        
        let info = AllocationInfo {
            address,
            size,
            timestamp,
            allocation_type: alloc_type,
        };
        
        // 이미 할당된 주소인지 확인 (이중 할당 감지)
        if self.active_allocations.contains_key(&address) {
            crate::log_warn!("Double allocation detected at address 0x{:X}", address);
            self.leak_count += 1;
        }
        
        self.active_allocations.insert(address, info);
        self.total_allocations += 1;
    }
    
    /// 해제 기록
    pub fn record_deallocation(&mut self, address: usize) -> bool {
        if !self.enabled {
            return true;
        }
        
        if self.active_allocations.remove(&address).is_some() {
            self.total_deallocations += 1;
            true
        } else {
            // 해제되지 않은 메모리 해제 시도 (이중 해제 또는 잘못된 주소)
            crate::log_warn!("Double deallocation or invalid address: 0x{:X}", address);
            false
        }
    }
    
    /// 누수 검사
    pub fn check_leaks(&self) -> Vec<&AllocationInfo> {
        let mut leaks = Vec::new();
        
        for (_, info) in &self.active_allocations {
            // 일정 시간 이상 할당된 메모리는 누수로 간주
            let age_ms = crate::drivers::timer::get_milliseconds().saturating_sub(info.timestamp);
            if age_ms > 60_000 { // 60초 이상
                leaks.push(info);
            }
        }
        
        leaks
    }
    
    /// 누수 리포트 생성
    pub fn generate_report(&self) -> LeakReport {
        let leaks = self.check_leaks();
        let total_leaked_size: usize = leaks.iter().map(|l| l.size).sum();
        
        LeakReport {
            total_allocations: self.total_allocations,
            total_deallocations: self.total_deallocations,
            active_allocations: self.active_allocations.len(),
            leak_count: leaks.len(),
            total_leaked_size,
            leaks: leaks.into_iter().cloned().collect(),
        }
    }
    
    /// 활성화/비활성화
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// 활성화 여부 확인
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// 통계 초기화
    pub fn reset_stats(&mut self) {
        self.active_allocations.clear();
        self.leak_count = 0;
        self.total_allocations = 0;
        self.total_deallocations = 0;
    }
}

/// 누수 리포트
#[derive(Debug)]
pub struct LeakReport {
    /// 총 할당 수
    pub total_allocations: u64,
    /// 총 해제 수
    pub total_deallocations: u64,
    /// 활성 할당 수
    pub active_allocations: usize,
    /// 누수 수
    pub leak_count: usize,
    /// 누수된 총 크기 (바이트)
    pub total_leaked_size: usize,
    /// 누수 목록
    pub leaks: Vec<AllocationInfo>,
}

impl LeakReport {
    /// 리포트 출력
    pub fn print(&self) {
        crate::log_info!("=== Memory Leak Report ===");
        crate::log_info!("Total allocations: {}", self.total_allocations);
        crate::log_info!("Total deallocations: {}", self.total_deallocations);
        crate::log_info!("Active allocations: {}", self.active_allocations);
        crate::log_info!("Leak count: {}", self.leak_count);
        crate::log_info!("Total leaked size: {} bytes ({:.2} KB)", 
                        self.total_leaked_size,
                        self.total_leaked_size as f32 / 1024.0);
        
        if !self.leaks.is_empty() {
            crate::log_info!("=== Leaked Allocations ===");
            for (i, leak) in self.leaks.iter().take(10).enumerate() {
                let age_s = (crate::drivers::timer::get_milliseconds() - leak.timestamp) / 1000;
                crate::log_info!("Leak {}: 0x{:X}, size={}, age={}s, type={:?}",
                                i + 1,
                                leak.address,
                                leak.size,
                                age_s,
                                leak.allocation_type);
            }
            if self.leaks.len() > 10 {
                crate::log_info!("... and {} more leaks", self.leaks.len() - 10);
            }
        }
    }
}

/// 전역 누수 감지기
static LEAK_DETECTOR: Mutex<LeakDetector> = Mutex::new(LeakDetector {
    active_allocations: BTreeMap::new(),
    leak_count: 0,
    total_allocations: 0,
    total_deallocations: 0,
    enabled: false, // 기본적으로 비활성화 (디버그 모드에서만 활성화)
});

/// 누수 감지기 초기화
pub fn init() {
    let mut detector = LEAK_DETECTOR.lock();
    detector.enabled = cfg!(debug_assertions); // 디버그 모드에서만 활성화
    crate::log_info!("Memory leak detector initialized (enabled: {})", detector.enabled);
}

/// 할당 기록 (공개 인터페이스)
pub fn record_allocation(address: usize, size: usize, alloc_type: AllocationType) {
    let mut detector = LEAK_DETECTOR.lock();
    detector.record_allocation(address, size, alloc_type);
}

/// 해제 기록 (공개 인터페이스)
pub fn record_deallocation(address: usize) -> bool {
    let mut detector = LEAK_DETECTOR.lock();
    detector.record_deallocation(address)
}

/// 누수 검사 (공개 인터페이스)
pub fn check_leaks() -> Vec<AllocationInfo> {
    let detector = LEAK_DETECTOR.lock();
    detector.check_leaks().into_iter().cloned().collect()
}

/// 누수 리포트 생성 (공개 인터페이스)
pub fn generate_report() -> LeakReport {
    let detector = LEAK_DETECTOR.lock();
    detector.generate_report()
}

/// 누수 감지기 활성화/비활성화
pub fn set_enabled(enabled: bool) {
    let mut detector = LEAK_DETECTOR.lock();
    detector.set_enabled(enabled);
}

/// 주기적 누수 검사 (타이머에서 호출)
pub fn periodic_check() {
    let detector = LEAK_DETECTOR.lock();
    if !detector.is_enabled() {
        return;
    }
    
    let leaks = detector.check_leaks();
    if !leaks.is_empty() {
        let report = detector.generate_report();
        if report.leak_count > 10 {
            // 누수가 많이 발생하면 리포트 출력
            drop(detector); // 락 해제
            report.print();
        }
    }
}

