//! 로드 밸런서
//!
//! 멀티코어 시스템에서 작업을 CPU 간에 균등하게 분배합니다.

use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;
use crate::scheduler::thread::Thread;

/// 로드 밸런싱 전략
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalancingStrategy {
    /// Round-Robin: 순차적으로 CPU에 할당
    RoundRobin,
    /// Least Loaded: 가장 부하가 적은 CPU에 할당
    LeastLoaded,
    /// Work Stealing: 유휴 CPU가 바쁜 CPU에서 작업을 가져옴
    WorkStealing,
}

/// CPU별 로드 정보
#[derive(Debug, Clone)]
pub struct CpuLoad {
    /// CPU ID
    pub cpu_id: u8,
    /// 실행 중인 스레드 수
    pub thread_count: usize,
    /// CPU 사용률 (0-100%)
    pub utilization: u8,
}

impl CpuLoad {
    /// 새 CPU 로드 정보 생성
    pub fn new(cpu_id: u8) -> Self {
        Self {
            cpu_id,
            thread_count: 0,
            utilization: 0,
        }
    }
    
    /// 스레드 추가
    pub fn add_thread(&mut self) {
        self.thread_count += 1;
    }
    
    /// 스레드 제거
    pub fn remove_thread(&mut self) {
        if self.thread_count > 0 {
            self.thread_count -= 1;
        }
    }
    
    /// 사용률 업데이트
    pub fn update_utilization(&mut self, utilization: u8) {
        self.utilization = utilization.min(100);
    }
}

/// 로드 밸런서
pub struct LoadBalancer {
    /// 로드 밸런싱 전략
    strategy: BalancingStrategy,
    /// CPU별 로드 정보
    cpu_loads: Vec<CpuLoad>,
    /// 다음 할당할 CPU (Round-Robin용)
    next_cpu: usize,
}

impl LoadBalancer {
    /// 새 로드 밸런서 생성
    ///
    /// # Arguments
    /// * `cpu_count` - CPU 수
    /// * `strategy` - 로드 밸런싱 전략
    pub fn new(cpu_count: usize, strategy: BalancingStrategy) -> Self {
        let mut cpu_loads = Vec::with_capacity(cpu_count);
        for i in 0..cpu_count {
            cpu_loads.push(CpuLoad::new(i as u8));
        }
        
        Self {
            strategy,
            cpu_loads,
            next_cpu: 0,
        }
    }
    
    /// 스레드를 할당할 CPU 선택
    ///
    /// # Returns
    /// 선택된 CPU ID
    pub fn select_cpu_for_thread(&mut self) -> u8 {
        match self.strategy {
            BalancingStrategy::RoundRobin => self.select_cpu_round_robin(),
            BalancingStrategy::LeastLoaded => self.select_cpu_least_loaded(),
            BalancingStrategy::WorkStealing => self.select_cpu_least_loaded(), // 동일하게 처리
        }
    }
    
    /// Round-Robin 방식으로 CPU 선택
    fn select_cpu_round_robin(&mut self) -> u8 {
        let cpu_id = self.next_cpu as u8;
        self.next_cpu = (self.next_cpu + 1) % self.cpu_loads.len();
        cpu_id
    }
    
    /// 가장 부하가 적은 CPU 선택
    fn select_cpu_least_loaded(&self) -> u8 {
        let mut min_load = usize::MAX;
        let mut selected_cpu = 0u8;
        
        for load in &self.cpu_loads {
            if load.thread_count < min_load {
                min_load = load.thread_count;
                selected_cpu = load.cpu_id;
            }
        }
        
        selected_cpu
    }
    
    /// CPU에 스레드 추가
    pub fn add_thread_to_cpu(&mut self, cpu_id: u8) {
        if let Some(load) = self.cpu_loads.get_mut(cpu_id as usize) {
            load.add_thread();
        }
    }
    
    /// CPU에서 스레드 제거
    pub fn remove_thread_from_cpu(&mut self, cpu_id: u8) {
        if let Some(load) = self.cpu_loads.get_mut(cpu_id as usize) {
            load.remove_thread();
        }
    }
    
    /// CPU 사용률 업데이트
    pub fn update_cpu_utilization(&mut self, cpu_id: u8, utilization: u8) {
        if let Some(load) = self.cpu_loads.get_mut(cpu_id as usize) {
            load.update_utilization(utilization);
        }
    }
    
    /// 로드 밸런싱 필요 여부 확인
    ///
    /// # Returns
    /// 로드 불균형이 있으면 `true`
    pub fn needs_rebalancing(&self) -> bool {
        if self.cpu_loads.len() < 2 {
            return false;
        }
        
        let mut min_load = usize::MAX;
        let mut max_load = 0usize;
        
        for load in &self.cpu_loads {
            if load.thread_count < min_load {
                min_load = load.thread_count;
            }
            if load.thread_count > max_load {
                max_load = load.thread_count;
            }
        }
        
        // 로드 차이가 2 이상이면 재조정 필요
        max_load - min_load >= 2
    }
    
    /// CPU 로드 정보 가져오기
    pub fn get_cpu_loads(&self) -> &[CpuLoad] {
        &self.cpu_loads
    }
    
    /// CPU 수 반환
    pub fn cpu_count(&self) -> usize {
        self.cpu_loads.len()
    }
}

/// 전역 로드 밸런서
static LOAD_BALANCER: Mutex<Option<LoadBalancer>> = Mutex::new(None);

/// 로드 밸런서 초기화
///
/// # Arguments
/// * `cpu_count` - CPU 수
/// * `strategy` - 로드 밸런싱 전략
pub fn init(cpu_count: usize, strategy: BalancingStrategy) {
    let mut balancer = LOAD_BALANCER.lock();
    *balancer = Some(LoadBalancer::new(cpu_count, strategy));
    crate::log_info!("Load balancer initialized for {} CPUs with {:?} strategy", 
                     cpu_count, strategy);
}

/// 스레드를 할당할 CPU 선택
pub fn select_cpu_for_thread() -> u8 {
    let mut balancer = LOAD_BALANCER.lock();
    if let Some(ref mut lb) = *balancer {
        lb.select_cpu_for_thread()
    } else {
        0 // 기본값: CPU 0
    }
}

/// CPU에 스레드 추가 통지
pub fn notify_thread_added(cpu_id: u8) {
    let mut balancer = LOAD_BALANCER.lock();
    if let Some(ref mut lb) = *balancer {
        lb.add_thread_to_cpu(cpu_id);
    }
}

/// CPU에서 스레드 제거 통지
pub fn notify_thread_removed(cpu_id: u8) {
    let mut balancer = LOAD_BALANCER.lock();
    if let Some(ref mut lb) = *balancer {
        lb.remove_thread_from_cpu(cpu_id);
    }
}

/// CPU 사용률 업데이트
pub fn update_cpu_utilization(cpu_id: u8, utilization: u8) {
    let mut balancer = LOAD_BALANCER.lock();
    if let Some(ref mut lb) = *balancer {
        lb.update_cpu_utilization(cpu_id, utilization);
    }
}

/// 로드 밸런싱 필요 여부 확인
pub fn needs_rebalancing() -> bool {
    let balancer = LOAD_BALANCER.lock();
    if let Some(ref lb) = *balancer {
        lb.needs_rebalancing()
    } else {
        false
    }
}

/// 모든 CPU의 로드 정보 가져오기
pub fn get_cpu_loads() -> Vec<CpuLoad> {
    let balancer = LOAD_BALANCER.lock();
    if let Some(ref lb) = *balancer {
        lb.get_cpu_loads().to_vec()
    } else {
        Vec::new()
    }
}

