//! CPU 정보 관리
//!
//! 각 CPU 코어의 정보를 관리합니다.

use alloc::string::String;

/// CPU 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuState {
    /// 비활성 (아직 초기화되지 않음)
    Inactive,
    /// 초기화 중
    Initializing,
    /// 활성 (실행 중)
    Active,
    /// 유휴 상태
    Idle,
    /// 오류 상태
    Error,
}

/// CPU 정보 구조체
#[derive(Debug, Clone)]
pub struct CpuInfo {
    /// CPU ID (APIC ID)
    pub id: u8,
    /// Bootstrap Processor 여부
    pub is_bsp: bool,
    /// CPU 상태
    pub state: CpuState,
    /// 로드 (0-100%)
    pub load: u8,
    /// 실행 중인 스레드 수
    pub thread_count: usize,
}

impl CpuInfo {
    /// 새 CPU 정보 생성
    ///
    /// # Arguments
    /// * `id` - CPU ID (APIC ID)
    /// * `is_bsp` - Bootstrap Processor 여부
    pub fn new(id: u8, is_bsp: bool) -> Self {
        Self {
            id,
            is_bsp,
            state: if is_bsp { CpuState::Active } else { CpuState::Inactive },
            load: 0,
            thread_count: 0,
        }
    }
    
    /// CPU 상태 설정
    pub fn set_state(&mut self, state: CpuState) {
        self.state = state;
    }
    
    /// 로드 업데이트
    pub fn update_load(&mut self, load: u8) {
        self.load = load.min(100);
    }
    
    /// 스레드 수 증가
    pub fn increment_thread_count(&mut self) {
        self.thread_count += 1;
    }
    
    /// 스레드 수 감소
    pub fn decrement_thread_count(&mut self) {
        if self.thread_count > 0 {
            self.thread_count -= 1;
        }
    }
    
    /// CPU 활성화 여부
    pub fn is_active(&self) -> bool {
        matches!(self.state, CpuState::Active | CpuState::Idle)
    }
}

/// CPUID 명령어를 통해 CPU 정보 읽기
///
/// # Arguments
/// * `leaf` - CPUID 기능 번호
/// * `subleaf` - 서브 기능 번호
///
/// # Returns
/// (EAX, EBX, ECX, EDX) 레지스터 값
#[inline]
pub fn cpuid(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let mut eax: u32;
    let mut ebx: u32;
    let mut ecx: u32;
    let mut edx: u32;
    
    unsafe {
        core::arch::asm!(
            "cpuid",
            inout("eax") leaf => eax,
            inout("ecx") subleaf => ecx,
            out("ebx") ebx,
            out("edx") edx,
            options(nostack, preserves_flags)
        );
    }
    
    (eax, ebx, ecx, edx)
}

/// CPU 기능 지원 여부 확인
pub fn check_cpu_features() {
    // CPUID 기능 0x1: Processor Info and Feature Bits
    let (_, _, ecx, edx) = cpuid(1, 0);
    
    crate::log_info!("CPU Features:");
    
    // EDX 레지스터의 주요 기능 플래그
    if edx & (1 << 0) != 0 {
        crate::log_info!("  - FPU: Floating Point Unit");
    }
    if edx & (1 << 9) != 0 {
        crate::log_info!("  - APIC: On-chip APIC");
    }
    if edx & (1 << 23) != 0 {
        crate::log_info!("  - MMX: MMX Technology");
    }
    if edx & (1 << 25) != 0 {
        crate::log_info!("  - SSE: Streaming SIMD Extensions");
    }
    if edx & (1 << 26) != 0 {
        crate::log_info!("  - SSE2: Streaming SIMD Extensions 2");
    }
    
    // ECX 레지스터의 주요 기능 플래그
    if ecx & (1 << 0) != 0 {
        crate::log_info!("  - SSE3: Streaming SIMD Extensions 3");
    }
    if ecx & (1 << 21) != 0 {
        crate::log_info!("  - x2APIC: Extended xAPIC");
    }
}

/// CPU 벤더 정보 읽기
pub fn get_cpu_vendor() -> [u8; 12] {
    let (_, ebx, ecx, edx) = cpuid(0, 0);
    
    let mut vendor = [0u8; 12];
    vendor[0..4].copy_from_slice(&ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&ecx.to_le_bytes());
    
    vendor
}

/// CPU 모델 정보 읽기
pub fn get_cpu_model_info() -> (u32, u32, u32, u32) {
    cpuid(1, 0)
}

